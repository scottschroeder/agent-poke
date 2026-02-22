use std::collections::BTreeSet;

use crate::core::env_keys;

pub fn stale_pane_ids(
    live_pane_ids: &BTreeSet<String>,
    env_pairs: &[(String, String)],
) -> BTreeSet<String> {
    env_pairs
        .iter()
        .filter_map(|(key, _)| env_keys::parse_pane_id_from_key(key).map(ToOwned::to_owned))
        .filter(|pane_id| !live_pane_ids.contains(pane_id))
        .collect()
}

pub fn stale_pane_env_keys(
    stale_pane_ids: &BTreeSet<String>,
    env_pairs: &[(String, String)],
) -> Vec<String> {
    env_pairs
        .iter()
        .filter(|(key, _)| {
            env_keys::parse_pane_id_from_key(key)
                .is_some_and(|pane_id| stale_pane_ids.contains(pane_id))
        })
        .map(|(key, _)| key.clone())
        .collect()
}

pub fn stale_active_pane_binding_keys(
    live_pane_ids: &BTreeSet<String>,
    env_pairs: &[(String, String)],
) -> Vec<String> {
    env_pairs
        .iter()
        .filter(|(key, value)| {
            env_keys::parse_active_pane_agent(key).is_some() && !live_pane_ids.contains(value)
        })
        .map(|(key, _)| key.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::{stale_active_pane_binding_keys, stale_pane_env_keys, stale_pane_ids};

    #[test]
    fn finds_stale_pane_ids_from_environment() {
        let live_pane_ids = BTreeSet::from(["%1".to_string(), "%2".to_string()]);
        let env_pairs = vec![
            (
                "TMUX_AGENT_PANE_%2_STATE".to_string(),
                "running".to_string(),
            ),
            ("TMUX_AGENT_PANE_%7_STATE".to_string(), "done".to_string()),
            (
                "TMUX_AGENT_PANE_%7_LAST_EVENT_TS_MS".to_string(),
                "100".to_string(),
            ),
        ];

        assert_eq!(
            stale_pane_ids(&live_pane_ids, &env_pairs),
            BTreeSet::from(["%7".to_string()])
        );
    }

    #[test]
    fn returns_all_keys_for_stale_panes() {
        let stale_panes = BTreeSet::from(["%7".to_string()]);
        let env_pairs = vec![
            (
                "TMUX_AGENT_PANE_%2_STATE".to_string(),
                "running".to_string(),
            ),
            ("TMUX_AGENT_PANE_%7_STATE".to_string(), "done".to_string()),
            (
                "TMUX_AGENT_PANE_%7_NOTIFICATION_ID".to_string(),
                "42".to_string(),
            ),
        ];

        assert_eq!(
            stale_pane_env_keys(&stale_panes, &env_pairs),
            vec![
                "TMUX_AGENT_PANE_%7_STATE".to_string(),
                "TMUX_AGENT_PANE_%7_NOTIFICATION_ID".to_string(),
            ]
        );
    }

    #[test]
    fn detects_stale_active_agent_bindings() {
        let live_pane_ids = BTreeSet::from(["%1".to_string()]);
        let env_pairs = vec![
            (
                "TMUX_AGENT_ACTIVE_PANE_opencode".to_string(),
                "%7".to_string(),
            ),
            (
                "TMUX_AGENT_ACTIVE_PANE_claude".to_string(),
                "%1".to_string(),
            ),
        ];

        assert_eq!(
            stale_active_pane_binding_keys(&live_pane_ids, &env_pairs),
            vec!["TMUX_AGENT_ACTIVE_PANE_opencode".to_string()]
        );
    }
}
