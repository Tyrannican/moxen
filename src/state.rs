use anyhow::{Context, Result};
use std::sync::Arc;
use tokio::task::JoinSet;

use crate::{
    addon::Addon,
    api::CurseClient,
    db::MoxenDbStore,
    store::{GameVersion, MoxenConfig, path::unzip_archive},
};

pub struct MoxenApp {
    config: MoxenConfig,
    db: Arc<MoxenDbStore>,
}

impl MoxenApp {
    pub fn initialise() -> Result<()> {
        println!("Initialising Moxen...\n");
        MoxenConfig::initialise().context("moxen initialise - config")?;

        println!("Successfully initialised Moxen!");
        Ok(())
    }

    pub async fn new() -> Result<Self> {
        let config = MoxenConfig::load().context("moxen state creation - config")?;
        let db = MoxenDbStore::load(config.version)
            .await
            .context("loading database")?;

        Ok(Self {
            config,
            db: Arc::new(db),
        })
    }

    pub fn switch_game_version(&mut self, version: GameVersion) -> Result<()> {
        self.config.version = version;
        self.config.save().context("saving config file")?;
        println!("Switched game version to '{version}'");

        Ok(())
    }

    pub async fn list_contents(&self) -> Result<()> {
        let addons = self
            .db
            .fetch_all()
            .await
            .context("retrieving all addon metadata")?;

        if addons.is_empty() {
            println!("No Addons tracked.");
        } else {
            println!("Tracked addons:");

            for addon in addons {
                println!("* {} ({}) - {}", addon.name, addon.id, addon.summary);
            }
        }

        Ok(())
    }

    pub async fn track_addons(&mut self, mod_ids: Vec<i32>) -> Result<()> {
        let client = Arc::new(CurseClient::new(&self.config.api_key));
        let mut js = JoinSet::new();

        for mid in mod_ids.into_iter() {
            let client = Arc::clone(&client);
            js.spawn(async move { client.get_addon(mid).await });
        }

        while let Some(addon) = js.join_next().await {
            let addon = addon
                .context("joining track task")?
                .context("track task result")?;

            if self
                .db
                .fetch_addon(addon.id)
                .await
                .context("checking tracked status")?
                .is_none()
            {
                println!("Tracking addon \"{}\" ({})", addon.name, addon.id);
                self.db
                    .insert_addon(&addon)
                    .await
                    .context("inserting addon")?;
            } else {
                println!("Already tracking \"{}\" ({})", addon.name, addon.id);
            }
        }

        Ok(())
    }

    pub async fn update_addons(&mut self) -> Result<()> {
        println!("Checking for updates...");
        let client = Arc::new(CurseClient::new(&self.config.api_key));
        let addons = self
            .check_updates(Arc::clone(&client))
            .await
            .context("checking for updates")?;

        if addons.is_empty() {
            println!("No updates required.");
            return Ok(());
        }

        println!("Updating {} addons...", addons.len());

        let mut js: JoinSet<Result<i32>> = JoinSet::new();
        for addon in addons {
            let client = Arc::clone(&client);
            let db = Arc::clone(&self.db);
            js.spawn(async move {
                println!("Updating {}", addon.name);
                let content = client
                    .download_addon(&addon)
                    .await
                    .with_context(|| format!("downloading latest version of {}", addon.name))?;

                db.insert_cache(&addon, content)
                    .await
                    .with_context(|| format!("inserting file for {} into cache", addon.slug))?;

                Ok(addon.id)
            });
        }

        while let Some(result) = js.join_next().await {
            let addon_id = result
                .context("joining update task")?
                .context("update task result")?;

            self.db
                .set_updated(addon_id)
                .await
                .context("setting update tracker")?;
        }

        println!("Update complete!");

        Ok(())
    }

