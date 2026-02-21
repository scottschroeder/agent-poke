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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationContext {
    pub agent_name: String,
    pub agent_state: AgentState,
    pub session_name: String,
    pub window_name: String,
    pub window_index: String,
}

impl NotificationContext {
    pub fn render(&self, template: &str) -> String {
        template
            .replace("#{agent_name}", &self.agent_name)
            .replace("#{agent_state}", self.agent_state.as_str())
            .replace("#{session_name}", &self.session_name)
            .replace("#{window_name}", &self.window_name)
            .replace("#{window_index}", &self.window_index)
    }
}

#[cfg(test)]
mod tests {
    use super::{AgentState, NotificationContext};

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
    fn renders_notification_template() {
        let ctx = NotificationContext {
            agent_name: "claude".to_string(),
            agent_state: AgentState::NeedsInput,
            session_name: "dev".to_string(),
            window_name: "editor".to_string(),
            window_index: "2".to_string(),
        };

        let rendered =
            ctx.render("[#{agent_name}] #{agent_state} (#{session_name}:#{window_name})");
        assert_eq!(rendered, "[claude] needs-input (dev:editor)");
    }
}
