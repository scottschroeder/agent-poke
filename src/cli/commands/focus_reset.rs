use anyhow::Result;
use clap::Args;

use crate::tmux::Tmux;

#[derive(Debug, Args)]
pub struct FocusResetCommand {
    #[arg(long)]
    pane: Option<String>,

    #[arg(long)]
    window: Option<String>,
}

impl FocusResetCommand {
    pub fn run(self) -> Result<()> {
        let tmux = Tmux::new();
        if !tmux.is_available() {
            return Ok(());
        }

        let pane_id = self
            .pane
            .or_else(|| std::env::var("TMUX_PANE").ok())
            .unwrap_or(tmux.display_current("#{pane_id}")?);
        let window_id = self
            .window
            .unwrap_or(tmux.display_target(&pane_id, "#{window_id}")?);

        let pane_active = tmux.display_target(&pane_id, "#{pane_active}")?;
        let window_active = tmux.display_target(&window_id, "#{window_active}")?;
        if pane_active != "1" || window_active != "1" {
            return Ok(());
        }

        let state_key = format!("TMUX_AGENT_PANE_{pane_id}_STATE");
        let agent_key = format!("TMUX_AGENT_PANE_{pane_id}_AGENT");
        let done_key = format!("TMUX_AGENT_PANE_{pane_id}_DONE");
        let done_window_key = format!("TMUX_AGENT_PANE_{pane_id}_DONE_WINDOW");
        let pending_reset_key = format!("TMUX_AGENT_PANE_{pane_id}_PENDING_RESET");

        tmux.unset_env_global(&pending_reset_key)?;
        tmux.unset_env_global(&done_key)?;
        tmux.unset_env_global(&done_window_key)?;
        tmux.unset_env_global(&state_key)?;
        tmux.unset_env_global(&agent_key)?;
        tmux.refresh_client();
        Ok(())
    }
}
