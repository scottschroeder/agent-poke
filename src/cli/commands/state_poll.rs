use std::collections::{BTreeSet, HashMap};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::Result;
use clap::Args;

use crate::cli::commands::state_cleanup;
use crate::core::AgentState;
use crate::core::env_keys;
use crate::tmux::Tmux;

#[derive(Debug, Args)]
pub struct StatePollCommand {
    #[arg(long, default_value_t = 500)]
    interval_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PaneStateSnapshot {
    state: AgentState,
    agent: Option<String>,
}

impl StatePollCommand {
    pub fn run(self) -> Result<()> {
        let tmux = Tmux::new();
        if !tmux.is_available() {
            return Ok(());
        }

        let interval = Duration::from_millis(self.interval_ms);
        let mut previous = collect_pane_states(&tmux)?;

        loop {
            let started = Instant::now();

            if let Err(err) = state_cleanup::cleanup_stale_state(&tmux, false) {
                log::warn!("state-poll stale cleanup failed: {err}");
            }
            if let Err(err) = state_cleanup::reconcile_state(&tmux) {
                log::warn!("state-poll reconcile failed: {err}");
            }

            let current = match collect_pane_states(&tmux) {
                Ok(current) => current,
                Err(err) => {
                    log::warn!("state-poll failed: {err}");
                    sleep_until_next_tick(interval, started);
                    continue;
                }
            };

            log_transitions(&tmux, &previous, &current);
            previous = current;
            sleep_until_next_tick(interval, started);
        }
    }
}

fn collect_pane_states(tmux: &Tmux) -> Result<HashMap<String, PaneStateSnapshot>> {
    let pane_ids = tmux.list_all_pane_ids()?;
    let env_pairs = tmux.show_env_global_all()?;

    let mut state_by_pane: HashMap<String, AgentState> = HashMap::new();
    let mut agent_by_pane: HashMap<String, String> = HashMap::new();

    for (key, value) in env_pairs {
        if let Some(pane_id) = parse_pane_key(&key, "_STATE")
            && let Ok(state) = value.parse::<AgentState>()
        {
            state_by_pane.insert(pane_id.to_string(), state);
            continue;
        }

        if let Some(pane_id) = parse_pane_key(&key, "_AGENT") {
            agent_by_pane.insert(pane_id.to_string(), value);
        }
    }

    let mut snapshots = HashMap::new();
    for pane_id in pane_ids {
        snapshots.insert(
            pane_id.clone(),
            PaneStateSnapshot {
                state: state_by_pane.remove(&pane_id).unwrap_or(AgentState::Idle),
                agent: agent_by_pane.remove(&pane_id),
            },
        );
    }

    Ok(snapshots)
}

fn parse_pane_key<'a>(key: &'a str, suffix: &str) -> Option<&'a str> {
    let pane_id = env_keys::parse_pane_id_from_key(key)?;
    if key.ends_with(suffix) {
        Some(pane_id)
    } else {
        None
    }
}

fn log_transitions(
    tmux: &Tmux,
    previous: &HashMap<String, PaneStateSnapshot>,
    current: &HashMap<String, PaneStateSnapshot>,
) {
    let mut pane_ids: BTreeSet<&str> = BTreeSet::new();
    pane_ids.extend(previous.keys().map(String::as_str));
    pane_ids.extend(current.keys().map(String::as_str));

    for pane_id in pane_ids {
        let from = previous.get(pane_id);
        let to = current.get(pane_id);

        match (from, to) {
            (Some(from), Some(to)) if from != to => {
                let now = now_epoch_seconds();
                let location = pane_location(tmux, pane_id);
                println!(
                    "[{now:.3}] pane={pane_id} location {} -> {} state {} -> {} agent {} -> {}",
                    location,
                    location,
                    from.state,
                    to.state,
                    display_agent(from.agent.as_deref()),
                    display_agent(to.agent.as_deref())
                );
            }
            (Some(from), None) => {
                let now = now_epoch_seconds();
                let location = pane_location(tmux, pane_id);
                println!(
                    "[{now:.3}] pane={pane_id} location {} state {} -> closed agent {} -> - (pane removed)",
                    location,
                    from.state,
                    display_agent(from.agent.as_deref())
                );
            }
            _ => {}
        }
    }
}

fn pane_location(tmux: &Tmux, pane_id: &str) -> String {
    let session_name = tmux
        .display_target(pane_id, "#S")
        .unwrap_or_else(|_| "-".to_string());
    let window_name = tmux
        .display_target(pane_id, "#W")
        .unwrap_or_else(|_| "-".to_string());
    display_location(&session_name, &window_name)
}

fn sleep_until_next_tick(interval: Duration, started: Instant) {
    let elapsed = started.elapsed();
    if elapsed < interval {
        thread::sleep(interval - elapsed);
    }
}

fn now_epoch_seconds() -> f64 {
    let Ok(duration) = SystemTime::now().duration_since(UNIX_EPOCH) else {
        return 0.0;
    };
    duration.as_secs_f64()
}

fn display_agent(agent: Option<&str>) -> &str {
    match agent {
        Some(agent) if !agent.is_empty() => agent,
        _ => "-",
    }
}

fn display_location(session_name: &str, window_name: &str) -> String {
    format!("{session_name}:{window_name}")
}

#[cfg(test)]
mod tests {
    use super::parse_pane_key;

    #[test]
    fn parses_pane_state_key() {
        assert_eq!(
            parse_pane_key("TMUX_AGENT_PANE_%13_STATE", "_STATE"),
            Some("%13")
        );
    }

    #[test]
    fn rejects_unrelated_key() {
        assert_eq!(parse_pane_key("TMUX_AGENT_WINDOW_@1_DONE", "_STATE"), None);
    }
}
