use anyhow::{Context, Result, anyhow};
use std::process::Command;

#[derive(Debug, Default)]
pub struct Tmux;

impl Tmux {
    pub fn new() -> Self {
        Self
    }

    pub fn is_available(&self) -> bool {
        Command::new("tmux")
            .arg("-V")
            .status()
            .map(|status| status.success())
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

    pub fn display_message(&self, message: &str) {
        self.status_ignore_failure(&["display-message", message]);
    }

    pub fn refresh_client(&self) {
        self.status_ignore_failure(&["refresh-client", "-S"]);
    }

    fn capture(&self, args: &[&str]) -> Result<String> {
        let output = Command::new("tmux")
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
        let status = Command::new("tmux")
            .args(args)
            .status()
            .with_context(|| format!("failed to execute tmux with args {args:?}"))?;

        if status.success() {
            Ok(())
        } else {
            Err(anyhow!("tmux command failed ({args:?})"))
        }
    }

    fn status_ignore_failure(&self, args: &[&str]) {
        let _ = Command::new("tmux").args(args).status();
    }
}
