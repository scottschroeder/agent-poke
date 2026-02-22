mod commands;

use anyhow::Result;
use clap::{ArgAction, Parser, Subcommand};

use commands::debug::DebugCommand;
use commands::focus_reset::FocusResetCommand;
use commands::heartbeat::HeartbeatCommand;
use commands::opencode_event::OpenCodeEventCommand;
use commands::pane_status::PaneStatusCommand;
use commands::state::StateCommand;
use commands::window_status::WindowStatusCommand;

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
    Heartbeat(HeartbeatCommand),
    FocusReset(FocusResetCommand),
    OpencodeEvent(OpenCodeEventCommand),
    PaneStatus(PaneStatusCommand),
    Debug(DebugCommand),
    WindowStatus(WindowStatusCommand),
}

impl Cli {
    pub fn run(self) -> Result<()> {
        match self.command {
            Commands::State(cmd) => cmd.run(),
            Commands::Heartbeat(cmd) => cmd.run(),
            Commands::FocusReset(cmd) => cmd.run(),
            Commands::OpencodeEvent(cmd) => cmd.run(),
            Commands::PaneStatus(cmd) => cmd.run(),
            Commands::Debug(cmd) => cmd.run(),
            Commands::WindowStatus(cmd) => cmd.run(),
        }
    }
}
