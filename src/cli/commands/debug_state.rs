use std::collections::{BTreeMap, BTreeSet};

use anyhow::Result;
use clap::Args;

use crate::cli::commands::state_cleanup;
use crate::core::env_keys;
use crate::tmux::Tmux;

#[derive(Debug, Args)]
pub struct DebugStateCommand;

impl DebugStateCommand {
    pub fn run(self) -> Result<()> {
        let tmux = Tmux::new();
        if !tmux.is_available() {
            return Ok(());
        }

        state_cleanup::cleanup_stale_state(&tmux, false)?;
        state_cleanup::reconcile_state(&tmux)?;

        let env_pairs = tmux.show_env_global_all()?;
        let mut agent_env: BTreeMap<String, String> = BTreeMap::new();
        for (key, value) in env_pairs {
            if key.starts_with("TMUX_AGENT_") {
                agent_env.insert(key, value);
            }
        }

        println!("# agent-poke debug state");
        println!(
            "current_pane={}",
            display_current_or_dash(&tmux, "#{pane_id}")
        );
        println!(
            "current_window={}",
            display_current_or_dash(&tmux, "#{window_id}")
        );
        println!("current_session={}", display_current_or_dash(&tmux, "#S"));

        let mut pane_ids = tmux.list_all_pane_ids()?;
        pane_ids.sort();

        println!("panes_total={}", pane_ids.len());
        for pane_id in &pane_ids {
            let window_id = display_target_or_dash(&tmux, pane_id, "#{window_id}");
            let session_name = display_target_or_dash(&tmux, pane_id, "#S");
            let window_name = display_target_or_dash(&tmux, pane_id, "#W");
            let pane_active = display_target_or_dash(&tmux, pane_id, "#{pane_active}");

            let state = env_or_dash(&agent_env, &format!("TMUX_AGENT_PANE_{pane_id}_STATE"));
            let agent = env_or_dash(&agent_env, &format!("TMUX_AGENT_PANE_{pane_id}_AGENT"));
            let instance_id = env_or_dash(
                &agent_env,
                &format!("TMUX_AGENT_PANE_{pane_id}_INSTANCE_ID"),
            );
            let done = env_or_dash(&agent_env, &format!("TMUX_AGENT_PANE_{pane_id}_DONE"));
            let done_window = env_or_dash(
                &agent_env,
                &format!("TMUX_AGENT_PANE_{pane_id}_DONE_WINDOW"),
            );
            let pending_reset = env_or_dash(
                &agent_env,
                &format!("TMUX_AGENT_PANE_{pane_id}_PENDING_RESET"),
            );
            let notification_id = env_or_dash(
                &agent_env,
                &format!("TMUX_AGENT_PANE_{pane_id}_NOTIFICATION_ID"),
            );
            let last_notified_state = env_or_dash(
                &agent_env,
                &format!("TMUX_AGENT_PANE_{pane_id}_LAST_NOTIFIED_STATE"),
            );
            let last_event_ts = env_or_dash(
                &agent_env,
                &format!("TMUX_AGENT_PANE_{pane_id}_LAST_EVENT_TS_MS"),
            );
            let last_heartbeat_ts = env_or_dash(
                &agent_env,
                &format!("TMUX_AGENT_PANE_{pane_id}_LAST_HEARTBEAT_TS_MS"),
            );
            let last_event_priority = env_or_dash(
                &agent_env,
                &format!("TMUX_AGENT_PANE_{pane_id}_LAST_EVENT_PRIORITY"),
            );
            let agent_pid =
                env_or_dash(&agent_env, &format!("TMUX_AGENT_PANE_{pane_id}_AGENT_PID"));
            let pane_pid_at_start = env_or_dash(
                &agent_env,
                &format!("TMUX_AGENT_PANE_{pane_id}_PANE_PID_AT_START"),
            );
            let pane_cmd_at_start = env_or_dash(
                &agent_env,
                &format!("TMUX_AGENT_PANE_{pane_id}_PANE_CMD_AT_START"),
            );
            let closed_ts_ms = env_or_dash(
                &agent_env,
                &format!("TMUX_AGENT_PANE_{pane_id}_CLOSED_TS_MS"),
            );

            println!(
                "pane={pane_id} active={pane_active} location={session_name}:{window_name} window={window_id} state={state} agent={agent} instance_id={instance_id} done={done} done_window={done_window} pending_reset={pending_reset} notification_id={notification_id} last_notified_state={last_notified_state} last_event_ts_ms={last_event_ts} last_heartbeat_ts_ms={last_heartbeat_ts} last_event_priority={last_event_priority} agent_pid={agent_pid} pane_pid_at_start={pane_pid_at_start} pane_cmd_at_start={pane_cmd_at_start} closed_ts_ms={closed_ts_ms}"
            );
        }

        let pane_id_set: BTreeSet<String> = pane_ids.into_iter().collect();
        let mut pane_ids_with_env: BTreeSet<String> = BTreeSet::new();
        let mut window_ids_with_env: BTreeSet<String> = BTreeSet::new();
        let mut active_agent_bindings: BTreeMap<String, String> = BTreeMap::new();

        for (key, value) in &agent_env {
            if let Some(agent) = key.strip_prefix("TMUX_AGENT_ACTIVE_PANE_") {
                active_agent_bindings.insert(agent.to_string(), value.clone());
            }
            if let Some(window_id) = env_keys::parse_window_id_from_key(key) {
                window_ids_with_env.insert(window_id.to_string());
            }
            if let Some(pane_id) = env_keys::parse_pane_id_from_key(key) {
                pane_ids_with_env.insert(pane_id.to_string());
            }
        }

        println!(
            "active_agent_bindings_total={}",
            active_agent_bindings.len()
        );
        for (agent, pane_id) in active_agent_bindings {
            println!("active_agent agent={agent} pane={pane_id}");
        }

        let mut stale_panes: Vec<String> = pane_ids_with_env
            .into_iter()
            .filter(|pane_id| !pane_id_set.contains(pane_id))
            .collect();
        stale_panes.sort();
        println!("stale_pane_ids_total={}", stale_panes.len());
        for pane_id in stale_panes {
            println!("stale_pane pane={pane_id}");
        }

        println!("window_env_total={}", window_ids_with_env.len());
        for window_id in window_ids_with_env {
            let done = env_or_dash(&agent_env, &format!("TMUX_AGENT_WINDOW_{window_id}_DONE"));
            let saved_border = env_or_dash(
                &agent_env,
                &format!("TMUX_AGENT_WINDOW_{window_id}_ORIG_ACTIVE_BORDER_STYLE"),
            );
            let saved_status = env_or_dash(
                &agent_env,
                &format!("TMUX_AGENT_WINDOW_{window_id}_ORIG_STATUS_STYLE"),
            );
            let saved_status_current = env_or_dash(
                &agent_env,
                &format!("TMUX_AGENT_WINDOW_{window_id}_ORIG_STATUS_CURRENT_STYLE"),
            );

            println!(
                "window={window_id} done={done} saved_active_border={saved_border} saved_status={saved_status} saved_status_current={saved_status_current}"
            );
        }

        println!("agent_env_total={}", agent_env.len());
        for (key, value) in agent_env {
            println!("env {key}={value}");
        }

        Ok(())
    }
}