    pub async fn install_addons(&mut self) -> Result<()> {
        self.update_addons().await.context("updating addons")?;

        let updated_addons = self
            .db
            .fetch_updated()
            .await
            .context("fetching recently updated addons")?;

        if updated_addons.is_empty() {
            return Ok(());
        }

        let install_dir = self.config.install_dir.addon_dir(&self.config.version);

        // Debug install dir for testing
        if cfg!(target_os = "linux") {
            std::fs::create_dir_all(&install_dir)
                .context("DEBUG - simulating install directory")?;
        }

        println!("Installing addons...");
        let mut js: JoinSet<Result<()>> = JoinSet::new();

        for addon_id in updated_addons {
            let (_, slug, _, _, _, content) = self
                .db
                .fetch_cache(addon_id)
                .await
                .context("fetching from cache")?
                .expect("addon just updated so has to be present");

            let install_dir = install_dir.clone();

            js.spawn_blocking(move || {
                println!("Installing {}...", slug);

                unzip_archive(&content, &install_dir)
                    .with_context(|| format!("unzipping {}", slug))?;

                Ok(())
            });
        }

        while let Some(result) = js.join_next().await {
            result
                .context("joining install task")?
                .context("install task result")?;
        }

        println!("Install complete!");
        self.db
            .clear_update_table()
            .await
            .context("clearing update table")?;

        Ok(())
    }

    pub async fn uninstall_addons(&mut self, mod_ids: Vec<i32>) -> Result<()> {
        let mut js: JoinSet<Result<()>> = JoinSet::new();
        let src_dir = self.config.install_dir.addon_dir(&self.config.version);

        for id in mod_ids {
            let Some(addon) = self
                .db
                .fetch_addon(id)
                .await
                .with_context(|| format!("fetching addon {id}"))?
            else {
                eprintln!("No such addon: {}", id);
                continue;
            };

            let src_dir = src_dir.clone();

            let modules = self
                .db
                .fetch_modules(id)
                .await
                .context("fetching modules")?;

            println!("Removing addon {}...", addon.name);
            self.db
                .remove_addon_modules(id)
                .await
                .context("clearing modules")?;
            self.db.remove_addon(id).await.context("removing addon")?;
            self.db
                .delete_from_cache(id)
                .await
                .context("removing addon from cache")?;

            js.spawn(async move {
                for module in modules.iter() {
                    let mod_path = src_dir.join(module);
                    tokio::fs::remove_dir_all(&mod_path)
                        .await
                        .with_context(|| format!("removing module {}", mod_path.display()))?;
                }

                Ok(())
            });
        }

        while let Some(result) = js.join_next().await {
            result
                .context("joining removal task")?
                .context("removal task result")?;
        }

        println!("Successfully removed addons!");

        Ok(())
    }

    pub async fn clear_cache(&self) -> Result<()> {
        self.db.clear_cache().await.context("clearing cache")?;

        println!("Cleared Moxen cache.");

        Ok(())
    }

    async fn check_updates(&self, client: Arc<CurseClient>) -> Result<Vec<Addon>> {
        let mut to_update = Vec::new();
        let mut js: JoinSet<Result<Addon>> = JoinSet::new();
        let addons = self.db.fetch_all().await.context("retrieving all addons")?;
        let addon_ids: Vec<i32> = addons.iter().map(|a| a.id).collect();
        for aid in addon_ids {
            let client = Arc::clone(&client);
            js.spawn(async move { client.get_addon(aid).await });
        }

        while let Some(addon) = js.join_next().await {
            let addon = addon
                .context("joining check update task")?
                .context("check update result")?;

            let reg_addon = addons
                .iter()
                .find(|a| a.id == addon.id)
                .expect("addon is present as it has to be retrieved upstream");

            let cached_entry = self
                .db
                .fetch_cache(reg_addon.id)
                .await
                .context("fetching entry from cache")?;

            if reg_addon.file_id != addon.main_file.id || cached_entry.is_none() {
                to_update.push(addon);
            }
        }

        Ok(to_update)
    }
}
