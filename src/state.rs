use anyhow::{Context, Result};
use std::sync::Arc;
use tokio::task::JoinSet;

use crate::{
    addon::Addon,
    api::CurseClient,
    store::{
        GameVersion, MoxenConfig,
        path::{MoxenPath, unzip_archive},
        registry::{self, MoxenRegistry},
    },
};

pub struct MoxenApp {
    config: MoxenConfig,
    registry: MoxenRegistry,
}

impl MoxenApp {
    pub fn initialise() -> Result<()> {
        registry::initialise().context("moxen initialise - registry")?;
        MoxenConfig::initialise().context("moxen initialise - config")?;
        Ok(())
    }

    pub fn new() -> Result<Self> {
        let config = MoxenConfig::load().context("moxen state creation - config")?;
        let registry =
            registry::load(&config.version).context("moxen state creation - registry")?;

        Ok(Self { config, registry })
    }

    pub fn switch_game_version(&mut self, version: GameVersion) -> Result<()> {
        self.config.version = version;
        self.config.save().context("saving config file")?;
        println!("Switched game version to '{version}'");

        Ok(())
    }

    pub fn list_contents(&self) {
        if self.registry.is_empty() {
            println!("No Addons tracked.");
        } else {
            println!("Tracked addons:");
            for (key, addon) in self.registry.iter() {
                println!("* {} ({}) - {}", addon.name, key, addon.summary);
            }
        }
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

            if !self.registry.contains_key(&addon.id) {
                println!("Tracking addon \"{}\" ({})", addon.name, addon.id);
                self.add_registry_item(addon);
            } else {
                println!("Already tracking \"{}\" ({})", addon.name, addon.id);
            }
        }

        self.save().context("tracking addons")?;
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

        let mut js: JoinSet<Result<()>> = JoinSet::new();
        for addon in addons {
            self.add_registry_item(addon.clone());

            let client = Arc::clone(&client);
            js.spawn(async move {
                println!("Updating {}", addon.name);
                let content = client
                    .download_addon(&addon)
                    .await
                    .with_context(|| format!("downloading latest version of {}", addon.name))?;

                let cache_path = MoxenPath::new()
                    .context("loading root moxen path")?
                    .dir("registry")
                    .context("loading registry path")?
                    .dir("cache")
                    .context("loading cache path")?
                    .dir(&addon.slug)
                    .with_context(|| format!("loading {} path", addon.slug))?
                    .build();

                let filename = cache_path.join(addon.main_file.file_name);

                tokio::fs::write(filename, &content)
                    .await
                    .with_context(|| format!("writing out {} to cache", addon.name))?;

                Ok(())
            });
        }

        while let Some(result) = js.join_next().await {
            result
                .context("joining update task")?
                .context("update task result")?;
        }

        self.save().context("saving registry after update")?;
        println!("Update complete!");

        Ok(())
    }

    pub async fn install_addons(&mut self) -> Result<()> {
        self.update_addons().await.context("updating addons")?;
        let install_dir = self.config.install_dir.addon_dir(&self.config.version);

        // Debug install dir for testing
        if cfg!(target_os = "linux") {
            std::fs::create_dir_all(&install_dir)
                .context("DEBUG - simulating install directory")?;
        }

        println!("Installing addons...");
        let mut js: JoinSet<Result<()>> = JoinSet::new();
        for addon in self.registry.values() {
            let addon = addon.clone();
            let install_dir = install_dir.clone();

            js.spawn_blocking(move || {
                println!("Installing {}...", addon.name);
                let file = MoxenPath::new()
                    .context("loading root moxen path")?
                    .dir("registry")
                    .context("loading registry path")?
                    .dir("cache")
                    .context("loading cache path")?
                    .dir(&addon.slug)
                    .with_context(|| format!("loading {} path", addon.slug))?
                    .file(&addon.main_file.file_name)
                    .build();

                unzip_archive(&file, &install_dir)
                    .with_context(|| format!("unzipping {}", file.display()))?;

                Ok(())
            });
        }

        while let Some(result) = js.join_next().await {
            result
                .context("joining install task")?
                .context("install task result")?;
        }

        println!("Install complete!");

        Ok(())
    }

    pub async fn uninstall_addons(&mut self, mod_ids: Vec<i32>) -> Result<()> {
        let mut js: JoinSet<Result<()>> = JoinSet::new();
        let src_dir = self.config.install_dir.addon_dir(&self.config.version);

        for id in mod_ids {
            let Some(addon) = self.remove_registry_item(id) else {
                eprintln!("No such addon: {}", id);
                continue;
            };

            let src_dir = src_dir.clone();
            println!("Removing addon {}...", addon.name);
            js.spawn(async move {
                let cache_dir = MoxenPath::new()
                    .context("loading root moxen path")?
                    .dir("registry")
                    .context("loading registry path")?
                    .dir("cache")
                    .context("loading cache path")?
                    .build();

                let addon_dir = cache_dir.join(addon.slug.clone());
                if addon_dir.exists() {
                    tokio::fs::remove_dir_all(&addon_dir)
                        .await
                        .with_context(|| format!("removing cached dir {}", addon_dir.display()))?;
                }

                for module in addon.main_file.modules.iter() {
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

        self.save().context("removal - saving registry")?;
        println!("Successfully removed addons!");

        Ok(())
    }

    pub fn clear_cache(&self) -> Result<()> {
        let cache = MoxenPath::new()
            .context("loading root moxen path")?
            .dir("registry")
            .context("loading registry path")?
            .dir("cache")
            .context("loading cache path")?
            .build();

        std::fs::remove_dir_all(cache).context("removing cache dir")?;
        println!("Cleared Moxen cache.");

        Ok(())
    }

    async fn check_updates(&self, client: Arc<CurseClient>) -> Result<Vec<Addon>> {
        let mut to_update = Vec::new();
        let mut js: JoinSet<Result<Addon>> = JoinSet::new();
        let addon_ids: Vec<i32> = self.registry.keys().copied().collect();
        for aid in addon_ids {
            let client = Arc::clone(&client);
            js.spawn(async move { client.get_addon(aid).await });
        }

        while let Some(addon) = js.join_next().await {
            let addon = addon
                .context("joining check update task")?
                .context("check update result")?;

            let reg_addon = self
                .registry
                .get(&addon.id)
                .expect("this has to be present at this point");

            let cache_path = MoxenPath::new()
                .context("loading root moxen path")?
                .dir("registry")
                .context("loading registry path")?
                .dir("cache")
                .context("loading cache path")?
                .dir(&addon.slug)
                .with_context(|| format!("loading {} path", addon.slug))?
                .build();

            let filename = cache_path.join(&addon.main_file.file_name);
            if reg_addon.main_file.id != addon.main_file.id || !filename.exists() {
                to_update.push(addon);
            }
        }

        Ok(to_update)
    }

    fn add_registry_item(&mut self, addon: Addon) {
        self.registry.insert(addon.id, addon);
    }

    fn remove_registry_item(&mut self, id: i32) -> Option<Addon> {
        self.registry.remove(&id)
    }

    fn save(&self) -> Result<()> {
        registry::save(&self.registry, &self.config.version).context("saving state - registry")
    }
}
