pub const PANE_PREFIX: &str = "TMUX_AGENT_PANE_";
pub const WINDOW_PREFIX: &str = "TMUX_AGENT_WINDOW_";
pub const ACTIVE_PANE_PREFIX: &str = "TMUX_AGENT_ACTIVE_PANE_";

#[derive(Debug, Clone)]
pub struct PaneEnvKeys {
    pub state: String,
    pub agent: String,
    pub done: String,
    pub done_window: String,
    pub pending_reset: String,
    pub notification_id: String,
    pub last_event_ts_ms: String,
    pub last_event_priority: String,
}

impl PaneEnvKeys {
    pub fn new(pane_id: &str) -> Self {
        Self {
            state: pane_key(pane_id, "_STATE"),
            agent: pane_key(pane_id, "_AGENT"),
            done: pane_key(pane_id, "_DONE"),
            done_window: pane_key(pane_id, "_DONE_WINDOW"),
            pending_reset: pane_key(pane_id, "_PENDING_RESET"),
            notification_id: pane_key(pane_id, "_NOTIFICATION_ID"),
            last_event_ts_ms: pane_key(pane_id, "_LAST_EVENT_TS_MS"),
            last_event_priority: pane_key(pane_id, "_LAST_EVENT_PRIORITY"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct WindowEnvKeys {
    pub done: String,
    pub orig_active_border_style: String,
    pub orig_status_style: String,
    pub orig_status_current_style: String,
}

impl WindowEnvKeys {
    pub fn new(window_id: &str) -> Self {
        Self {
            done: window_key(window_id, "_DONE"),
            orig_active_border_style: window_key(window_id, "_ORIG_ACTIVE_BORDER_STYLE"),
            orig_status_style: window_key(window_id, "_ORIG_STATUS_STYLE"),
            orig_status_current_style: window_key(window_id, "_ORIG_STATUS_CURRENT_STYLE"),
        }
    }
}

pub fn active_pane_key(agent: &str) -> String {
    format!("{ACTIVE_PANE_PREFIX}{agent}")
}

pub fn parse_pane_id_from_key(key: &str) -> Option<&str> {
    parse_id_with_prefix(key, PANE_PREFIX)
}

pub fn parse_window_id_from_key(key: &str) -> Option<&str> {
    parse_id_with_prefix(key, WINDOW_PREFIX)
}

pub fn parse_active_pane_agent(key: &str) -> Option<&str> {
    key.strip_prefix(ACTIVE_PANE_PREFIX)
}

fn pane_key(pane_id: &str, suffix: &str) -> String {
    format!("{PANE_PREFIX}{pane_id}{suffix}")
}

fn window_key(window_id: &str, suffix: &str) -> String {
    format!("{WINDOW_PREFIX}{window_id}{suffix}")
}

fn parse_id_with_prefix<'a>(key: &'a str, prefix: &str) -> Option<&'a str> {
    let rest = key.strip_prefix(prefix)?;
    rest.split_once('_').map(|(id, _)| id)
}

#[cfg(test)]
mod tests {
    use super::{
        PaneEnvKeys, WindowEnvKeys, active_pane_key, parse_active_pane_agent,
        parse_pane_id_from_key, parse_window_id_from_key,
    };

    #[test]
    fn builds_pane_keys_consistently() {
        let keys = PaneEnvKeys::new("%13");
        assert_eq!(keys.state, "TMUX_AGENT_PANE_%13_STATE");
        assert_eq!(
            keys.last_event_priority,
            "TMUX_AGENT_PANE_%13_LAST_EVENT_PRIORITY"
        );
    }

    #[test]
    fn builds_window_keys_consistently() {
        let keys = WindowEnvKeys::new("@4");
        assert_eq!(keys.done, "TMUX_AGENT_WINDOW_@4_DONE");
        assert_eq!(
            keys.orig_status_current_style,
            "TMUX_AGENT_WINDOW_@4_ORIG_STATUS_CURRENT_STYLE"
        );
    }

    #[test]
    fn parses_pane_and_window_ids_from_env_keys() {
        assert_eq!(
            parse_pane_id_from_key("TMUX_AGENT_PANE_%17_LAST_EVENT_TS_MS"),
            Some("%17")
        );
        assert_eq!(
            parse_window_id_from_key("TMUX_AGENT_WINDOW_@4_ORIG_STATUS_STYLE"),
            Some("@4")
        );
    }

    #[test]
    fn parses_active_agent_binding_key() {
        let key = active_pane_key("opencode");
        assert_eq!(key, "TMUX_AGENT_ACTIVE_PANE_opencode");
        assert_eq!(parse_active_pane_agent(&key), Some("opencode"));
    }
}
