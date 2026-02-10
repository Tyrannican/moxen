use anyhow::{Context, Result};
use std::sync::Arc;
use tokio::task::JoinSet;

use crate::{
    addon::Addon,
    api::CurseClient,
    store::{
        MoxenConfig,
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
        Ok(Self {
            config: MoxenConfig::load().context("moxen state creation - config")?,
            registry: registry::load().context("moxen state creation - registry")?,
        })
    }

    pub fn list_contents(&self) {
        println!("Tracked addons:");
        for (key, addon) in self.registry.iter() {
            println!("* {} ({}) - {}", addon.name, key, addon.summary);
            println!("{addon:?}\n");
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

    pub async fn update_addons(&self) -> Result<()> {
        let client = Arc::new(CurseClient::new(&self.config.api_key));
        let needs_update = self
            .check_updates(Arc::clone(&client))
            .await
            .context("checking for update")?;

        if needs_update.is_empty() {
            println!("All addons up to date.");
        }

        Ok(())
    }

    async fn check_updates(&self, client: Arc<CurseClient>) -> Result<Vec<i32>> {
        let mut to_update = Vec::new();
        let mut js: JoinSet<Result<(i32, i32)>> = JoinSet::new();
        for aid in self.registry.keys() {
            let client = Arc::clone(&client);
            let aid = aid.clone();
            js.spawn(async move {
                let addon = client.get_addon(aid).await?;
                Ok((addon.id, addon.main_file.id))
            });
        }

        while let Some(addon_ids) = js.join_next().await {
            let (id, main_file_id) = addon_ids??;
            let addon = self
                .registry
                .get(&id)
                .expect("this has to be present at this point");

            if addon.main_file.id != main_file_id {
                println!("Addon {} requires an update", addon.name);
                to_update.push(addon.id);
            }
        }

        Ok(to_update)
    }

    fn add_registry_item(&mut self, addon: Addon) {
        self.registry.insert(addon.id, addon);
    }

    fn save(&self) -> Result<()> {
        registry::save(&self.registry).context("saving state - registry")?;

        Ok(())
    }
}
