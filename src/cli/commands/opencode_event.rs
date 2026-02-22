use anyhow::Result;
use clap::Args;

use crate::cli::commands::state::run_state;
use crate::cli::commands::state_cleanup;
use crate::core::AgentState;
use crate::core::event_guard::{
    DEFAULT_LOCK_TIMEOUT, EventApplyOutcome, EventGuard, state_priority,
};
use crate::tmux::Tmux;

#[derive(Debug, Args)]
pub struct OpenCodeEventCommand {
    #[arg(long)]
    event: String,

    #[arg(long)]
    status: Option<String>,

    #[arg(long, default_value = "opencode")]
    agent: String,

    #[arg(long)]
    pane: Option<String>,

    #[arg(long)]
    event_ts_ms: Option<u64>,
}

impl OpenCodeEventCommand {
    pub fn run(self) -> Result<()> {
        run_opencode_event(
            self.agent,
            &self.event,
            self.status.as_deref(),
            self.pane,
            self.event_ts_ms,
        )
    }
}

pub(crate) fn run_opencode_event(
    agent: String,
    event: &str,
    status: Option<&str>,
    pane: Option<String>,
    event_ts_ms: Option<u64>,
) -> Result<()> {
    let state = map_event_to_state(event, status);
    if let Some(state) = state {
        let tmux = Tmux::new();
        if !tmux.is_available() {
            return Ok(());
        }

        state_cleanup::cleanup_stale_state(&tmux, false)?;

        let pane_id = resolve_pane_id(&tmux, pane.as_deref())?;
        let event_ts_ms = event_ts_ms.unwrap_or_else(now_epoch_millis);

        let guard = EventGuard::new(&tmux, pane_id.clone(), DEFAULT_LOCK_TIMEOUT);
        match guard.apply(state, event_ts_ms, || {
            run_state(agent.clone(), state, Some(pane_id.clone()))
        })? {
            EventApplyOutcome::Applied => {
                log::debug!(
                    "applied opencode event event={} status={:?} pane={} ts_ms={} priority={}",
                    event,
                    status,
                    pane_id,
                    event_ts_ms,
                    state_priority(state)
                );
            }
            EventApplyOutcome::IgnoredStale => {
                log::debug!(
                    "ignored stale opencode event event={} status={:?} pane={} ts_ms={} priority={}",
                    event,
                    status,
                    pane_id,
                    event_ts_ms,
                    state_priority(state)
                );
            }
            EventApplyOutcome::LockTimeout => {
                log::warn!(
                    "timed out waiting for pane event lock after {}ms event={} status={:?} pane={} ts_ms={} priority={}",
                    DEFAULT_LOCK_TIMEOUT.as_millis(),
                    event,
                    status,
                    pane_id,
                    event_ts_ms,
                    state_priority(state)
                );
            }
        }
    }
    Ok(())
}

fn map_event_to_state(event: &str, status: Option<&str>) -> Option<AgentState> {
    match event {
        "permission.asked" | "permission.ask" => Some(AgentState::NeedsInput),
        "session.idle" | "session.error" => Some(AgentState::Done),
        "session.status" => match status {
            Some("busy") | Some("retry") => Some(AgentState::Running),
            Some("idle") => Some(AgentState::Done),
            _ => None,
        },
        _ => None,
    }
}

fn resolve_pane_id(tmux: &Tmux, pane: Option<&str>) -> Result<String> {
    if let Some(pane) = pane {
        return Ok(pane.to_string());
    }

    if let Ok(from_env) = std::env::var("TMUX_PANE")
        && !from_env.is_empty()
    {
        return Ok(from_env);
    }

    tmux.display_current("#{pane_id}")
}

fn now_epoch_millis() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};

    let Ok(duration) = SystemTime::now().duration_since(UNIX_EPOCH) else {
        return 0;
    };
    duration.as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::{map_event_to_state, state_priority};
    use crate::core::AgentState;

    #[test]
    fn maps_opencode_permission_event() {
        assert_eq!(
            map_event_to_state("permission.asked", None),
            Some(AgentState::NeedsInput)
        );
    }

    #[test]
    fn maps_opencode_legacy_permission_event() {
        assert_eq!(
            map_event_to_state("permission.ask", None),
            Some(AgentState::NeedsInput)
        );
    }

    #[test]
    fn maps_opencode_session_status_busy_to_running() {
        assert_eq!(
            map_event_to_state("session.status", Some("busy")),
            Some(AgentState::Running)
        );
    }

    #[test]
    fn maps_opencode_session_status_idle_to_done() {
        assert_eq!(
            map_event_to_state("session.status", Some("idle")),
            Some(AgentState::Done)
        );
    }

    #[test]
    fn ignores_unknown_events() {
        assert_eq!(map_event_to_state("session.updated", None), None);
    }

    #[test]
    fn ranks_done_and_input_over_running() {
        assert!(state_priority(AgentState::Done) > state_priority(AgentState::Running));
        assert!(state_priority(AgentState::NeedsInput) > state_priority(AgentState::Done));
    }
}
