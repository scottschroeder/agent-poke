use std::collections::{BTreeSet, HashMap};

use anyhow::Result;

use crate::config;
use crate::core::AgentState;
use crate::core::env_keys::{PaneEnvKeys, active_pane_key};
use crate::core::reconcile::{PaneReconcileInput, ReconcileAction, reconcile_pane};
use crate::core::stale;
use crate::core::util::now_epoch_millis;
use crate::notify;
use crate::tmux::{PaneInfo, Tmux};

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

    if !stale_pane_ids.is_empty() {
        log::info!(
            "cleanup stale panes: {:?} ({} keys, {} bindings)",
            stale_pane_ids,
            stale_pane_keys.len(),
            stale_binding_keys.len(),
        );
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
    // One bulk environment dump and one bulk pane inventory replace the
    // per-pane `show-environment`/`display-message` round-trips this used to
    // issue; on a busy server that was hundreds of tmux spawns per call,
    // and reconcile runs inside every agent-poke command.
    let env_pairs = tmux.show_env_global_all()?;
    let env: HashMap<&str, &str> = env_pairs
        .iter()
        .map(|(key, value)| (key.as_str(), value.as_str()))
        .filter(|(_, value)| !value.is_empty())
        .collect();
    let pane_info = tmux.list_all_pane_info()?;
    let info_by_id: HashMap<&str, &PaneInfo> = pane_info
        .iter()
        .map(|info| (info.pane_id.as_str(), info))
        .collect();
    let now_ms = now_epoch_millis();
    let closed_retention_ms = config::closed_retention_ms(tmux);

    let mut running_timeout_cache: HashMap<&str, u64> = HashMap::new();
    let mut touched = 0usize;

    for pane_id in pane_ids_with_state(&env_pairs) {
        let keys = PaneEnvKeys::new(&pane_id);
        let Some(raw_state) = env.get(keys.state.as_str()) else {
            continue;
        };
        let Ok(state) = raw_state.parse::<AgentState>() else {
            continue;
        };

        let pane_alive = info_by_id.contains_key(pane_id.as_str());
        let process_alive = if pane_alive && state != AgentState::Closed {
            pane_process_looks_alive(tmux, info_by_id.get(pane_id.as_str()).copied(), &env, &keys)
        } else {
            true
        };
        let closed_ts_ms = env
            .get(keys.closed_ts_ms.as_str())
            .and_then(|value| value.parse::<u64>().ok());
        let heartbeat_ts_ms = env
            .get(keys.last_heartbeat_ts_ms.as_str())
            .and_then(|value| value.parse::<u64>().ok());
        let running_timeout_ms = if state == AgentState::Running {
            let agent = env.get(keys.agent.as_str()).copied().unwrap_or("");
            *running_timeout_cache
                .entry(agent)
                .or_insert_with(|| config::running_heartbeat_timeout_ms_for_agent(tmux, agent))
        } else {
            0
        };

        let action = reconcile_pane(&PaneReconcileInput {
            pane_alive,
            state,
            process_alive,
            closed_ts_ms,
            heartbeat_ts_ms,
            now_ms,
            closed_retention_ms,
            running_timeout_ms,
        });

        match action {
            ReconcileAction::None => {}
            ReconcileAction::SetClosed => {
                let reason = if !pane_alive {
                    "pane gone"
                } else {
                    "process dead"
                };
                log::info!(
                    "reconcile pane={} {} -> closed ({})",
                    pane_id,
                    state,
                    reason,
                );
                set_closed_state(tmux, &pane_id, now_ms)?;
                touched += 1;
            }
            ReconcileAction::ClearState => {
                log::info!(
                    "reconcile pane={} closed -> cleared (retention expired)",
                    pane_id,
                );
                clear_pane_state(tmux, &pane_id)?;
                touched += 1;
            }
            ReconcileAction::SetIdle => {
                let agent = env.get(keys.agent.as_str()).copied().unwrap_or_default();
                log::info!(
                    "reconcile pane={} agent={} running -> idle (heartbeat timeout {}ms)",
                    pane_id,
                    agent,
                    now_ms.saturating_sub(heartbeat_ts_ms.unwrap_or(now_ms)),
                );
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

fn pane_process_looks_alive(
    tmux: &Tmux,
    info: Option<&PaneInfo>,
    env: &HashMap<&str, &str>,
    keys: &PaneEnvKeys,
) -> bool {
    let current_pane_pid = info.and_then(|info| info.pid);
    let current_cmd = info.and_then(|info| info.command.as_deref());
    let tracked_agent_pid = env
        .get(keys.agent_pid.as_str())
        .and_then(|value| value.parse::<u32>().ok());
    let tracked_pane_pid = env
        .get(keys.pane_pid_at_start.as_str())
        .and_then(|value| value.parse::<u32>().ok());
    let tracked_cmd = env.get(keys.pane_cmd_at_start.as_str()).copied();

    if let Some(agent_pid) = tracked_agent_pid
        && !tmux.pid_is_alive(agent_pid)
    {
        return false;
    }

    if let (Some(tracked), Some(current)) = (tracked_pane_pid, current_pane_pid)
        && tracked != current
    {
        return false;
    }

    if let (Some(tracked), Some(current)) = (tracked_cmd, current_cmd)
        && tracked != current
    {
        return false;
    }

    if tracked_agent_pid.is_none() && tracked_pane_pid.is_none() {
        return current_pane_pid.is_some();
    }

    true
}

pub(crate) fn set_closed_state(tmux: &Tmux, pane_id: &str, now_ms: u64) -> Result<()> {
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

/// Creates a `Tmux` handle, checks availability, and runs stale cleanup + reconciliation.
/// Returns `Ok(None)` if tmux is not available (the caller should exit gracefully).
pub fn prepare_with_maintenance(clear_notifications: bool) -> Result<Option<Tmux>> {
    let tmux = Tmux::new();
    if !tmux.is_available() {
        return Ok(None);
    }
    cleanup_stale_state(&tmux, clear_notifications)?;
    reconcile_state(&tmux)?;
    Ok(Some(tmux))
}
