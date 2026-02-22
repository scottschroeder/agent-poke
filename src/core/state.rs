use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentState {
    Idle,
    Running,
    NeedsInput,
    Done,
    Closed,
}

impl AgentState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Running => "running",
            Self::NeedsInput => "needs-input",
            Self::Done => "done",
            Self::Closed => "closed",
        }
    }

    pub fn is_valid_transition(from: Option<Self>, to: Self) -> bool {
        match (from, to) {
            (None, Self::Idle | Self::Running | Self::NeedsInput | Self::Done) => true,
            (None, Self::Closed) => false,
            (Some(Self::Closed), Self::Idle | Self::Running | Self::NeedsInput | Self::Done) => {
                true
            }
            (Some(Self::Closed), Self::Closed) => true,
            (
                Some(Self::Idle),
                Self::Idle | Self::Running | Self::NeedsInput | Self::Done | Self::Closed,
            ) => true,
            (
                Some(Self::Running),
                Self::Idle | Self::Running | Self::NeedsInput | Self::Done | Self::Closed,
            ) => true,
            (
                Some(Self::NeedsInput),
                Self::Idle | Self::Running | Self::NeedsInput | Self::Done | Self::Closed,
            ) => true,
            (
                Some(Self::Done),
                Self::Idle | Self::Running | Self::NeedsInput | Self::Done | Self::Closed,
            ) => true,
        }
    }

    pub fn notification_label(self) -> &'static str {
        match self {
            Self::NeedsInput => "Needs Attention",
            Self::Done => "Done",
            Self::Running => "Running",
            Self::Idle => "Idle",
            Self::Closed => "Closed",
        }
    }
}

impl fmt::Display for AgentState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for AgentState {
    type Err = String;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input {
            "idle" => Ok(Self::Idle),
            "running" => Ok(Self::Running),
            "needs-input" => Ok(Self::NeedsInput),
            "done" => Ok(Self::Done),
            "closed" => Ok(Self::Closed),
            _ => Err(format!(
                "invalid state '{input}', expected idle|running|needs-input|done|closed"
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AgentState;

    #[test]
    fn parses_valid_state_values() {
        assert_eq!("idle".parse::<AgentState>().ok(), Some(AgentState::Idle));
        assert_eq!(
            "running".parse::<AgentState>().ok(),
            Some(AgentState::Running)
        );
        assert_eq!(
            "needs-input".parse::<AgentState>().ok(),
            Some(AgentState::NeedsInput)
        );
        assert_eq!("done".parse::<AgentState>().ok(), Some(AgentState::Done));
        assert_eq!(
            "closed".parse::<AgentState>().ok(),
            Some(AgentState::Closed)
        );
    }

    #[test]
    fn rejects_invalid_state_values() {
        assert!("paused".parse::<AgentState>().is_err());
    }

    #[test]
    fn allows_defined_transitions() {
        assert!(AgentState::is_valid_transition(None, AgentState::Running));
        assert!(AgentState::is_valid_transition(
            Some(AgentState::Running),
            AgentState::NeedsInput
        ));
        assert!(AgentState::is_valid_transition(
            Some(AgentState::NeedsInput),
            AgentState::Done
        ));
        assert!(AgentState::is_valid_transition(
            Some(AgentState::Done),
            AgentState::Closed
        ));
        assert!(AgentState::is_valid_transition(
            Some(AgentState::Done),
            AgentState::Running
        ));
        assert!(AgentState::is_valid_transition(
            Some(AgentState::Closed),
            AgentState::Idle
        ));
    }

    #[test]
    fn provides_human_friendly_notification_labels() {
        assert_eq!(
            AgentState::NeedsInput.notification_label(),
            "Needs Attention"
        );
        assert_eq!(AgentState::Done.notification_label(), "Done");
    }
}
