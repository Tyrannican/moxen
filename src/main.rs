use anyhow::{Context, Result};
use clap::Parser;

use moxen::{Cli, MoxenCommand, state::MoxenApp, store::MoxenConfig};

fn is_initialised() -> Result<bool> {
    if !MoxenConfig::is_initialised()? {
        eprintln!("you must initialise the Moxen app with `moxen init` first");
        return Ok(false);
    }

    Ok(true)
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.command == MoxenCommand::Init {
        MoxenApp::initialise().context("moxen initialisation")?;
        return Ok(());
    }

    if !is_initialised()? {
        return Ok(());
    }

    let mut state = MoxenApp::new().context("loading application")?;
    match cli.command {
        MoxenCommand::List => {
            state.list_contents();
        }
        MoxenCommand::Track { mod_ids } => {
            state.track_addons(mod_ids.clone()).await?;
        }
        MoxenCommand::Update => {
            state.update_addons().await.context("updating addons")?;
        }
        MoxenCommand::Switch { registry } => {
            state
                .switch_game_version(registry)
                .context("switching game version")?;
        }
        MoxenCommand::ClearCache => {
            state.clear_cache().context("clearing cache")?;
        }
        _ => unreachable!("covered above"),
    }

    Ok(())
}
