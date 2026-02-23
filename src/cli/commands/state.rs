use anyhow::{Context, Result};
use clap::Args;

use crate::cli::commands::state_cleanup;
use crate::config;
use crate::core::AgentState;
use crate::core::env_keys::{PaneEnvKeys, active_pane_key};
use crate::notify;
use crate::notify::NotifyOptions;
use crate::tmux::Tmux;

#[derive(Debug, Args)]
pub struct StateCommand {
    #[arg(long)]
    agent: String,

    #[arg(long)]
    state: String,

    #[arg(long)]
    pane: Option<String>,

    #[arg(long)]
    event: Option<String>,
}

impl StateCommand {
    pub fn run(self) -> Result<()> {
        let state = self
            .state
            .parse::<AgentState>()
            .map_err(anyhow::Error::msg)?;
        run_state(
            self.agent,
            state,
            self.pane,
            self.event.as_deref(),
        )
    }
}

pub(crate) fn run_state(
    agent: String,
    input_state: AgentState,
    pane: Option<String>,
    event: Option<&str>,
) -> Result<()> {
    run_state_impl(agent, input_state, pane, event, true)
}

pub(crate) fn run_state_prepared(
    agent: String,
    input_state: AgentState,
    pane: Option<String>,
) -> Result<()> {
    run_state_impl(agent, input_state, pane, None, false)
}

fn run_state_impl(
    agent: String,
    input_state: AgentState,
    pane: Option<String>,
    event: Option<&str>,
    run_maintenance: bool,
) -> Result<()> {
    let tmux = Tmux::new();
    if !tmux.is_available() {
        return Ok(());
    }

    if run_maintenance {
        state_cleanup::cleanup_stale_state(&tmux, false)?;
        state_cleanup::reconcile_state(&tmux)?;
    }

    let pane_id = resolve_pane_id(&tmux, pane.as_deref())?;
    let pane_keys = PaneEnvKeys::new(&pane_id);

    let mut state = input_state;
    if state != AgentState::Closed && !config::state_enabled(&tmux, state) {
        state = AgentState::Idle;
    }

    let previous_state = tmux
        .show_env_global(&pane_keys.state)?
        .and_then(|value| value.parse::<AgentState>().ok());
    if !AgentState::is_valid_transition(previous_state, state) {
        log::warn!(
            "ignoring unusual transition for agent={} pane={} from={:?} to={}",
            agent,
            pane_id,
            previous_state,
            state
        );
    }

    log::info!(
        "transition agent={} pane={} {:?} -> {}{}",
        agent,
        pane_id,
        previous_state.map(|s| s.as_str()).unwrap_or("None"),
        state,
        event
            .map(|e| format!(" event={e}"))
            .unwrap_or_default(),
    );

    let previous_agent = tmux.show_env_global(&pane_keys.agent)?;
    let now_ms = now_epoch_millis();

    if state == AgentState::Closed {
        if let Some(notification_id) = tmux.show_env_global(&pane_keys.notification_id)? {
            notify::clear_system_notification(&notification_id);
        }

        tmux.set_env_global(&pane_keys.state, state.as_str())?;
        tmux.set_env_global(&pane_keys.closed_ts_ms, &now_ms.to_string())?;
        tmux.unset_env_global(&pane_keys.done)?;
        tmux.unset_env_global(&pane_keys.done_window)?;
        tmux.unset_env_global(&pane_keys.pending_reset)?;
        tmux.unset_env_global(&pane_keys.notification_id)?;
        tmux.unset_env_global(&pane_keys.last_notified_state)?;

        tmux.unset_env_global(&active_pane_key(&agent))?;
        if let Some(previous_agent) = previous_agent.as_deref()
            && previous_agent != agent
        {
            tmux.unset_env_global(&active_pane_key(previous_agent))?;
        }

        tmux.refresh_client();
        return Ok(());
    }

    if should_create_instance(
        previous_state,
        tmux.show_env_global(&pane_keys.instance_id)?.as_deref(),
    ) {
        tmux.set_env_global(&pane_keys.instance_id, &new_instance_id(&pane_id, now_ms))?;
        if let Some(pane_pid) = tmux.pane_pid(&pane_id)? {
            tmux.set_env_global(&pane_keys.pane_pid_at_start, &pane_pid.to_string())?;
            tmux.set_env_global(&pane_keys.agent_pid, &pane_pid.to_string())?;
        }
        if let Some(command) = tmux.pane_current_command(&pane_id)? {
            tmux.set_env_global(&pane_keys.pane_cmd_at_start, &command)?;
        }
    }

    tmux.set_env_global(&pane_keys.state, state.as_str())?;
    tmux.set_env_global(&pane_keys.agent, &agent)?;
    tmux.unset_env_global(&pane_keys.closed_ts_ms)?;
    tmux.set_env_global(&active_pane_key(&agent), &pane_id)?;

    if state == AgentState::Done {
        let window_id = tmux
            .display_target(&pane_id, "#{window_id}")
            .with_context(|| format!("failed to resolve window id for pane {pane_id}"))?;
        tmux.set_env_global(&pane_keys.done, "1")?;
        tmux.set_env_global(&pane_keys.done_window, &window_id)?;
        if config::enabled(&tmux, "@agent-indicator-reset-on-focus", true) {
            tmux.set_env_global(&pane_keys.pending_reset, "1")?;
        } else {
            tmux.unset_env_global(&pane_keys.pending_reset)?;
        }
    } else {
        tmux.unset_env_global(&pane_keys.done)?;
        tmux.unset_env_global(&pane_keys.done_window)?;
        tmux.unset_env_global(&pane_keys.pending_reset)?;
    }

    if state == AgentState::Running {
        tmux.set_env_global(&pane_keys.last_heartbeat_ts_ms, &now_ms.to_string())?;
    }

    if should_notify(&tmux, &pane_keys, state)? {
        let notification_id = notify(&tmux, &agent, state, &pane_id)?;
        if let Some(notification_id) = notification_id {
            tmux.set_env_global(&pane_keys.notification_id, &notification_id)?;
        } else {
            tmux.unset_env_global(&pane_keys.notification_id)?;
        }
        tmux.set_env_global(&pane_keys.last_notified_state, state.as_str())?;
    } else if matches!(state, AgentState::Idle | AgentState::Running) {
        if let Some(notification_id) = tmux.show_env_global(&pane_keys.notification_id)? {
            notify::clear_system_notification(&notification_id);
        }
        tmux.unset_env_global(&pane_keys.notification_id)?;
        tmux.unset_env_global(&pane_keys.last_notified_state)?;
    }

    tmux.refresh_client();
    Ok(())
}

