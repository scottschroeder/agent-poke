use anyhow::Result;
use clap::Args;

use crate::cli::commands::state_cleanup;
use crate::core::AgentState;
use crate::core::env_keys::PaneEnvKeys;
use crate::core::util::now_epoch_millis;
use crate::notify;

#[derive(Debug, Args)]
pub struct FocusResetCommand {
    #[arg(long)]
    pane: Option<String>,

    #[arg(long)]
    window: Option<String>,
}

impl FocusResetCommand {
    pub fn run(self) -> Result<()> {
        let Some(tmux) = state_cleanup::prepare_with_maintenance(true)? else {
            return Ok(());
        };

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

        let pane_keys = PaneEnvKeys::new(&pane_id);
        let Some(state_raw) = tmux.show_env_global(&pane_keys.state)? else {
            return Ok(());
        };
        let Ok(state) = state_raw.parse::<AgentState>() else {
            return Ok(());
        };

        if matches!(state, AgentState::Done | AgentState::NeedsInput) {
            log::info!(
                "focus-reset pane={} {} -> idle",
                pane_id,
                state,
            );
            if let Some(notification_id) = tmux.show_env_global(&pane_keys.notification_id)? {
                notify::clear_system_notification(&notification_id);
            }
            tmux.unset_env_global(&pane_keys.notification_id)?;
            tmux.unset_env_global(&pane_keys.last_notified_state)?;
            tmux.unset_env_global(&pane_keys.done)?;
            tmux.unset_env_global(&pane_keys.done_window)?;
            tmux.unset_env_global(&pane_keys.pending_reset)?;
            tmux.set_env_global(&pane_keys.state, AgentState::Idle.as_str())?;
            tmux.set_env_global(
                &pane_keys.last_heartbeat_ts_ms,
                &now_epoch_millis().to_string(),
            )?;
        }

        tmux.refresh_client();
        Ok(())
    }
}
