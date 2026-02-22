use std::collections::BTreeSet;

use anyhow::Result;

use crate::core::stale;
use crate::notify;
use crate::tmux::Tmux;

pub fn cleanup_stale_state(tmux: &Tmux, clear_notifications: bool) -> Result<usize> {
    let live_pane_ids: BTreeSet<String> = tmux.list_all_pane_ids()?.into_iter().collect();
    let env_pairs = tmux.show_env_global_all()?;
    let stale_pane_ids = stale::stale_pane_ids(&live_pane_ids, &env_pairs);
    let stale_pane_keys = stale::stale_pane_env_keys(&stale_pane_ids, &env_pairs);
    let stale_binding_keys = stale::stale_active_pane_binding_keys(&live_pane_ids, &env_pairs);

    if clear_notifications {
        let stale_key_set: BTreeSet<&str> = stale_pane_keys.iter().map(String::as_str).collect();
        for (key, value) in &env_pairs {
            if key.ends_with("_NOTIFICATION_ID")
                && stale_key_set.contains(key.as_str())
                && !value.is_empty()
            {
                notify::clear_system_notification(value);
            }
        }
    }

    for key in &stale_pane_keys {
        tmux.unset_env_global(key)?;
    }
    for key in &stale_binding_keys {
        tmux.unset_env_global(key)?;
    }

    Ok(stale_pane_keys.len() + stale_binding_keys.len())
}
