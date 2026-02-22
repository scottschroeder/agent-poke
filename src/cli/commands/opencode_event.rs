use anyhow::Result;
use clap::Args;

use crate::cli::commands::state::run_state_prepared;
use crate::cli::commands::state::{now_epoch_millis, resolve_pane_id};
use crate::cli::commands::state_cleanup;
use crate::core::AgentState;
use crate::core::env_keys::PaneEnvKeys;
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
    let tmux = Tmux::new();
    if !tmux.is_available() {
        return Ok(());
    }

    state_cleanup::cleanup_stale_state(&tmux, false)?;
    state_cleanup::reconcile_state(&tmux)?;

    let pane_id = resolve_pane_id(&tmux, pane.as_deref())?;
    let event_ts_ms = event_ts_ms.unwrap_or_else(now_epoch_millis);

    match map_event_to_action(event, status) {
        OpenCodeEventAction::Transition(state) => {
            let guard = EventGuard::new(&tmux, pane_id.clone(), DEFAULT_LOCK_TIMEOUT);
            match guard.apply(state, event_ts_ms, || {
                run_state_prepared(agent.clone(), state, Some(pane_id.clone()))
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
                    log::debug!(
                        "skipped event due pane lock timeout after {}ms event={} status={:?} pane={} ts_ms={} priority={}",
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
        OpenCodeEventAction::HeartbeatOnly => {
            touch_heartbeat(&tmux, &pane_id, event_ts_ms)?;
            log::debug!(
                "applied heartbeat-only opencode event event={} status={:?} pane={} ts_ms={}",
                event,
                status,
                pane_id,
                event_ts_ms,
            );
        }
    }

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OpenCodeEventAction {
    Transition(AgentState),
    HeartbeatOnly,
}

fn map_event_to_action(event: &str, status: Option<&str>) -> OpenCodeEventAction {
    match event {
        "permission.asked" => OpenCodeEventAction::Transition(AgentState::NeedsInput),
        "session.idle" | "session.error" => OpenCodeEventAction::Transition(AgentState::Done),
        "session.status" => match status {
            Some("busy") | Some("retry") => OpenCodeEventAction::Transition(AgentState::Running),
            Some("idle") => OpenCodeEventAction::Transition(AgentState::Done),
            _ => OpenCodeEventAction::HeartbeatOnly,
        },
        _ => OpenCodeEventAction::HeartbeatOnly,
    }
}

fn touch_heartbeat(tmux: &Tmux, pane_id: &str, ts_ms: u64) -> Result<()> {
    let keys = PaneEnvKeys::new(pane_id);
    let Some(state_raw) = tmux.show_env_global(&keys.state)? else {
        return Ok(());
    };
    let Ok(state) = state_raw.parse::<AgentState>() else {
        return Ok(());
    };
    if state != AgentState::Running {
        return Ok(());
    }

    tmux.set_env_global(&keys.last_heartbeat_ts_ms, &ts_ms.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{OpenCodeEventAction, map_event_to_action, state_priority};
    use crate::core::AgentState;

    #[test]
    fn maps_opencode_permission_event() {
        assert_eq!(
            map_event_to_action("permission.asked", None),
            OpenCodeEventAction::Transition(AgentState::NeedsInput)
        );
    }

    #[test]
    fn maps_opencode_session_status_busy_to_running() {
        assert_eq!(
            map_event_to_action("session.status", Some("busy")),
            OpenCodeEventAction::Transition(AgentState::Running)
        );
    }

    #[test]
    fn maps_opencode_session_status_idle_to_done() {
        assert_eq!(
            map_event_to_action("session.status", Some("idle")),
            OpenCodeEventAction::Transition(AgentState::Done)
        );
    }

    #[test]
    fn treats_unknown_events_as_heartbeat() {
        assert_eq!(
            map_event_to_action("session.updated", None),
            OpenCodeEventAction::HeartbeatOnly
        );
    }

    #[test]
    fn ranks_done_and_input_over_running() {
        assert!(state_priority(AgentState::Done) > state_priority(AgentState::Running));
        assert!(state_priority(AgentState::NeedsInput) > state_priority(AgentState::Done));
    }
}