fn should_notify(tmux: &Tmux, pane_keys: &PaneEnvKeys, state: AgentState) -> Result<bool> {
    if !matches!(state, AgentState::Done | AgentState::NeedsInput) {
        return Ok(false);
    }
    if !config::state_notification_enabled(tmux, state) {
        return Ok(false);
    }

    let last_notified = tmux.show_env_global(&pane_keys.last_notified_state)?;
    Ok(last_notified.as_deref() != Some(state.as_str()))
}

fn should_create_instance(previous_state: Option<AgentState>, instance_id: Option<&str>) -> bool {
    if instance_id.map(str::is_empty).unwrap_or(true) {
        return true;
    }

    previous_state.is_none() || previous_state == Some(AgentState::Closed)
}

pub(crate) fn resolve_pane_id(tmux: &Tmux, pane: Option<&str>) -> Result<String> {
    if let Some(pane) = pane {
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

pub(crate) fn now_epoch_millis() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};

    let Ok(duration) = SystemTime::now().duration_since(UNIX_EPOCH) else {
        return 0;
    };
    duration.as_millis() as u64
}

fn new_instance_id(pane_id: &str, now_ms: u64) -> String {
    let pid = std::process::id();
    let normalized_pane = pane_id.replace('%', "p");
    format!("{normalized_pane}-{pid}-{now_ms}")
}

fn notify(tmux: &Tmux, agent: &str, state: AgentState, pane_id: &str) -> Result<Option<String>> {
    let session_name = tmux.display_target(pane_id, "#S")?;
    let window_name = tmux.display_target(pane_id, "#W")?;

    let message = format!(
        "[{}] {} > {}",
        state.notification_label(),
        session_name,
        window_name
    );

    let options = NotifyOptions {
        persistent: config::notification_persistent(tmux),
        urgency: config::notification_urgency(tmux),
        sound: config::notification_sound_enabled(tmux),
        sound_name: config::notification_sound_name(tmux),
        replace_tag: Some(format!("agent-poke-{pane_id}")),
    };

    let notification_id = notify::notify_system(agent, &message, &options)?;
    Ok(notification_id)
}

#[cfg(test)]
mod tests {
    use super::should_create_instance;
    use crate::core::AgentState;

    #[test]
    fn creates_instance_when_record_missing() {
        assert!(should_create_instance(None, None));
        assert!(should_create_instance(Some(AgentState::Running), Some("")));
    }

    #[test]
    fn creates_instance_after_closed() {
        assert!(should_create_instance(
            Some(AgentState::Closed),
            Some("abc")
        ));
    }

    #[test]
    fn keeps_existing_instance_for_live_state() {
        assert!(!should_create_instance(Some(AgentState::Idle), Some("abc")));
    }
}
