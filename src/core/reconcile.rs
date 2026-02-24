use crate::core::AgentState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReconcileAction {
    /// No action needed for this pane.
    None,
    /// Set this pane to Closed state.
    SetClosed,
    /// Clear all state for this pane (retention expired).
    ClearState,
    /// Set this pane to Idle (heartbeat timeout).
    SetIdle,
}

pub struct PaneReconcileInput {
    pub pane_alive: bool,
    pub state: AgentState,
    pub process_alive: bool,
    pub closed_ts_ms: Option<u64>,
    pub heartbeat_ts_ms: Option<u64>,
    pub now_ms: u64,
    pub closed_retention_ms: u64,
    pub running_timeout_ms: u64,
}

pub fn reconcile_pane(input: &PaneReconcileInput) -> ReconcileAction {
    if !input.pane_alive {
        return ReconcileAction::SetClosed;
    }

    if input.state == AgentState::Closed {
        let closed_ts = input.closed_ts_ms.unwrap_or(input.now_ms);
        if input.now_ms.saturating_sub(closed_ts) >= input.closed_retention_ms {
            return ReconcileAction::ClearState;
        }
        return ReconcileAction::None;
    }

    if !input.process_alive {
        return ReconcileAction::SetClosed;
    }

    if input.state == AgentState::Running {
        let heartbeat_ts = input.heartbeat_ts_ms.unwrap_or(input.now_ms);
        if input.now_ms.saturating_sub(heartbeat_ts) >= input.running_timeout_ms {
            return ReconcileAction::SetIdle;
        }
    }

    ReconcileAction::None
}

#[cfg(test)]
mod tests {
    use super::{PaneReconcileInput, ReconcileAction, reconcile_pane};
    use crate::core::AgentState;

    fn input(state: AgentState) -> PaneReconcileInput {
        PaneReconcileInput {
            pane_alive: true,
            state,
            process_alive: true,
            closed_ts_ms: None,
            heartbeat_ts_ms: None,
            now_ms: 1000,
            closed_retention_ms: 900_000,
            running_timeout_ms: 60_000,
        }
    }

    #[test]
    fn dead_pane_becomes_closed() {
        let mut i = input(AgentState::Running);
        i.pane_alive = false;
        assert_eq!(reconcile_pane(&i), ReconcileAction::SetClosed);
    }

    #[test]
    fn dead_pane_closed_regardless_of_state() {
        for state in [
            AgentState::Idle,
            AgentState::Running,
            AgentState::NeedsInput,
            AgentState::Done,
            AgentState::Closed,
        ] {
            let mut i = input(state);
            i.pane_alive = false;
            assert_eq!(reconcile_pane(&i), ReconcileAction::SetClosed);
        }
    }

    #[test]
    fn closed_retention_expired_clears_state() {
        let mut i = input(AgentState::Closed);
        i.closed_ts_ms = Some(100);
        i.now_ms = 1100;
        i.closed_retention_ms = 900;
        assert_eq!(reconcile_pane(&i), ReconcileAction::ClearState);
    }

    #[test]
    fn closed_retention_not_expired_keeps_state() {
        let mut i = input(AgentState::Closed);
        i.closed_ts_ms = Some(500);
        i.now_ms = 1000;
        i.closed_retention_ms = 900;
        assert_eq!(reconcile_pane(&i), ReconcileAction::None);
    }

    #[test]
    fn closed_missing_timestamp_uses_now() {
        let mut i = input(AgentState::Closed);
        i.closed_retention_ms = 900;
        assert_eq!(reconcile_pane(&i), ReconcileAction::None);
    }

    #[test]
    fn dead_process_becomes_closed() {
        let mut i = input(AgentState::Running);
        i.process_alive = false;
        assert_eq!(reconcile_pane(&i), ReconcileAction::SetClosed);
    }

    #[test]
    fn running_heartbeat_timeout_becomes_idle() {
        let mut i = input(AgentState::Running);
        i.heartbeat_ts_ms = Some(100);
        i.now_ms = 70_000;
        assert_eq!(reconcile_pane(&i), ReconcileAction::SetIdle);
    }

    #[test]
    fn running_fresh_heartbeat_stays() {
        let mut i = input(AgentState::Running);
        i.heartbeat_ts_ms = Some(69_500);
        i.now_ms = 70_000;
        assert_eq!(reconcile_pane(&i), ReconcileAction::None);
    }

    #[test]
    fn running_missing_heartbeat_uses_now() {
        assert_eq!(reconcile_pane(&input(AgentState::Running)), ReconcileAction::None);
    }

    #[test]
    fn idle_alive_no_action() {
        assert_eq!(reconcile_pane(&input(AgentState::Idle)), ReconcileAction::None);
    }

    #[test]
    fn done_alive_no_action() {
        assert_eq!(reconcile_pane(&input(AgentState::Done)), ReconcileAction::None);
    }

    #[test]
    fn needs_input_alive_no_action() {
        assert_eq!(reconcile_pane(&input(AgentState::NeedsInput)), ReconcileAction::None);
    }

    #[test]
    fn non_running_state_ignores_heartbeat_timeout() {
        let mut i = input(AgentState::Done);
        i.heartbeat_ts_ms = Some(100);
        i.now_ms = 70_000;
        assert_eq!(reconcile_pane(&i), ReconcileAction::None);
    }
}
