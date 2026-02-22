use std::collections::BTreeSet;

use anyhow::Result;

use crate::config;
use crate::core::AgentState;
use crate::core::env_keys::{PaneEnvKeys, active_pane_key};
use crate::core::stale;
use crate::notify;
use crate::tmux::Tmux;

pub fn cleanup_stale_state(tmux: &Tmux, clear_notifications: bool) -> Result<usize> {
    let live_pane_ids: BTreeSet<String> = tmux.list_all_pane_ids()?.into_iter().collect();
    let env_pairs = tmux.show_env_global_all()?;
    let stale_pane_ids = stale::stale_pane_ids(&live_pane_ids, &env_pairs);
    let stale_pane_keys = stale::stale_pane_env_keys(&stale_pane_ids, &env_pairs);
    let stale_binding_keys = stale::stale_active_pane_binding_keys(&live_pane_ids, &env_pairs);

    if clear_notifications {
        let stale_key_set: BTreeSet<&str> = stale_pane_keys.iter().map(String::as_str).collect();
        for (key, value) in &env_pairs {
            if key.ends_with("_NOTIFICATION_ID")
                && stale_key_set.contains(key.as_str())
                && !value.is_empty()
            {
                notify::clear_system_notification(value);
            }
        }
    }

    for key in &stale_pane_keys {
        tmux.unset_env_global(key)?;
    }
    for key in &stale_binding_keys {
        tmux.unset_env_global(key)?;
    }

    Ok(stale_pane_keys.len() + stale_binding_keys.len())
}

pub fn reconcile_state(tmux: &Tmux) -> Result<usize> {
    let env_pairs = tmux.show_env_global_all()?;
    let live_pane_ids: BTreeSet<String> = tmux.list_all_pane_ids()?.into_iter().collect();
    let now_ms = now_epoch_millis();
    let closed_retention_ms = config::closed_retention_ms(tmux);

    let mut touched = 0usize;

    for pane_id in pane_ids_with_state(&env_pairs) {
        let keys = PaneEnvKeys::new(&pane_id);
        let Some(raw_state) = tmux.show_env_global(&keys.state)? else {
            continue;
        };
        let Ok(state) = raw_state.parse::<AgentState>() else {
            continue;
        };

        if !live_pane_ids.contains(&pane_id) {
            set_closed_state(tmux, &pane_id, now_ms)?;
            touched += 1;
            continue;
        }

        if state == AgentState::Closed {
            let closed_ts = tmux
                .show_env_global(&keys.closed_ts_ms)?
                .and_then(|value| value.parse::<u64>().ok())
                .unwrap_or(now_ms);
            if now_ms.saturating_sub(closed_ts) >= closed_retention_ms {
                clear_pane_state(tmux, &pane_id)?;
                touched += 1;
            }
            continue;
        }

        if !pane_process_looks_alive(tmux, &pane_id, &keys)? {
            set_closed_state(tmux, &pane_id, now_ms)?;
            touched += 1;
            continue;
        }

        if state == AgentState::Running {
            let agent = tmux
                .show_env_global(&keys.agent)?
                .unwrap_or_default();
            let running_timeout_ms =
                config::running_heartbeat_timeout_ms_for_agent(tmux, &agent);
            let heartbeat_ts = tmux
                .show_env_global(&keys.last_heartbeat_ts_ms)?
                .and_then(|value| value.parse::<u64>().ok())
                .unwrap_or(now_ms);
            if now_ms.saturating_sub(heartbeat_ts) >= running_timeout_ms {
                tmux.set_env_global(&keys.state, AgentState::Idle.as_str())?;
                touched += 1;
            }
        }
    }

    Ok(touched)
}

fn pane_ids_with_state(env_pairs: &[(String, String)]) -> BTreeSet<String> {
    env_pairs
        .iter()
        .filter_map(|(key, _)| {
            if key.ends_with("_STATE") {
                crate::core::env_keys::parse_pane_id_from_key(key).map(ToOwned::to_owned)
            } else {
                None
            }
        })
        .collect()
}

