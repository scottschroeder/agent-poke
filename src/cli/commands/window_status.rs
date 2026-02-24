use anyhow::Result;
use clap::Args;

use crate::cli::commands::state_cleanup;
use crate::config;

#[derive(Debug, Args)]
pub struct WindowStatusCommand {
    #[arg(long)]
    window: Option<String>,
}

impl WindowStatusCommand {
    pub fn run(self) -> Result<()> {
        let Some(tmux) = state_cleanup::prepare_with_maintenance(false)? else {
            return Ok(());
        };

        if !config::enabled(&tmux, "@agent-indicator-window-status-enabled", true) {
            println!();
            return Ok(());
        }

        let window_id = match self.window {
            Some(window_id) => window_id,
            None => tmux.display_current("#{window_id}")?,
        };

        let glyphs = config::window_status_glyphs(&tmux);
        let mut has_running = false;
        let mut has_done = false;
        let mut has_needs_input = false;

        for pane in tmux.list_panes_in_window(&window_id)? {
            let key = format!("TMUX_AGENT_PANE_{}_STATE", pane.pane_id);
            let Some(state) = tmux.show_env_global(&key)? else {
                continue;
            };

            match state.as_str() {
                "needs-input" => has_needs_input = true,
                "done" => has_done = true,
                "running" => has_running = true,
                _ => {}
            }

            if has_needs_input {
                break;
            }
        }

        if has_needs_input {
            println!("{}", glyphs.needs_input);
        } else if has_done {
            println!("{}", glyphs.done);
        } else if has_running {
            println!("{}", glyphs.running);
        } else {
            println!();
        }

        Ok(())
    }
}
