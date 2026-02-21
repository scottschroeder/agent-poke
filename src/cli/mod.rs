mod commands;

use anyhow::Result;
use clap::{ArgAction, Parser, Subcommand};

use commands::focus_reset::FocusResetCommand;
use commands::indicator::IndicatorCommand;
use commands::install::InstallCommand;
use commands::state::StateCommand;
use commands::uninstall::UninstallCommand;

#[derive(Debug, Parser)]
#[command(name = "agent-poke")]
#[command(about = "Event-driven tmux attention notifier for coding agents")]
pub struct Cli {
    #[arg(short, long, action = ArgAction::Count, global = true)]
    pub verbose: u8,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    State(StateCommand),
    Indicator(IndicatorCommand),
    FocusReset(FocusResetCommand),
    Install(InstallCommand),
    Uninstall(UninstallCommand),
}

impl Cli {
    pub fn run(self) -> Result<()> {
        match self.command {
            Commands::State(cmd) => cmd.run(),
            Commands::Indicator(cmd) => cmd.run(),
            Commands::FocusReset(cmd) => cmd.run(),
            Commands::Install(cmd) => cmd.run(),
            Commands::Uninstall(cmd) => cmd.run(),
        }
    }
}
