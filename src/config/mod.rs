use crate::core::AgentState;
use crate::tmux::Tmux;

pub const DEFAULT_NOTIFICATION_FORMAT: &str =
    "[#{agent_name}] #{agent_state} (#{session_name}:#{window_name})";

pub fn option_or_default(tmux: &Tmux, option: &str, default: &str) -> String {
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

pub fn icon_map(tmux: &Tmux) -> String {
    option_or_default(
        tmux,
        "@agent-indicator-icons",
        "claude=🤖,codex=🧠,default=🤖",
    )
}

pub fn icon_for_agent(agent: &str, map: &str) -> String {
    let mut default_icon = "🤖".to_string();

    for pair in map.split(',') {
        let mut parts = pair.splitn(2, '=');
        let key = parts.next().unwrap_or_default().trim();
        let value = parts.next().unwrap_or_default().trim();

        if key == "default" && !value.is_empty() {
            default_icon = value.to_string();
        }

        if key == agent && !value.is_empty() {
            return value.to_string();
        }
    }

    default_icon
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

pub fn notification_format(tmux: &Tmux) -> String {
    option_or_default(
        tmux,
        "@agent-indicator-notification-format",
        DEFAULT_NOTIFICATION_FORMAT,
    )
}
