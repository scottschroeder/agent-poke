use anyhow::Result;
use clap::Args;

use crate::cli::commands::state_cleanup;
use crate::core::env_keys::{PaneEnvKeys, WindowEnvKeys};
use crate::notify;
use crate::tmux::Tmux;

#[derive(Debug, Args)]
pub struct FocusResetCommand {
    #[arg(long)]
    pane: Option<String>,

    #[arg(long)]
    window: Option<String>,
}

impl FocusResetCommand {
    pub fn run(self) -> Result<()> {
        let tmux = Tmux::new();
        if !tmux.is_available() {
            return Ok(());
        }

        state_cleanup::cleanup_stale_state(&tmux, true)?;

        let pane_id = self
            .pane
            .or_else(|| std::env::var("TMUX_PANE").ok())
            .unwrap_or(tmux.display_current("#{pane_id}")?);
        let window_id = self
            .window
            .unwrap_or(tmux.display_target(&pane_id, "#{window_id}")?);

        let pane_active = tmux.display_target(&pane_id, "#{pane_active}")?;
        let window_active = tmux.display_target(&window_id, "#{window_active}")?;
        if pane_active != "1" || window_active != "1" {
            return Ok(());
        }

        let pane_keys = PaneEnvKeys::new(&pane_id);

        let window_keys = WindowEnvKeys::new(&window_id);

        let pending_reset = tmux
            .show_env_global(&pane_keys.pending_reset)?
            .unwrap_or_default();
        if pending_reset == "1" {
            restore_window_option(
                &tmux,
                &window_id,
                "pane-active-border-style",
                &window_keys.orig_active_border_style,
            )?;
            tmux.unset_env_global(&pane_keys.pending_reset)?;
        }

        let window_done = tmux.show_env_global(&window_keys.done)?.unwrap_or_default();
        let state = tmux.show_env_global(&pane_keys.state)?.unwrap_or_default();
        let done_marker = tmux.show_env_global(&pane_keys.done)?.unwrap_or_default();

        if (state == "needs-input" || state == "done" || done_marker == "1" || window_done == "1")
            && let Some(notification_id) = tmux.show_env_global(&pane_keys.notification_id)?
        {
            notify::clear_system_notification(&notification_id);
            tmux.unset_env_global(&pane_keys.notification_id)?;
        }

        let done_window = tmux
            .show_env_global(&pane_keys.done_window)?
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| window_id.clone());
        let done_window_keys = WindowEnvKeys::new(&done_window);

        if state == "needs-input" {
            restore_window_title_style(&tmux, &window_id)?;
            restore_window_option(
                &tmux,
                &window_id,
                "pane-active-border-style",
                &window_keys.orig_active_border_style,
            )?;
        }

        if window_done == "1" || state == "done" || done_marker == "1" {
            restore_window_title_style(&tmux, &done_window)?;
            restore_window_option(
                &tmux,
                &done_window,
                "pane-active-border-style",
                &done_window_keys.orig_active_border_style,
            )?;

            tmux.unset_env_global(&done_window_keys.done)?;
            tmux.unset_env_global(&pane_keys.done)?;
            tmux.unset_env_global(&pane_keys.done_window)?;
            tmux.unset_env_global(&pane_keys.state)?;
            tmux.unset_env_global(&pane_keys.agent)?;
            tmux.unset_env_global(&pane_keys.last_event_ts_ms)?;
            tmux.unset_env_global(&pane_keys.last_event_priority)?;

            for (key, value) in tmux.show_env_global_all()? {
                if !key.starts_with("TMUX_AGENT_PANE_") || !key.ends_with("_DONE_WINDOW") {
                    continue;
                }
                if value != done_window {
                    continue;
                }

                let Some(prefix) = key.strip_suffix("_DONE_WINDOW") else {
                    continue;
                };

                tmux.unset_env_global(&format!("{prefix}_DONE"))?;
                tmux.unset_env_global(&format!("{prefix}_DONE_WINDOW"))?;
                tmux.unset_env_global(&format!("{prefix}_PENDING_RESET"))?;
                tmux.unset_env_global(&format!("{prefix}_STATE"))?;
                tmux.unset_env_global(&format!("{prefix}_AGENT"))?;
                tmux.unset_env_global(&format!("{prefix}_LAST_EVENT_TS_MS"))?;
                tmux.unset_env_global(&format!("{prefix}_LAST_EVENT_PRIORITY"))?;

                let notification_key = format!("{prefix}_NOTIFICATION_ID");
                if let Some(notification_id) = tmux.show_env_global(&notification_key)? {
                    notify::clear_system_notification(&notification_id);
                }
                tmux.unset_env_global(&notification_key)?;
            }
        }

        tmux.refresh_client();
        Ok(())
    }
}

fn restore_window_title_style(tmux: &Tmux, window_id: &str) -> Result<()> {
    let keys = WindowEnvKeys::new(window_id);

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
    tmux.unset_env_global(&keys.done)
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
