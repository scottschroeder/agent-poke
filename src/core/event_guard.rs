use std::time::Duration;

use anyhow::Result;

use crate::core::AgentState;
use crate::core::env_keys::PaneEnvKeys;
use crate::tmux::Tmux;

pub const DEFAULT_LOCK_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventApplyOutcome {
    Applied,
    IgnoredStale,
    LockTimeout,
}

pub struct EventGuard<'a> {
    tmux: &'a Tmux,
    pane_id: String,
    lock_timeout: Duration,
}

impl<'a> EventGuard<'a> {
    pub fn new(tmux: &'a Tmux, pane_id: impl Into<String>, lock_timeout: Duration) -> Self {
        Self {
            tmux,
            pane_id: pane_id.into(),
            lock_timeout,
        }
    }

    pub fn apply<F>(
        &self,
        state: AgentState,
        event_ts_ms: u64,
        apply_state: F,
    ) -> Result<EventApplyOutcome>
    where
        F: FnOnce() -> Result<()>,
    {
        let lock_key = format!("agent-poke:pane:{}", self.pane_id);
        let Some(_lock) = self.tmux.try_lock_channel(&lock_key, self.lock_timeout)? else {
            self.tmux.unlock_channel(&lock_key);
            let Some(_lock) = self
                .tmux
                .try_lock_channel(&lock_key, Duration::from_millis(250))?
            else {
                return Ok(EventApplyOutcome::LockTimeout);
            };

            return self.apply_with_lock(state, event_ts_ms, apply_state);
        };

        self.apply_with_lock(state, event_ts_ms, apply_state)
    }

    fn apply_with_lock<F>(
        &self,
        state: AgentState,
        event_ts_ms: u64,
        apply_state: F,
    ) -> Result<EventApplyOutcome>
    where
        F: FnOnce() -> Result<()>,
    {
        let pane_keys = PaneEnvKeys::new(&self.pane_id);
        let previous_ts = self
            .tmux
            .show_env_global(&pane_keys.last_event_ts_ms)?
            .and_then(|value| value.parse::<u64>().ok());
        let previous_priority = self
            .tmux
            .show_env_global(&pane_keys.last_event_priority)?
            .and_then(|value| value.parse::<u8>().ok())
            .unwrap_or(0);
        let event_priority = state_priority(state);

        if !should_apply_update(previous_ts, previous_priority, event_ts_ms, event_priority) {
            return Ok(EventApplyOutcome::IgnoredStale);
        }

        apply_state()?;

        self.tmux
            .set_env_global(&pane_keys.last_event_ts_ms, &event_ts_ms.to_string())?;
        self.tmux
            .set_env_global(&pane_keys.last_event_priority, &event_priority.to_string())?;

        Ok(EventApplyOutcome::Applied)
    }
}

pub fn state_priority(state: AgentState) -> u8 {
    match state {
        AgentState::Idle => 0,
        AgentState::Running => 1,
        AgentState::Done => 2,
        AgentState::NeedsInput => 3,
        AgentState::Closed => 4,
    }
}

fn should_apply_update(
    previous_ts: Option<u64>,
    previous_priority: u8,
    event_ts_ms: u64,
    event_priority: u8,
) -> bool {
    match previous_ts {
        None => true,
        Some(prev_ts) if event_ts_ms > prev_ts => true,
        Some(prev_ts) if event_ts_ms < prev_ts => false,
        Some(_) => event_priority >= previous_priority,
    }
}

#[cfg(test)]
mod tests {
    use super::{should_apply_update, state_priority};
    use crate::core::AgentState;

    #[test]
    fn accepts_newer_timestamps() {
        assert!(should_apply_update(Some(100), 1, 101, 0));
    }

    #[test]
    fn rejects_older_timestamps() {
        assert!(!should_apply_update(Some(100), 0, 99, 2));
    }

    #[test]
    fn resolves_same_timestamp_with_priority() {
        assert!(should_apply_update(Some(100), 0, 100, 1));
        assert!(!should_apply_update(Some(100), 2, 100, 1));
        assert!(should_apply_update(Some(100), 1, 100, 1));
    }

    #[test]
    fn ranks_needs_input_and_done_over_running() {
        assert!(state_priority(AgentState::Done) > state_priority(AgentState::Running));
        assert!(state_priority(AgentState::NeedsInput) > state_priority(AgentState::Done));
    }
}
