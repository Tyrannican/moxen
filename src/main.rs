use anyhow::{Context, Result};
use clap::Parser;

use moxen::{Cli, MoxenCommand, is_initialised, state::MoxenApp};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.command == MoxenCommand::Init {
        MoxenApp::initialise().context("moxen initialisation")?;
        return Ok(());
    }

    if !is_initialised().context("initialisation")? {
        return Ok(());
    }

    let mut state = MoxenApp::new().context("loading application")?;
    match cli.command {
        MoxenCommand::List => {
            state.list_contents();
            Ok(())
        }
        MoxenCommand::Track { addon_ids } => state
            .track_addons(addon_ids)
            .await
            .context("tracking addons"),
        MoxenCommand::Update => state.update_addons().await.context("updating addons"),
        MoxenCommand::Switch { registry } => state
            .switch_game_version(registry)
            .context("switching game version"),
        MoxenCommand::ClearCache => state.clear_cache().context("clearing cache"),
        MoxenCommand::Install => state.install_addons().await.context("installing addons"),
        MoxenCommand::Uninstall { addon_ids } => state
            .uninstall_addons(addon_ids)
            .await
            .context("uninstalling addons"),
        MoxenCommand::Init => unreachable!("this is covered above"),
    }
}
