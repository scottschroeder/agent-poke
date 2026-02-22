use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentState {
    Running,
    NeedsInput,
    Done,
    Off,
}

impl AgentState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::NeedsInput => "needs-input",
            Self::Done => "done",
            Self::Off => "off",
        }
    }

    pub fn is_valid_transition(from: Option<Self>, to: Self) -> bool {
        match (from, to) {
            (None, Self::Running | Self::NeedsInput | Self::Done | Self::Off) => true,
            (Some(Self::Off), Self::Running | Self::NeedsInput | Self::Done | Self::Off) => true,
            (Some(Self::Running), Self::NeedsInput | Self::Done | Self::Off | Self::Running) => {
                true
            }
            (Some(Self::NeedsInput), Self::Running | Self::Done | Self::Off | Self::NeedsInput) => {
                true
            }
            (Some(Self::Done), Self::Running | Self::NeedsInput | Self::Off | Self::Done) => true,
        }
    }

    pub fn notification_label(self) -> &'static str {
        match self {
            Self::NeedsInput => "Needs Attention",
            Self::Done => "Done",
            Self::Running => "Running",
            Self::Off => "Off",
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
            "running" => Ok(Self::Running),
            "needs-input" => Ok(Self::NeedsInput),
            "done" => Ok(Self::Done),
            "off" => Ok(Self::Off),
            _ => Err(format!(
                "invalid state '{input}', expected running|needs-input|done|off"
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AgentState;

    #[test]
    fn parses_valid_state_values() {
        assert_eq!(
            "running".parse::<AgentState>().ok(),
            Some(AgentState::Running)
        );
        assert_eq!(
            "needs-input".parse::<AgentState>().ok(),
            Some(AgentState::NeedsInput)
        );
        assert_eq!("done".parse::<AgentState>().ok(), Some(AgentState::Done));
        assert_eq!("off".parse::<AgentState>().ok(), Some(AgentState::Off));
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
            AgentState::Off
        ));
        assert!(AgentState::is_valid_transition(
            Some(AgentState::Done),
            AgentState::Running
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
