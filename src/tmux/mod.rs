use anyhow::{Context, Result, anyhow};
use std::ffi::OsString;
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

/// Environment variable that overrides the tmux binary path.
/// On macOS with Homebrew, tmux's `#()` subprocesses inherit a minimal
/// system PATH that lacks `/opt/homebrew/bin`, so agent-poke cannot find
/// the `tmux` binary by name. Set this via `set-environment -g` in
/// tmux.conf to provide the absolute path.
const TMUX_BIN_ENV: &str = "AGENT_POKE_TMUX_BIN";

#[derive(Debug)]
pub struct Tmux {
    bin: OsString,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaneSummary {
    pub pane_id: String,
}

/// Full pane snapshot fetched in bulk via [`Tmux::list_all_pane_info`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaneInfo {
    pub pane_id: String,
    pub pid: Option<u32>,
    pub command: Option<String>,
}

impl Tmux {
    pub fn new() -> Self {
        let bin = std::env::var_os(TMUX_BIN_ENV).unwrap_or_else(|| OsString::from("tmux"));
        Self { bin }
    }

    fn tmux_cmd(&self) -> Command {
        Command::new(&self.bin)
    }

    pub fn is_available(&self) -> bool {
        self.tmux_cmd()
            .arg("-V")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    pub fn display_current(&self, format: &str) -> Result<String> {
        self.capture(&["display-message", "-p", format])
    }

    pub fn display_target(&self, target: &str, format: &str) -> Result<String> {
        self.capture(&["display-message", "-p", "-t", target, format])
    }

    pub fn show_env_global(&self, key: &str) -> Result<Option<String>> {
        match self.capture(&["show-environment", "-g", key]) {
            Ok(line) => {
                let value = line
                    .split_once('=')
                    .map(|(_, rhs)| rhs.to_string())
                    .unwrap_or_default();
                if value.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(value))
                }
            }
            Err(_) => Ok(None),
        }
    }

    pub fn show_env_global_all(&self) -> Result<Vec<(String, String)>> {
        let output = self.capture(&["show-environment", "-g"])?;
        let pairs = output
            .lines()
            .filter_map(|line| {
                line.split_once('=')
                    .map(|(key, value)| (key.to_string(), value.to_string()))
            })
            .collect();
        Ok(pairs)
    }

    pub fn option_is_set_global(&self, option: &str) -> bool {
        let output = self
            .tmux_cmd()
            .args(["show-option", "-gq", option])
            .output();
        match output {
            Ok(result) if result.status.success() => {
                !String::from_utf8_lossy(&result.stdout).trim().is_empty()
            }
            _ => false,
        }
    }

    pub fn set_env_global(&self, key: &str, value: &str) -> Result<()> {
        self.status(&["set-environment", "-g", key, value])
            .with_context(|| format!("failed to set tmux env key {key}"))
    }

    pub fn unset_env_global(&self, key: &str) -> Result<()> {
        self.status_ignore_failure(&["set-environment", "-gu", key]);
        Ok(())
    }

    pub fn show_option_global(&self, option: &str) -> Result<Option<String>> {
        match self.capture(&["show-option", "-gqv", option]) {
            Ok(value) if value.is_empty() => Ok(None),
            Ok(value) => Ok(Some(value)),
            Err(_) => Ok(None),
        }
    }

    pub fn list_panes_in_window(&self, window_id: &str) -> Result<Vec<PaneSummary>> {
        let output = self.capture(&["list-panes", "-t", window_id, "-F", "#{pane_id}"])?;

        let panes = output
            .lines()
            .filter_map(|line| {
                let pane_id = line.trim();
                if pane_id.is_empty() {
                    None
                } else {
                    Some(PaneSummary {
                        pane_id: pane_id.to_string(),
                    })
                }
            })
            .collect();

        Ok(panes)
    }

    pub fn list_all_pane_ids(&self) -> Result<Vec<String>> {
        let output = self.capture(&["list-panes", "-a", "-F", "#{pane_id}"])?;
        Ok(output
            .lines()
            .filter(|line| !line.is_empty())
            .map(ToOwned::to_owned)
            .collect())
    }

    /// Bulk pane inventory in a single tmux call: id, pid, and current command
    /// for every pane on the server. Replaces per-pane `display-message`
    /// round-trips in hot paths (reconcile iterates every pane with state).
    pub fn list_all_pane_info(&self) -> Result<Vec<PaneInfo>> {
        let output = self.capture(&[
            "list-panes",
            "-a",
            "-F",
            "#{pane_id}\t#{pane_pid}\t#{pane_current_command}",
        ])?;

        let mut panes = Vec::new();
        for line in output.lines() {
            let mut fields = line.split('\t');
            let (Some(pane_id), pid, command) = (fields.next(), fields.next(), fields.next())
            else {
                continue;
            };
            if pane_id.is_empty() {
                continue;
            }
            panes.push(PaneInfo {
                pane_id: pane_id.to_owned(),
                pid: pid.and_then(|pid| pid.parse::<u32>().ok()),
                command: command.filter(|cmd| !cmd.is_empty()).map(ToOwned::to_owned),
            });
        }
        Ok(panes)
    }

    pub fn pane_pid(&self, pane_id: &str) -> Result<Option<u32>> {
        let raw = self.display_target(pane_id, "#{pane_pid}")?;
        if raw.is_empty() {
            return Ok(None);
        }
        Ok(raw.parse::<u32>().ok())
    }

    pub fn pane_current_command(&self, pane_id: &str) -> Result<Option<String>> {
        let value = self.display_target(pane_id, "#{pane_current_command}")?;
        if value.is_empty() {
            Ok(None)
        } else {
            Ok(Some(value))
        }
    }

    pub fn pid_is_alive(&self, pid: u32) -> bool {
        // /proc existence check avoids spawning `kill` once per tracked pane;
        // reconcile walks every pane with state, so this is a hot path.
        #[cfg(target_os = "linux")]
        {
            std::path::Path::new(&format!("/proc/{pid}")).exists()
        }

        #[cfg(not(target_os = "linux"))]
        {
            Command::new("kill")
                .args(["-0", &pid.to_string()])
                .output()
                .map(|output| output.status.success())
                .unwrap_or(false)
        }
    }

    pub fn refresh_client(&self) {
        self.status_ignore_failure(&["refresh-client", "-S"]);
    }

    pub fn try_lock_channel<'a>(
        &'a self,
        key: &str,
        timeout: Duration,
    ) -> Result<Option<TmuxChannelLock<'a>>> {
        if self.wait_for_lock(key, timeout)? {
            Ok(Some(TmuxChannelLock {
                tmux: self,
                key: key.to_string(),
            }))
        } else {
            Ok(None)
        }
    }

    pub fn unlock_channel(&self, key: &str) {
        self.status_ignore_failure(&["wait-for", "-U", key]);
    }

    fn capture(&self, args: &[&str]) -> Result<String> {
        let output = self
            .tmux_cmd()
            .args(args)
            .output()
            .with_context(|| format!("failed to execute tmux with args {args:?}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(anyhow!("tmux command failed ({args:?}): {stderr}"));
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn status(&self, args: &[&str]) -> Result<()> {
        let output = self
            .tmux_cmd()
            .args(args)
            .output()
            .with_context(|| format!("failed to execute tmux with args {args:?}"))?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            Err(anyhow!("tmux command failed ({args:?}): {stderr}"))
        }
    }

    fn status_ignore_failure(&self, args: &[&str]) {
        let _ = self.tmux_cmd().args(args).output();
    }

    fn wait_for_lock(&self, key: &str, timeout: Duration) -> Result<bool> {
        let mut child = self
            .tmux_cmd()
            .args(["wait-for", "-L", key])
            .spawn()
            .with_context(|| format!("failed to execute tmux wait-for lock for key {key}"))?;

        let start = Instant::now();
        loop {
            if let Some(status) = child
                .try_wait()
                .with_context(|| format!("failed to poll tmux wait-for lock for key {key}"))?
            {
                if status.success() {
                    return Ok(true);
                }
                return Err(anyhow!(
                    "tmux wait-for lock failed for key {key} with status {status}"
                ));
            }

            if start.elapsed() >= timeout {
                let _ = child.kill();
                let _ = child.wait();
                return Ok(false);
            }

            thread::sleep(Duration::from_millis(10));
        }
    }
}

pub struct TmuxChannelLock<'a> {
    tmux: &'a Tmux,
    key: String,
}

impl Drop for TmuxChannelLock<'_> {
    fn drop(&mut self) {
        self.tmux.unlock_channel(&self.key);
    }
}
