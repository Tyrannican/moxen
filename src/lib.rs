pub mod addon;
pub mod api;
pub mod state;
pub mod store;

use clap::{Parser, Subcommand};

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
        mod_ids: Vec<i32>,
    },

    /// List tracked addons in the registry
    List,

    /// Download the latest version of the addon(s)
    Update,
}
