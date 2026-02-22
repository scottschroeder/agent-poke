use crate::core::AgentState;
use crate::tmux::Tmux;

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
        running: option_or_default(tmux, "@agent-indicator-window-status-running-glyph", "🔄"),
        needs_input: option_or_default(
            tmux,
            "@agent-indicator-window-status-needs-input-glyph",
            "⏸️",
        ),
        done: option_or_default(tmux, "@agent-indicator-window-status-done-glyph", "✅"),
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

pub fn closed_retention_ms(tmux: &Tmux) -> u64 {
    option_or_default(tmux, "@agent-indicator-closed-retention-ms", "900000")
        .parse::<u64>()
        .ok()
        .filter(|value| *value > 0)
        .unwrap_or(900000)
}
