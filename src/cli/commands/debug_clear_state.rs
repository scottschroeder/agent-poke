use anyhow::Result;
use clap::Args;

use crate::notify;
use crate::tmux::Tmux;

#[derive(Debug, Args)]
pub struct DebugClearStateCommand;

impl DebugClearStateCommand {
    pub fn run(self) -> Result<()> {
        let tmux = Tmux::new();
        if !tmux.is_available() {
            return Ok(());
        }

        let env_pairs = tmux.show_env_global_all()?;
        let agent_env: Vec<(String, String)> = env_pairs
            .into_iter()
            .filter(|(key, _)| key.starts_with("TMUX_AGENT_"))
            .collect();

        for (_, notification_id) in agent_env
            .iter()
            .filter(|(key, _)| key.ends_with("_NOTIFICATION_ID"))
        {
            notify::clear_system_notification(notification_id);
        }

        for (key, _) in &agent_env {
            tmux.unset_env_global(key)?;
        }

        tmux.refresh_client();
        println!("cleared {} state keys", agent_env.len());
        Ok(())
    }
}
