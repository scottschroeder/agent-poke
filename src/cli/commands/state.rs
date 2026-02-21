use anyhow::{Context, Result};
use clap::Args;

use crate::config;
use crate::core::{AgentState, NotificationContext};
use crate::notify;
use crate::tmux::Tmux;

#[derive(Debug, Args)]
pub struct StateCommand {
    #[arg(long)]
    agent: String,

    #[arg(long)]
    state: String,

    #[arg(long)]
    pane: Option<String>,
}

impl StateCommand {
    pub fn run(self) -> Result<()> {
        let tmux = Tmux::new();
        if !tmux.is_available() {
            return Ok(());
        }

        let state = self
            .state
            .parse::<AgentState>()
            .map_err(anyhow::Error::msg)?;
        let pane_id = self.resolve_pane_id(&tmux)?;
        let window_id = tmux
            .display_target(&pane_id, "#{window_id}")
            .with_context(|| format!("failed to resolve window id for pane {pane_id}"))?;

        let state_key = format!("TMUX_AGENT_PANE_{pane_id}_STATE");
        let agent_key = format!("TMUX_AGENT_PANE_{pane_id}_AGENT");
        let done_key = format!("TMUX_AGENT_PANE_{pane_id}_DONE");
        let done_window_key = format!("TMUX_AGENT_PANE_{pane_id}_DONE_WINDOW");
        let pending_reset_key = format!("TMUX_AGENT_PANE_{pane_id}_PENDING_RESET");

        let previous_state = tmux
            .show_env_global(&state_key)?
            .and_then(|value| value.parse::<AgentState>().ok());
        if !AgentState::is_valid_transition(previous_state, state) {
            log::warn!(
                "ignoring unusual transition for agent={} pane={} from={:?} to={}",
                self.agent,
                pane_id,
                previous_state,
                state
            );
        }

        match state {
            AgentState::Off => {
                tmux.unset_env_global(&state_key)?;
                tmux.unset_env_global(&agent_key)?;
                tmux.unset_env_global(&done_key)?;
                tmux.unset_env_global(&done_window_key)?;
                tmux.unset_env_global(&pending_reset_key)?;
                tmux.unset_env_global(&format!("TMUX_AGENT_ACTIVE_PANE_{}", self.agent))?;
            }
            AgentState::Done => {
                tmux.set_env_global(&state_key, state.as_str())?;
                tmux.set_env_global(&agent_key, &self.agent)?;
                tmux.set_env_global(&done_key, "1")?;
                tmux.set_env_global(&done_window_key, &window_id)?;
                tmux.set_env_global(&format!("TMUX_AGENT_ACTIVE_PANE_{}", self.agent), &pane_id)?;

                if config::enabled(&tmux, "@agent-indicator-reset-on-focus", true) {
                    tmux.set_env_global(&pending_reset_key, "1")?;
                }
            }
            AgentState::Running | AgentState::NeedsInput => {
                tmux.set_env_global(&state_key, state.as_str())?;
                tmux.set_env_global(&agent_key, &self.agent)?;
                tmux.unset_env_global(&done_key)?;
                tmux.unset_env_global(&done_window_key)?;
                tmux.unset_env_global(&pending_reset_key)?;
                tmux.set_env_global(&format!("TMUX_AGENT_ACTIVE_PANE_{}", self.agent), &pane_id)?;
            }
        }

        if config::state_notification_enabled(&tmux, state) {
            self.notify(&tmux, state, &pane_id)?;
        }

        tmux.refresh_client();
        Ok(())
    }

    fn resolve_pane_id(&self, tmux: &Tmux) -> Result<String> {
        if let Some(pane) = self.pane.as_deref() {
            return Ok(pane.to_string());
        }

        if let Ok(from_env) = std::env::var("TMUX_PANE")
            && !from_env.is_empty()
        {
            return Ok(from_env);
        }

        tmux.display_current("#{pane_id}")
            .context("failed to resolve current tmux pane id")
    }

    fn notify(&self, tmux: &Tmux, state: AgentState, pane_id: &str) -> Result<()> {
        let session_name = tmux.display_target(pane_id, "#S")?;
        let window_name = tmux.display_target(pane_id, "#W")?;
        let window_index = tmux.display_target(pane_id, "#I")?;

        let context = NotificationContext {
            agent_name: self.agent.clone(),
            agent_state: state,
            session_name,
            window_name,
            window_index,
        };

        let message = context.render(&config::notification_format(tmux));
        tmux.display_message(&message);

        let _ = notify::notify_system("agent-poke", &message);
        Ok(())
    }
}
