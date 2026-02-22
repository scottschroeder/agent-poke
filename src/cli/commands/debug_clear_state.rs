use std::collections::BTreeSet;

use anyhow::Result;
use clap::Args;

use crate::core::env_keys;
use crate::core::env_keys::WindowEnvKeys;
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

        let window_ids: BTreeSet<String> = agent_env
            .iter()
            .filter_map(|(key, _)| env_keys::parse_window_id_from_key(key))
            .map(ToOwned::to_owned)
            .collect();

        for window_id in &window_ids {
            restore_window_visuals(&tmux, window_id)?;
        }

        for (key, _) in &agent_env {
            tmux.unset_env_global(key)?;
        }

        tmux.refresh_client();
        println!(
            "cleared {} state keys across {} windows",
            agent_env.len(),
            window_ids.len()
        );
        Ok(())
    }
}

fn restore_window_visuals(tmux: &Tmux, window_id: &str) -> Result<()> {
    let keys = WindowEnvKeys::new(window_id);
    restore_window_option(
        tmux,
        window_id,
        "pane-active-border-style",
        &keys.orig_active_border_style,
    )?;
    restore_window_option(
        tmux,
        window_id,
        "window-status-style",
        &keys.orig_status_style,
    )?;
    restore_window_option(
        tmux,
        window_id,
        "window-status-current-style",
        &keys.orig_status_current_style,
    )?;
    tmux.unset_env_global(&keys.done)?;
    Ok(())
}

fn restore_window_option(tmux: &Tmux, window_id: &str, option: &str, env_key: &str) -> Result<()> {
    let Some(saved) = tmux.show_env_global(env_key)? else {
        return Ok(());
    };

    if saved == "__UNSET__" {
        tmux.unset_window_option_target(window_id, option);
    } else {
        tmux.set_window_option_target(window_id, option, &saved)?;
    }
    tmux.unset_env_global(env_key)?;
    Ok(())
}
