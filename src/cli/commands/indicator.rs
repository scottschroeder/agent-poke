use anyhow::Result;
use clap::Args;

use crate::config;
use crate::tmux::Tmux;

#[derive(Debug, Args)]
pub struct IndicatorCommand;

impl IndicatorCommand {
    pub fn run(self) -> Result<()> {
        let tmux = Tmux::new();
        if !tmux.is_available() {
            return Ok(());
        }

        if !config::enabled(&tmux, "@agent-indicator-indicator-enabled", true) {
            println!();
            return Ok(());
        }

        let pane_id = tmux.display_current("#{pane_id}")?;
        let state_key = format!("TMUX_AGENT_PANE_{pane_id}_STATE");
        let agent_key = format!("TMUX_AGENT_PANE_{pane_id}_AGENT");

        if let Some(state) = tmux.show_env_global(&state_key)?
            && state != "off"
        {
            let agent = tmux
                .show_env_global(&agent_key)?
                .unwrap_or_else(|| "default".to_string());
            let icon_map = config::icon_map(&tmux);
            println!("{}", config::icon_for_agent(&agent, &icon_map));
            return Ok(());
        }

        println!();
        Ok(())
    }
}
