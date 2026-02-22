use anyhow::{Context, Result};
use clap::Args;

use crate::cli::commands::state_cleanup;
use crate::config;
use crate::config::StateVisualConfig;
use crate::core::AgentState;
use crate::core::env_keys::{PaneEnvKeys, WindowEnvKeys, active_pane_key};
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
}

impl StateCommand {
    pub fn run(self) -> Result<()> {
        let state = self
            .state
            .parse::<AgentState>()
            .map_err(anyhow::Error::msg)?;
        run_state(self.agent, state, self.pane)
    }
}

pub(crate) fn run_state(
    agent: String,
    input_state: AgentState,
    pane: Option<String>,
) -> Result<()> {
    let tmux = Tmux::new();
    if !tmux.is_available() {
        return Ok(());
    }

    let stale_entries = state_cleanup::cleanup_stale_state(&tmux, false)?;
    if stale_entries > 0 {
        log::debug!("cleared {stale_entries} stale tmux env entries");
    }

    let mut state = input_state;
    let pane_id = resolve_pane_id(&tmux, pane.as_deref())?;
    let window_id = tmux
        .display_target(&pane_id, "#{window_id}")
        .with_context(|| format!("failed to resolve window id for pane {pane_id}"))?;

    let pane_keys = PaneEnvKeys::new(&pane_id);

    if state != AgentState::Off && !config::state_enabled(&tmux, state) {
        state = AgentState::Off;
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

    let visuals = match state {
        AgentState::Running | AgentState::NeedsInput | AgentState::Done => {
            Some(config::state_visual_config(&tmux, state))
        }
        AgentState::Off => None,
    };

    match state {
        AgentState::Off => {
            clear_window_title_style(&tmux, &window_id)?;
            tmux.unset_env_global(&pane_keys.state)?;
            tmux.unset_env_global(&pane_keys.agent)?;
            tmux.unset_env_global(&pane_keys.done)?;
            tmux.unset_env_global(&pane_keys.done_window)?;
            tmux.unset_env_global(&pane_keys.pending_reset)?;
            tmux.unset_env_global(&pane_keys.notification_id)?;
            tmux.unset_env_global(&pane_keys.last_event_ts_ms)?;
            tmux.unset_env_global(&pane_keys.last_event_priority)?;
            tmux.unset_env_global(&active_pane_key(&agent))?;
            restore_active_border_style(&tmux, &window_id)?;
        }
        AgentState::Done => {
            tmux.set_env_global(&pane_keys.state, state.as_str())?;
            tmux.set_env_global(&pane_keys.agent, &agent)?;
            tmux.set_env_global(&pane_keys.done, "1")?;
            tmux.set_env_global(&pane_keys.done_window, &window_id)?;
            tmux.set_env_global(&active_pane_key(&agent), &pane_id)?;

            if let Some(style) = visuals.as_ref() {
                apply_done_visuals(&tmux, &window_id, style)?;
            }

            if config::enabled(&tmux, "@agent-indicator-reset-on-focus", true) {
                tmux.set_env_global(&pane_keys.pending_reset, "1")?;
            } else {
                tmux.unset_env_global(&pane_keys.pending_reset)?;
                restore_active_border_style(&tmux, &window_id)?;
                clear_window_title_style(&tmux, &window_id)?;
                tmux.unset_env_global(&pane_keys.done)?;
                tmux.unset_env_global(&pane_keys.done_window)?;
                tmux.unset_env_global(&pane_keys.state)?;
                tmux.unset_env_global(&pane_keys.agent)?;
            }
        }
        AgentState::Running | AgentState::NeedsInput => {
            clear_window_title_style(&tmux, &window_id)?;
            tmux.unset_env_global(&pane_keys.done)?;
            tmux.unset_env_global(&pane_keys.done_window)?;
            tmux.unset_env_global(&pane_keys.pending_reset)?;
            tmux.unset_env_global(&pane_keys.notification_id)?;
            tmux.set_env_global(&pane_keys.state, state.as_str())?;
            tmux.set_env_global(&pane_keys.agent, &agent)?;
            tmux.set_env_global(&active_pane_key(&agent), &pane_id)?;

            if let Some(style) = visuals.as_ref() {
                apply_running_or_input_visuals(&tmux, &window_id, style)?;
            }
        }
    }

    if should_notify_transition(previous_state, state) && config::state_notification_enabled(&tmux, state)
    {
        let notification_id = notify(&tmux, &agent, state, &pane_id)?;
        if let Some(notification_id) = notification_id {
            tmux.set_env_global(&pane_keys.notification_id, &notification_id)?;
        } else {
            tmux.unset_env_global(&pane_keys.notification_id)?;
        }
    }

    tmux.refresh_client();
    Ok(())
}

fn should_notify_transition(previous_state: Option<AgentState>, state: AgentState) -> bool {
    matches!(state, AgentState::Done | AgentState::NeedsInput) && previous_state != Some(state)
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
        .context("failed to resolve current tmux pane id")
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

fn apply_running_or_input_visuals(
    tmux: &Tmux,
    window_id: &str,
    style: &StateVisualConfig,
) -> Result<()> {
    if config::enabled(tmux, "@agent-indicator-border-enabled", true) {
        if style.border.is_empty() {
        } else if style.border == "default" {
            restore_active_border_style(tmux, window_id)?;
        } else {
            apply_active_border_style(tmux, window_id, &style.border)?;
        }
    } else {
        restore_active_border_style(tmux, window_id)?;
    }

    apply_window_title_style(tmux, window_id, &style.title_bg, &style.title_fg, false)
}

fn apply_done_visuals(tmux: &Tmux, window_id: &str, style: &StateVisualConfig) -> Result<()> {
    if config::enabled(tmux, "@agent-indicator-border-enabled", true) {
        if style.border.is_empty() {
        } else if style.border == "default" {
            restore_active_border_style(tmux, window_id)?;
        } else {
            apply_active_border_style(tmux, window_id, &style.border)?;
        }
    }

    apply_window_title_style(tmux, window_id, &style.title_bg, &style.title_fg, true)
}

fn apply_active_border_style(tmux: &Tmux, window_id: &str, border: &str) -> Result<()> {
    let keys = WindowEnvKeys::new(window_id);
    save_window_option_once(
        tmux,
        window_id,
        "pane-active-border-style",
        &keys.orig_active_border_style,
    )?;
    tmux.set_window_option_target(
        window_id,
        "pane-active-border-style",
        &format!("fg={border},bold"),
    )
}

fn restore_active_border_style(tmux: &Tmux, window_id: &str) -> Result<()> {
    let keys = WindowEnvKeys::new(window_id);
    restore_window_option(
        tmux,
        window_id,
        "pane-active-border-style",
        &keys.orig_active_border_style,
    )
}

fn apply_window_title_style(
    tmux: &Tmux,
    window_id: &str,
    bg: &str,
    fg: &str,
    mark_done: bool,
) -> Result<()> {
    if bg.is_empty() || fg.is_empty() {
        return Ok(());
    }

    let keys = WindowEnvKeys::new(window_id);

    save_window_option_once(
        tmux,
        window_id,
        "window-status-style",
        &keys.orig_status_style,
    )?;
    save_window_option_once(
        tmux,
        window_id,
        "window-status-current-style",
        &keys.orig_status_current_style,
    )?;
    tmux.set_window_option_target(
        window_id,
        "window-status-style",
        &format!("bg={bg},fg={fg}"),
    )?;
    tmux.set_window_option_target(
        window_id,
        "window-status-current-style",
        &format!("bg={bg},fg={fg}"),
    )?;

    if mark_done {
        tmux.set_env_global(&keys.done, "1")?;
    }

    Ok(())
}

fn clear_window_title_style(tmux: &Tmux, window_id: &str) -> Result<()> {
    let keys = WindowEnvKeys::new(window_id);

    restore_window_option(
        tmux,
        window_id,
        "window-status-style",
        &keys.orig_status_style,
    )?;
    restore_window_option(
        tmux,
        window_id,
        "window-status-current-style",
        &keys.orig_status_current_style,
    )?;
    tmux.unset_env_global(&keys.done)
}

fn save_window_option_once(
    tmux: &Tmux,
    window_id: &str,
    option: &str,
    env_key: &str,
) -> Result<()> {
    if tmux.show_env_global(env_key)?.is_some() {
        return Ok(());
    }

    match tmux.show_window_option_target(window_id, option)? {
        Some(value) => tmux.set_env_global(env_key, &value)?,
        None => tmux.set_env_global(env_key, "__UNSET__")?,
    }
    Ok(())
}

fn restore_window_option(tmux: &Tmux, window_id: &str, option: &str, env_key: &str) -> Result<()> {
    let Some(saved) = tmux.show_env_global(env_key)? else {
        return Ok(());
    };

    if saved == "__UNSET__" {
        tmux.unset_window_option_target(window_id, option);
    } else {
        tmux.set_window_option_target(window_id, option, &saved)?;
    }
    tmux.unset_env_global(env_key)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::should_notify_transition;
    use crate::core::AgentState;

    #[test]
    fn notifies_on_transition_into_done_or_needs_input() {
        assert!(should_notify_transition(
            Some(AgentState::Running),
            AgentState::Done
        ));
        assert!(should_notify_transition(
            Some(AgentState::Running),
            AgentState::NeedsInput
        ));
    }

    #[test]
    fn does_not_notify_when_state_is_unchanged() {
        assert!(!should_notify_transition(
            Some(AgentState::Done),
            AgentState::Done
        ));
        assert!(!should_notify_transition(
            Some(AgentState::NeedsInput),
            AgentState::NeedsInput
        ));
    }

    #[test]
    fn does_not_notify_for_running_or_off_targets() {
        assert!(!should_notify_transition(
            Some(AgentState::Done),
            AgentState::Running
        ));
        assert!(!should_notify_transition(
            Some(AgentState::NeedsInput),
            AgentState::Off
        ));
    }

    #[test]
    fn treats_none_or_off_as_transition_when_entering_done() {
        assert!(should_notify_transition(None, AgentState::Done));
        assert!(should_notify_transition(
            Some(AgentState::Off),
            AgentState::Done
        ));
    }
}
