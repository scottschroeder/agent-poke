use std::collections::{HashMap, HashSet};

use anyhow::Result;
use clap::Args;

use crate::cli::commands::state_cleanup;
use crate::core::env_keys;
use crate::core::AgentState;
use crate::tmux::Tmux;

const IDLE_EMOJI: &str = "☑️";
const RUNNING_EMOJI: &str = "🔄";
const NEEDS_INPUT_EMOJI: &str = "⏸️";
const DONE_EMOJI: &str = "✅";

#[derive(Debug, Args)]
pub struct PaneStatusCommand;

impl PaneStatusCommand {
    pub fn run(self) -> Result<()> {
        let tmux = Tmux::new();
        if !tmux.is_available() {
            return Ok(());
        }

        state_cleanup::cleanup_stale_state(&tmux, false)?;
        state_cleanup::reconcile_state(&tmux)?;

        let live_pane_ids = tmux.list_all_pane_ids()?;
        let env_pairs = tmux.show_env_global_all()?;

        let counts = counts_for_live_panes(&live_pane_ids, &env_pairs);

        let mut parts = vec![format!("{IDLE_EMOJI}: {}", counts.idle)];
        if counts.running > 0 {
            parts.push(format!("{RUNNING_EMOJI}: {}", counts.running));
        }
        if counts.needs_input > 0 {
            parts.push(format!("{NEEDS_INPUT_EMOJI}: {}", counts.needs_input));
        }
        if counts.done > 0 {
            parts.push(format!("{DONE_EMOJI}: {}", counts.done));
        }
        println!("{}", parts.join(" "));
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct StatusCounts {
    idle: usize,
    running: usize,
    needs_input: usize,
    done: usize,
}

fn counts_for_live_panes(live_pane_ids: &[String], env_pairs: &[(String, String)]) -> StatusCounts {
    let live: HashSet<&str> = live_pane_ids.iter().map(String::as_str).collect();
    let mut state_by_pane: HashMap<&str, AgentState> = HashMap::new();

    for (key, value) in env_pairs {
        let Some(pane_id) = parse_state_key(key) else {
            continue;
        };

        if !live.contains(pane_id) {
            continue;
        }

        let Ok(state) = value.parse::<AgentState>() else {
            continue;
        };

        state_by_pane.insert(pane_id, state);
    }

    let mut idle = 0;
    let mut running = 0;
    let mut needs_input = 0;
    let mut done = 0;

    for state in state_by_pane.values() {
        match state {
            AgentState::Idle => idle += 1,
            AgentState::Running => running += 1,
            AgentState::NeedsInput => needs_input += 1,
            AgentState::Done => done += 1,
            AgentState::Closed => {}
        }
    }

    StatusCounts {
        idle,
        running,
        needs_input,
        done,
    }
}

fn parse_state_key(key: &str) -> Option<&str> {
    let pane_id = env_keys::parse_pane_id_from_key(key)?;
    if key.ends_with("_STATE") {
        Some(pane_id)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::{counts_for_live_panes, parse_state_key, StatusCounts};

    #[test]
    fn parses_only_pane_state_keys() {
        assert_eq!(parse_state_key("TMUX_AGENT_PANE_%7_STATE"), Some("%7"));
        assert_eq!(parse_state_key("TMUX_AGENT_PANE_%7_AGENT"), None);
        assert_eq!(parse_state_key("TMUX_AGENT_WINDOW_@1_DONE"), None);
    }

    #[test]
    fn counts_states_and_idle_from_live_panes() {
        let live_pane_ids = vec!["%1".to_string(), "%2".to_string(), "%3".to_string()];
        let env_pairs = vec![
            (
                "TMUX_AGENT_PANE_%1_STATE".to_string(),
                "running".to_string(),
            ),
            (
                "TMUX_AGENT_PANE_%2_STATE".to_string(),
                "needs-input".to_string(),
            ),
            (
                "TMUX_AGENT_PANE_%3_AGENT".to_string(),
                "opencode".to_string(),
            ),
            ("TMUX_AGENT_PANE_%9_STATE".to_string(), "done".to_string()),
        ];

        assert_eq!(
            counts_for_live_panes(&live_pane_ids, &env_pairs),
            StatusCounts {
                idle: 0,
                running: 1,
                needs_input: 1,
                done: 0,
            }
        );
    }

    #[test]
    fn ignores_untracked_or_invalid_values_for_idle_count() {
        let live_pane_ids = vec!["%1".to_string(), "%2".to_string()];
        let env_pairs = vec![
            ("TMUX_AGENT_PANE_%1_STATE".to_string(), "idle".to_string()),
            (
                "TMUX_AGENT_PANE_%2_STATE".to_string(),
                "invalid".to_string(),
            ),
        ];

        assert_eq!(
            counts_for_live_panes(&live_pane_ids, &env_pairs),
            StatusCounts {
                idle: 1,
                running: 0,
                needs_input: 0,
                done: 0,
            }
        );
    }
}
