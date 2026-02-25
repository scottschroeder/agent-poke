use anyhow::Result;
use clap::Args;

use crate::cli::commands::state::resolve_pane_id;
use crate::core::AgentState;
use crate::core::env_keys::PaneEnvKeys;
use crate::core::util::now_epoch_millis;
use crate::tmux::Tmux;

#[derive(Debug, Args)]
pub struct HeartbeatCommand {
    #[arg(long)]
    agent: String,

    #[arg(long)]
    pane: Option<String>,

    #[arg(long)]
    event: Option<String>,
}

impl HeartbeatCommand {
    pub fn run(self) -> Result<()> {
        let tmux = Tmux::new();
        if !tmux.is_available() {
            return Ok(());
        }

        let pane_id = resolve_pane_id(&tmux, self.pane.as_deref())?;
        let keys = PaneEnvKeys::new(&pane_id);

        let Some(raw_state) = tmux.show_env_global(&keys.state)? else {
            log::info!(
                "heartbeat: no state for pane={} agent={} event={:?}, skipping",
                pane_id,
                self.agent,
                self.event,
            );
            return Ok(());
        };
        let Ok(state) = raw_state.parse::<AgentState>() else {
            return Ok(());
        };

        if state != AgentState::Running {
            log::info!(
                "heartbeat: pane={} state={} (not running), skipping event={:?}",
                pane_id,
                state.as_str(),
                self.event,
            );
            return Ok(());
        }

        let now_ms = now_epoch_millis();
        tmux.set_env_global(&keys.last_heartbeat_ts_ms, &now_ms.to_string())?;

        log::info!(
            "heartbeat: updated pane={} agent={} event={:?} ts_ms={}",
            pane_id,
            self.agent,
            self.event,
            now_ms,
        );

        Ok(())
    }
}
