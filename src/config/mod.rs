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
    tmux.show_option_global(option)
        .ok()
        .flatten()
        .unwrap_or_else(|| default.to_string())
}

pub fn enabled(tmux: &Tmux, option: &str, default: bool) -> bool {
    let raw_default = if default { "on" } else { "off" };
    let value = option_or_default(tmux, option, raw_default);
    parse_bool(&value)
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
    state_in_list(state, &states)
}

pub fn notification_persistent(tmux: &Tmux) -> bool {
    enabled(tmux, "@agent-indicator-notification-persistent", false)
}

pub fn notification_urgency(tmux: &Tmux) -> String {
    option_or_default(tmux, "@agent-indicator-notification-urgency", "critical")
}

pub fn notification_sound_enabled(tmux: &Tmux) -> bool {
    enabled(tmux, "@agent-indicator-notification-sound", true)
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
    let raw = option_or_default(
        tmux,
        "@agent-indicator-running-heartbeat-timeout-ms",
        "60000",
    );
    parse_positive_u64(&raw, 60000)
}

const DEFAULT_HEARTBEAT_TIMEOUT_MS: u64 = 60_000;

fn builtin_agent_heartbeat_timeout_ms(agent: &str) -> u64 {
    match agent {
        "claude" => 150_000,
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
    let raw = option_or_default(tmux, "@agent-indicator-closed-retention-ms", "900000");
    parse_positive_u64(&raw, 900000)
}

fn parse_bool(value: &str) -> bool {
    matches!(value, "on" | "true" | "yes" | "1")
}

fn parse_positive_u64(value: &str, default: u64) -> u64 {
    value
        .parse::<u64>()
        .ok()
        .filter(|v| *v > 0)
        .unwrap_or(default)
}

fn state_in_list(state: AgentState, list: &str) -> bool {
    list.split(',')
        .map(str::trim)
        .any(|configured| configured == state.as_str())
}

#[cfg(test)]
mod tests {
    use super::{builtin_agent_heartbeat_timeout_ms, parse_bool, parse_positive_u64, state_in_list};
    use crate::core::AgentState;

    #[test]
    fn parse_bool_truthy_values() {
        assert!(parse_bool("on"));
        assert!(parse_bool("true"));
        assert!(parse_bool("yes"));
        assert!(parse_bool("1"));
    }

    #[test]
    fn parse_bool_falsy_values() {
        assert!(!parse_bool("off"));
        assert!(!parse_bool("false"));
        assert!(!parse_bool("no"));
        assert!(!parse_bool("0"));
        assert!(!parse_bool(""));
        assert!(!parse_bool("random"));
    }

    #[test]
    fn parse_positive_u64_valid() {
        assert_eq!(parse_positive_u64("60000", 999), 60000);
        assert_eq!(parse_positive_u64("1", 999), 1);
    }

    #[test]
    fn parse_positive_u64_zero_returns_default() {
        assert_eq!(parse_positive_u64("0", 42), 42);
    }

    #[test]
    fn parse_positive_u64_invalid_returns_default() {
        assert_eq!(parse_positive_u64("abc", 42), 42);
        assert_eq!(parse_positive_u64("", 42), 42);
        assert_eq!(parse_positive_u64("-1", 42), 42);
    }

    #[test]
    fn state_in_list_finds_present_state() {
        assert!(state_in_list(AgentState::Done, "needs-input,done"));
        assert!(state_in_list(AgentState::NeedsInput, "needs-input,done"));
    }

    #[test]
    fn state_in_list_rejects_absent_state() {
        assert!(!state_in_list(AgentState::Running, "needs-input,done"));
        assert!(!state_in_list(AgentState::Idle, "needs-input,done"));
    }

    #[test]
    fn state_in_list_handles_whitespace() {
        assert!(state_in_list(AgentState::Done, "needs-input , done"));
        assert!(state_in_list(AgentState::NeedsInput, " needs-input , done "));
    }

    #[test]
    fn state_in_list_single_value() {
        assert!(state_in_list(AgentState::Done, "done"));
        assert!(!state_in_list(AgentState::Running, "done"));
    }

    #[test]
    fn builtin_claude_timeout_is_longer() {
        assert_eq!(builtin_agent_heartbeat_timeout_ms("claude"), 150_000);
        assert_eq!(builtin_agent_heartbeat_timeout_ms("opencode"), 60_000);
        assert_eq!(builtin_agent_heartbeat_timeout_ms("unknown"), 60_000);
    }
}
