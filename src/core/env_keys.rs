pub const PANE_PREFIX: &str = "TMUX_AGENT_PANE_";
pub const WINDOW_PREFIX: &str = "TMUX_AGENT_WINDOW_";
pub const ACTIVE_PANE_PREFIX: &str = "TMUX_AGENT_ACTIVE_PANE_";

#[derive(Debug, Clone)]
pub struct PaneEnvKeys {
    pub instance_id: String,
    pub state: String,
    pub agent: String,
    pub done: String,
    pub done_window: String,
    pub pending_reset: String,
    pub notification_id: String,
    pub last_notified_state: String,
    pub last_event_ts_ms: String,
    pub last_heartbeat_ts_ms: String,
    pub last_event_priority: String,
    pub agent_pid: String,
    pub pane_pid_at_start: String,
    pub pane_cmd_at_start: String,
    pub closed_ts_ms: String,
}

impl PaneEnvKeys {
    pub fn new(pane_id: &str) -> Self {
        Self {
            instance_id: pane_key(pane_id, "_INSTANCE_ID"),
            state: pane_key(pane_id, "_STATE"),
            agent: pane_key(pane_id, "_AGENT"),
            done: pane_key(pane_id, "_DONE"),
            done_window: pane_key(pane_id, "_DONE_WINDOW"),
            pending_reset: pane_key(pane_id, "_PENDING_RESET"),
            notification_id: pane_key(pane_id, "_NOTIFICATION_ID"),
            last_notified_state: pane_key(pane_id, "_LAST_NOTIFIED_STATE"),
            last_event_ts_ms: pane_key(pane_id, "_LAST_EVENT_TS_MS"),
            last_heartbeat_ts_ms: pane_key(pane_id, "_LAST_HEARTBEAT_TS_MS"),
            last_event_priority: pane_key(pane_id, "_LAST_EVENT_PRIORITY"),
            agent_pid: pane_key(pane_id, "_AGENT_PID"),
            pane_pid_at_start: pane_key(pane_id, "_PANE_PID_AT_START"),
            pane_cmd_at_start: pane_key(pane_id, "_PANE_CMD_AT_START"),
            closed_ts_ms: pane_key(pane_id, "_CLOSED_TS_MS"),
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

fn parse_id_with_prefix<'a>(key: &'a str, prefix: &str) -> Option<&'a str> {
    let rest = key.strip_prefix(prefix)?;
    rest.split_once('_').map(|(id, _)| id)
}

#[cfg(test)]
mod tests {
    use super::{
        PaneEnvKeys, active_pane_key, parse_active_pane_agent, parse_pane_id_from_key,
        parse_window_id_from_key,
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
