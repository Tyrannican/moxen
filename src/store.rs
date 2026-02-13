use std::{io::Write, path::PathBuf};

use anyhow::{Context, Result};
use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::addon::Addon;
use path::MoxenPath;

#[derive(Deserialize, Serialize)]
pub struct AddonInstallPath(pub PathBuf);

impl AddonInstallPath {
    pub fn addon_dir(&self, version: &GameVersion) -> PathBuf {
        self.0
            .join(format!("{}/Interface/AddOns", version.suffix()))
    }
}

impl std::fmt::Display for AddonInstallPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.display())
    }
}

impl Default for AddonInstallPath {
    fn default() -> Self {
        Self(MoxenPath::root().expect("this is for testing"))
    }
}

const VERSIONS: [GameVersion; 5] = [
    GameVersion::Retail,
    GameVersion::Beta,
    GameVersion::Ptr,
    GameVersion::Classic,
    GameVersion::ClassicEra,
];

#[derive(Deserialize, Serialize, Default, Debug, PartialEq, Eq, Copy, Clone, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum GameVersion {
    #[default]
    Retail,
    Beta,
    Ptr,
    Classic,
    ClassicEra,
}

impl GameVersion {
    pub fn registry_path(&self) -> Result<PathBuf> {
        let registry = MoxenPath::dir("registry").context("loading registry path")?;
        if !registry.exists() {
            std::fs::create_dir_all(&registry).context("creating registry dir")?;
        }

        Ok(registry.join(format!("{}.json", self)))
    }

    pub fn suffix(&self) -> String {
        match self {
            Self::Retail => "_retail_".to_string(),
            Self::Beta => "_beta_".to_string(),
            Self::Ptr => "_ptr_".to_string(),
            Self::Classic => "_classic_".to_string(),
            Self::ClassicEra => "_classic_era_".to_string(),
        }
    }
}

impl std::fmt::Display for GameVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Retail => write!(f, "retail"),
            Self::Beta => write!(f, "beta"),
            Self::Classic => write!(f, "classic"),
            Self::ClassicEra => write!(f, "classic_era"),
            Self::Ptr => write!(f, "ptr"),
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct MoxenConfig {
    pub api_key: String,
    pub version: GameVersion,
    pub install_dir: AddonInstallPath,
}

impl MoxenConfig {
    pub fn is_initialised() -> Result<bool> {
        let cfg_path = MoxenPath::file("config.toml").context("config file path")?;
        let registries_exist = VERSIONS.iter().all(|version| {
            let path = version
                .registry_path()
                .context("loading registry path")
                .expect("error loading path");

            path.exists()
        });

        Ok(cfg_path.exists() && registries_exist)
    }

    pub fn initialise() -> Result<()> {
        let cfg_path = MoxenPath::file("config.toml")?;
        let api_key =
            rpassword::prompt_password("Enter Curseforge API Key: ").context("reading password")?;

        print!(
            "Enter World of Warcraft install directory (default is \"{}\"): ",
            AddonInstallPath::default()
        );
        std::io::stdout().flush().context("flushing stdout")?;
        let mut install_input = String::new();
        std::io::stdin()
            .read_line(&mut install_input)
            .context("reading user input")?;

        let install_dir = if install_input.trim().is_empty() {
            AddonInstallPath::default()
        } else {
            AddonInstallPath(PathBuf::from(install_input.trim()))
        };

        assert!(install_dir.0.exists());
        let cfg = MoxenConfig {
            api_key,
            version: GameVersion::default(),
            install_dir,
        };

        let content = toml::to_string_pretty(&cfg).context("serialising config")?;
        std::fs::write(&cfg_path, &content).context("writing out config file")?;

        Ok(())
    }

    pub fn load() -> Result<Self> {
        let cfg_path = MoxenPath::file("config.toml")?;
        let content = std::fs::read_to_string(&cfg_path).context("reading config file")?;
        toml::from_str(&content).context("deserialising config")
    }

    pub fn save(&self) -> Result<()> {
        let cfg_path = MoxenPath::file("config.toml")?;
        let content = toml::to_string_pretty(&self).context("serialising config")?;
        std::fs::write(&cfg_path, &content).context("writing out config file")?;

        Ok(())
    }
}

pub mod path {
    use anyhow::{Context, Result};
    use std::path::{Path, PathBuf};
    use zip::ZipArchive;

    pub struct MoxenPath;

    impl MoxenPath {
        pub fn root() -> Result<PathBuf> {
            let Some(root) = dotstore::home_store("moxen").context("initialising home store")?
            else {
                eprintln!("unable to get path to home directory");
                panic!("USE CUSTOM ERROR");
            };

            Ok(root)
        }

        pub fn dir(dir_name: impl AsRef<Path>) -> Result<PathBuf> {
            let root = Self::root().context("loading root path")?;
            let dir = root.join(dir_name);

            if !dir.exists() {
                std::fs::create_dir_all(&dir)
                    .with_context(|| format!("creating dir {}", dir.display()))?;
            }

            Ok(dir)
        }

        pub fn file(filename: impl AsRef<Path>) -> Result<PathBuf> {
            let root = Self::root().context("loading root path")?;
            let file = root.join(filename);

            Ok(file)
        }
    }

    pub fn unzip_archive(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> Result<()> {
        let file = std::fs::File::open(&src)
            .with_context(|| format!("opening zip archive: {}", src.as_ref().display()))?;

        let mut archive = ZipArchive::new(file).context("initialising ZipArchive")?;

        for idx in 0..archive.len() {
            let mut entry = archive.by_index(idx).context("retrieving zip entry")?;
            let Some(entry_path) = entry.enclosed_name() else {
                continue;
            };

            let dst_path = dst.as_ref().join(entry_path);

            if entry.is_dir() {
                std::fs::create_dir_all(&dst_path).with_context(|| {
                    format!("creating sub-dir for zip entry - {}", dst_path.display())
                })?;
            } else {
                if let Some(parent) = dst_path.parent() {
                    std::fs::create_dir_all(parent).with_context(|| {
                        format!(
                            "creating parent directory {} for {}",
                            parent.display(),
                            dst_path.display()
                        )
                    })?;
                }

                let mut output = std::fs::File::create(&dst_path)
                    .with_context(|| format!("creating file {}", dst_path.display()))?;

                std::io::copy(&mut entry, &mut output).with_context(|| {
                    format!(
                        "copying {:?} to {}",
                        entry.enclosed_name(),
                        dst_path.display()
                    )
                })?;
            }
        }

        Ok(())
    }
}

pub mod registry {
    use super::*;

    pub type MoxenRegistry = HashMap<i32, Addon>;

    pub fn initialise() -> Result<()> {
        for version in VERSIONS {
            let reg = MoxenRegistry::new();
            let registry_path = version.registry_path().context("registry path")?;
            let registry = serde_json::to_string_pretty(&reg).context("serialising registry")?;
            std::fs::write(&registry_path, &registry).context("writing new registry")?;
        }

        Ok(())
    }

    pub fn load(version: &GameVersion) -> Result<MoxenRegistry> {
        let registry_path = version.registry_path().context("registry path")?;
        let content = std::fs::read_to_string(&registry_path).context("reading registry file")?;
        let registry = serde_json::from_str(&content).context("deserialising registry")?;

        Ok(registry)
    }

    pub fn save(registry: &MoxenRegistry, version: &GameVersion) -> Result<()> {
        let registry_path = version.registry_path().context("registry path")?;
        let contents = serde_json::to_string_pretty(registry).context("serialising registry")?;
        std::fs::write(&registry_path, &contents).context("saving registry to disk")?;

        Ok(())
    }
}
