pub mod addon;
pub mod api;
pub mod state;
pub mod store;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

use crate::store::GameVersion;

#[derive(Debug, Parser)]
pub struct Cli {
    #[clap(subcommand)]
    pub command: MoxenCommand,
}

#[derive(Debug, PartialEq, Subcommand)]
pub enum MoxenCommand {
    /// Initialise Moxen
    Init,

    /// Track new addons in the registry
    Track {
        /// List of Addon IDs to track (Project IDs from Curseforge)
        addon_ids: Vec<i32>,
    },

    /// Switch registry to use (retail, ptr, beta, classic)
    Switch {
        /// Game version to use
        registry: GameVersion,
    },

    /// List tracked addons in the registry
    List,

    /// Clear the Moxen file cache
    ClearCache,

    /// Download the latest version of the addon(s)
    Update,

    /// Install the addons in the WoW directory
    Install,

    /// Uninstall the selected Addons
    Uninstall {
        /// Addon IDs to remove
        addon_ids: Vec<i32>,
    },
}

pub fn is_initialised() -> Result<bool> {
    if !store::MoxenConfig::is_initialised().context("checking moxen initialisation from config")? {
        eprintln!("you must initialise the Moxen app with `moxen init` first");
        return Ok(false);
    }

    Ok(true)
}