fn pane_process_looks_alive(tmux: &Tmux, pane_id: &str, keys: &PaneEnvKeys) -> Result<bool> {
    let current_pane_pid = tmux.pane_pid(pane_id)?;
    let current_cmd = tmux.pane_current_command(pane_id)?;
    let tracked_agent_pid = tmux
        .show_env_global(&keys.agent_pid)?
        .and_then(|value| value.parse::<u32>().ok());
    let tracked_pane_pid = tmux
        .show_env_global(&keys.pane_pid_at_start)?
        .and_then(|value| value.parse::<u32>().ok());
    let tracked_cmd = tmux.show_env_global(&keys.pane_cmd_at_start)?;

    if let Some(agent_pid) = tracked_agent_pid
        && !tmux.pid_is_alive(agent_pid)
    {
        return Ok(false);
    }

    if let (Some(tracked), Some(current)) = (tracked_pane_pid, current_pane_pid)
        && tracked != current
    {
        return Ok(false);
    }

    if let (Some(tracked), Some(current)) = (tracked_cmd.as_deref(), current_cmd.as_deref())
        && tracked != current
    {
        return Ok(false);
    }

    if tracked_agent_pid.is_none() && tracked_pane_pid.is_none() {
        return Ok(current_pane_pid.is_some());
    }

    Ok(true)
}

fn set_closed_state(tmux: &Tmux, pane_id: &str, now_ms: u64) -> Result<()> {
    let keys = PaneEnvKeys::new(pane_id);
    if let Some(agent) = tmux.show_env_global(&keys.agent)? {
        tmux.unset_env_global(&active_pane_key(&agent))?;
    }
    if let Some(notification_id) = tmux.show_env_global(&keys.notification_id)? {
        notify::clear_system_notification(&notification_id);
    }

    tmux.set_env_global(&keys.state, AgentState::Closed.as_str())?;
    tmux.set_env_global(&keys.closed_ts_ms, &now_ms.to_string())?;
    tmux.unset_env_global(&keys.done)?;
    tmux.unset_env_global(&keys.done_window)?;
    tmux.unset_env_global(&keys.pending_reset)?;
    tmux.unset_env_global(&keys.notification_id)?;
    tmux.unset_env_global(&keys.last_notified_state)?;
    Ok(())
}

fn clear_pane_state(tmux: &Tmux, pane_id: &str) -> Result<()> {
    let keys = PaneEnvKeys::new(pane_id);
    if let Some(agent) = tmux.show_env_global(&keys.agent)? {
        tmux.unset_env_global(&active_pane_key(&agent))?;
    }
    tmux.unset_env_global(&keys.instance_id)?;
    tmux.unset_env_global(&keys.state)?;
    tmux.unset_env_global(&keys.agent)?;
    tmux.unset_env_global(&keys.done)?;
    tmux.unset_env_global(&keys.done_window)?;
    tmux.unset_env_global(&keys.pending_reset)?;
    tmux.unset_env_global(&keys.notification_id)?;
    tmux.unset_env_global(&keys.last_notified_state)?;
    tmux.unset_env_global(&keys.last_event_ts_ms)?;
    tmux.unset_env_global(&keys.last_heartbeat_ts_ms)?;
    tmux.unset_env_global(&keys.last_event_priority)?;
    tmux.unset_env_global(&keys.agent_pid)?;
    tmux.unset_env_global(&keys.pane_pid_at_start)?;
    tmux.unset_env_global(&keys.pane_cmd_at_start)?;
    tmux.unset_env_global(&keys.closed_ts_ms)?;
    Ok(())
}

fn now_epoch_millis() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};

    let Ok(duration) = SystemTime::now().duration_since(UNIX_EPOCH) else {
        return 0;
    };
    duration.as_millis() as u64
}
