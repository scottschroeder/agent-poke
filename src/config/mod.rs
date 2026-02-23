use crate::core::AgentState;
use crate::tmux::Tmux;

pub const DEFAULT_IDLE_GLYPH: &str = "💤";
pub const DEFAULT_RUNNING_GLYPH: &str = "🔄";
pub const DEFAULT_NEEDS_INPUT_GLYPH: &str = "🔔";
pub const DEFAULT_DONE_GLYPH: &str = "✅";

#[derive(Debug, Clone)]
pub struct WindowStatusGlyphConfig {
    pub running: String,
    pub needs_input: String,
    pub done: String,
}

pub fn option_or_default(tmux: &Tmux, option: &str, default: &str) -> String {
    if tmux.option_is_set_global(option) {
        return tmux
            .show_option_global(option)
            .ok()
            .flatten()
            .unwrap_or_default();
    }

    tmux.show_option_global(option)
        .ok()
        .flatten()
        .unwrap_or_else(|| default.to_string())
}

pub fn enabled(tmux: &Tmux, option: &str, default: bool) -> bool {
    let raw_default = if default { "on" } else { "off" };
    let value = option_or_default(tmux, option, raw_default);
    matches!(value.as_str(), "on" | "true" | "yes" | "1")
}

pub fn window_status_glyphs(tmux: &Tmux) -> WindowStatusGlyphConfig {
    WindowStatusGlyphConfig {
        running: option_or_default(tmux, "@agent-indicator-window-status-running-glyph", DEFAULT_RUNNING_GLYPH),
        needs_input: option_or_default(
            tmux,
            "@agent-indicator-window-status-needs-input-glyph",
            DEFAULT_NEEDS_INPUT_GLYPH,
        ),
        done: option_or_default(tmux, "@agent-indicator-window-status-done-glyph", DEFAULT_DONE_GLYPH),
    }
}

pub fn state_notification_enabled(tmux: &Tmux, state: AgentState) -> bool {
    if !enabled(tmux, "@agent-indicator-notification-enabled", true) {
        return false;
    }

    let states = option_or_default(
        tmux,
        "@agent-indicator-notification-states",
        "needs-input,done",
    );
    states
        .split(',')
        .map(str::trim)
        .any(|configured| configured == state.as_str())
}

pub fn notification_persistent(tmux: &Tmux) -> bool {
    enabled(tmux, "@agent-indicator-notification-persistent", false)
}

pub fn notification_urgency(tmux: &Tmux) -> String {
    option_or_default(tmux, "@agent-indicator-notification-urgency", "critical")
}

pub fn notification_sound_enabled(tmux: &Tmux) -> bool {
    enabled(tmux, "@agent-indicator-notification-sound", false)
}

pub fn notification_sound_name(tmux: &Tmux) -> Option<String> {
    let value = option_or_default(tmux, "@agent-indicator-notification-sound-name", "");
    if value.is_empty() { None } else { Some(value) }
}

pub fn state_enabled(tmux: &Tmux, state: AgentState) -> bool {
    enabled(
        tmux,
        &format!("@agent-indicator-{}-enabled", state.as_str()),
        true,
    )
}

pub fn running_heartbeat_timeout_ms(tmux: &Tmux) -> u64 {
    option_or_default(
        tmux,
        "@agent-indicator-running-heartbeat-timeout-ms",
        "30000",
    )
    .parse::<u64>()
    .ok()
    .filter(|value| *value > 0)
    .unwrap_or(30000)
}

const DEFAULT_HEARTBEAT_TIMEOUT_MS: u64 = 30_000;

fn builtin_agent_heartbeat_timeout_ms(agent: &str) -> u64 {
    match agent {
        "claude" => 300_000,
        _ => DEFAULT_HEARTBEAT_TIMEOUT_MS,
    }
}

pub fn running_heartbeat_timeout_ms_for_agent(tmux: &Tmux, agent: &str) -> u64 {
    // 1. Agent-specific tmux override
    let agent_key = format!("@agent-indicator-running-heartbeat-timeout-ms-{agent}");
    if tmux.option_is_set_global(&agent_key)
        && let Some(value) = tmux
            .show_option_global(&agent_key)
            .ok()
            .flatten()
            .and_then(|v| v.parse::<u64>().ok())
            .filter(|v| *v > 0)
    {
        return value;
    }
    // 2. Global tmux override
    if tmux.option_is_set_global("@agent-indicator-running-heartbeat-timeout-ms") {
        return running_heartbeat_timeout_ms(tmux);
    }
    // 3. Built-in per-agent default
    builtin_agent_heartbeat_timeout_ms(agent)
}

pub fn closed_retention_ms(tmux: &Tmux) -> u64 {
    option_or_default(tmux, "@agent-indicator-closed-retention-ms", "900000")
        .parse::<u64>()
        .ok()
        .filter(|value| *value > 0)
        .unwrap_or(900000)
}