fn display_current_or_dash(tmux: &Tmux, format: &str) -> String {
    tmux.display_current(format)
        .unwrap_or_else(|_| "-".to_string())
}

fn display_target_or_dash(tmux: &Tmux, target: &str, format: &str) -> String {
    tmux.display_target(target, format)
        .unwrap_or_else(|_| "-".to_string())
}

fn env_or_dash(env: &BTreeMap<String, String>, key: &str) -> String {
    env.get(key).cloned().unwrap_or_else(|| "-".to_string())
}

#[cfg(test)]
mod tests {
    use crate::core::env_keys;

    #[test]
    fn parses_pane_id_from_agent_env_key() {
        assert_eq!(
            env_keys::parse_pane_id_from_key("TMUX_AGENT_PANE_%17_LAST_EVENT_TS_MS"),
            Some("%17")
        );
    }

    #[test]
    fn parses_window_id_from_agent_env_key() {
        assert_eq!(
            env_keys::parse_window_id_from_key("TMUX_AGENT_WINDOW_@4_ORIG_STATUS_STYLE"),
            Some("@4")
        );
    }

    #[test]
    fn ignores_unrelated_keys() {
        assert_eq!(
            env_keys::parse_pane_id_from_key("TMUX_AGENT_ACTIVE_PANE_opencode"),
            None
        );
        assert_eq!(
            env_keys::parse_window_id_from_key("TMUX_AGENT_PANE_%1_STATE"),
            None
        );
    }
}
