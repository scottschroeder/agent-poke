use anyhow::Result;
use clap::{Args, Subcommand};

use crate::cli::commands::debug_clear_state::DebugClearStateCommand;
use crate::cli::commands::debug_state::DebugStateCommand;
use crate::cli::commands::debug_stress::DebugStressCommand;
use crate::cli::commands::state_poll::StatePollCommand;

#[derive(Debug, Args)]
pub struct DebugCommand {
    #[command(subcommand)]
    command: DebugCommands,
}

#[derive(Debug, Subcommand)]
enum DebugCommands {
    State(DebugStateCommand),
    Poll(StatePollCommand),
    Stress(DebugStressCommand),
    ClearState(DebugClearStateCommand),
}

impl DebugCommand {
    pub fn run(self) -> Result<()> {
        match self.command {
            DebugCommands::State(cmd) => cmd.run(),
            DebugCommands::Poll(cmd) => cmd.run(),
            DebugCommands::Stress(cmd) => cmd.run(),
            DebugCommands::ClearState(cmd) => cmd.run(),
        }
    }
}
