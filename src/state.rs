use anyhow::{Context, Result};
use std::sync::Arc;
use tokio::task::JoinSet;

use crate::{
    addon::Addon,
    api::CurseClient,
    store::{
        GameVersion, MoxenConfig, MoxenPath,
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
        println!("Switched game version to {version}");

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
            let addon = addon??;
            println!("Tracking addon \"{}\" ({})", addon.name, addon.id);
            self.add_registry_item(addon);
        }

        self.save().context("tracking addons")?;
        Ok(())
    }

    pub async fn update_addons(&mut self) -> Result<()> {
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

                let cache_path = MoxenPath::dir(format!("registry/cache/{}", addon.slug))
                    .context("constructing cache path")?;
                let filename = cache_path.join(addon.main_file.file_name);

                std::fs::write(filename, &content)
                    .with_context(|| format!("writing out {} to cache", addon.name))?;

                Ok(())
            });
        }

        js.join_all().await;
        self.save().context("saving registry after update")?;
        println!("Update complete!");

        Ok(())
    }

    pub fn install_addons(&self) -> Result<()> {
        Ok(())
    }

    pub fn clear_cache(&self) -> Result<()> {
        let cache = MoxenPath::dir("registry/cache").context("loading cache path")?;
        std::fs::remove_dir_all(cache).context("removing cache dir")?;
        println!("Cleared Moxen cache.");

        Ok(())
    }

    async fn check_updates(&self, client: Arc<CurseClient>) -> Result<Vec<Addon>> {
        let mut to_update = Vec::new();
        let mut js: JoinSet<Result<Addon>> = JoinSet::new();
        for aid in self.registry.keys() {
            let client = Arc::clone(&client);
            let aid = aid.clone();
            js.spawn(async move {
                let addon = client.get_addon(aid).await?;
                Ok(addon)
            });
        }

        while let Some(addon) = js.join_next().await {
            let addon = addon??;
            let reg_addon = self
                .registry
                .get(&addon.id)
                .expect("this has to be present at this point");

            let cache_path = MoxenPath::dir(format!("registry/cache/{}", addon.slug))
                .context("constructing cache path")?;

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

    fn save(&self) -> Result<()> {
        registry::save(&self.registry, &self.config.version).context("saving state - registry")?;

        Ok(())
    }
}
