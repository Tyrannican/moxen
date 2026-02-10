use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::addon::Addon;

// TODO: Custom error handling
// TODO: Track `GameVersion` in the config (retail, beta, classic etc)

#[derive(Deserialize, Serialize)]
pub struct MoxenConfig {
    pub api_key: String,
}

impl MoxenConfig {
    pub fn is_initialised() -> Result<bool> {
        let cfg_path = load_path("config.toml").context("config file path")?;
        let registry_path = load_path("registry.json").context("registry file path")?;
        Ok(cfg_path.exists() && registry_path.exists())
    }

    pub fn initialise() -> Result<()> {
        let cfg_path = load_path("config.toml")?;
        let api_key =
            rpassword::prompt_password("Enter Curseforge API Key: ").context("reading password")?;

        let cfg = MoxenConfig { api_key };
        let content = toml::to_string_pretty(&cfg).context("serialising config")?;
        std::fs::write(&cfg_path, &content).context("writing out config file")?;

        Ok(())
    }

    pub fn load() -> Result<Self> {
        let cfg_path = load_path("config.toml")?;
        let content = std::fs::read_to_string(&cfg_path).context("reading config file")?;
        toml::from_str(&content).context("deserialising config")
    }
}

pub fn root_path() -> Result<PathBuf> {
    let Some(root) = dotstore::home_store("moxen").context("initialising home store")? else {
        eprintln!("unable to get path to home directory");
        panic!("USE CUSTOM ERROR");
    };

    Ok(root)
}

pub fn load_path(file: impl AsRef<Path>) -> Result<PathBuf> {
    let root = root_path().context("loading home store directory")?;
    let path = root.join(file);

    Ok(path)
}

pub mod registry {
    use super::*;

    pub type MoxenRegistry = HashMap<i32, Addon>;

    pub fn initialise() -> Result<()> {
        let Some(root) = dotstore::home_store("moxen").context("initialising registry")? else {
            panic!("unable to create home store directory");
        };

        let registry_path = root.join("registry.json");

        let reg = MoxenRegistry::new();
        let registry = serde_json::to_string_pretty(&reg).context("serialising registry")?;
        std::fs::write(&registry_path, &registry).context("writing new registry")
    }

    pub fn load() -> Result<MoxenRegistry> {
        let Some(root) = dotstore::home_store("moxen").context("initialising registry")? else {
            panic!("unable to create home store directory");
        };

        let registry_path = root.join("registry.json");
        let content = std::fs::read_to_string(&registry_path).context("reading registry file")?;
        let registry = serde_json::from_str(&content).context("deserialising registry")?;

        Ok(registry)
    }

    pub fn save(registry: &MoxenRegistry) -> Result<()> {
        let Some(root) = dotstore::home_store("moxen").context("initialising registry")? else {
            panic!("unable to create home store directory");
        };

        let registry_path = root.join("registry.json");
        let contents = serde_json::to_string_pretty(registry).context("serialising registry")?;
        std::fs::write(&registry_path, &contents).context("saving registry to disk")?;

        Ok(())
    }
}
