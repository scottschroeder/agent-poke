use std::fs::{OpenOptions, create_dir_all};
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use clap::Args;

use crate::tmux::Tmux;

#[derive(Debug, Args)]
pub struct DebugStressCommand {
    #[arg(long, default_value_t = 25)]
    iterations: u32,

    #[arg(long, default_value_t = 10)]
    sleep_ms: u64,

    #[arg(long, default_value = "opencode")]
    agent: String,

    #[arg(long)]
    pane: Option<String>,

    #[arg(long)]
    window: Option<String>,

    #[arg(long)]
    log_path: Option<PathBuf>,
}

impl DebugStressCommand {
    pub fn run(self) -> Result<()> {
        let tmux = Tmux::new();
        if !tmux.is_available() {
            return Ok(());
        }

        let pane_id = resolve_pane_id(&tmux, self.pane.as_deref())?;
        let window_id = self
            .window
            .unwrap_or(tmux.display_target(&pane_id, "#{window_id}")?);
        let log_path = self.log_path.unwrap_or_else(default_log_path);

        if let Some(parent) = log_path.parent() {
            create_dir_all(parent).with_context(|| {
                format!("failed to create debug log directory {}", parent.display())
            })?;
        }

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .with_context(|| format!("failed to open debug log file {}", log_path.display()))?;

        let current_exe =
            std::env::current_exe().context("failed to resolve current executable")?;
        let state_key = format!("TMUX_AGENT_PANE_{pane_id}_STATE");

        let mut leaked_stdout_steps = 0u32;
        let mut leaked_stderr_steps = 0u32;
        let mut focus_change_steps = 0u32;
        let mut total_steps = 0u32;

        for iter in 0..self.iterations {
            let base_ts = now_epoch_millis().saturating_add((iter as u64) * 100);
            let steps = build_steps(&self.agent, &pane_id, &window_id, base_ts);

            for step in steps {
                total_steps = total_steps.saturating_add(1);

                let before_focus = current_focus(&tmux)?;
                let output = Command::new(&current_exe)
                    .args(&step.args)
                    .output()
                    .with_context(|| format!("failed to run {}", step.name))?;
                let after_focus = current_focus(&tmux)?;

                let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                let state_after = tmux
                    .show_env_global(&state_key)?
                    .unwrap_or_else(|| "idle".to_string());

                let focus_changed = before_focus != after_focus;
                if !stdout.is_empty() {
                    leaked_stdout_steps = leaked_stdout_steps.saturating_add(1);
                }
                if !stderr.is_empty() {
                    leaked_stderr_steps = leaked_stderr_steps.saturating_add(1);
                }
                if focus_changed {
                    focus_change_steps = focus_change_steps.saturating_add(1);
                }

                let line = format!(
                    "ts_ms={} iter={} step={} status={} focus_before={}:{} focus_after={}:{} focus_changed={} state_after={} stdout={} stderr={} args={}\n",
                    now_epoch_millis(),
                    iter,
                    step.name,
                    output.status,
                    before_focus.0,
                    before_focus.1,
                    after_focus.0,
                    after_focus.1,
                    focus_changed,
                    state_after,
                    shell_escape(&stdout),
                    shell_escape(&stderr),
                    step.args.join(" ")
                );
                file.write_all(line.as_bytes())
                    .context("failed to append debug log line")?;

                if self.sleep_ms > 0 {
                    thread::sleep(Duration::from_millis(self.sleep_ms));
                }
            }
        }

        println!(
            "debug-stress complete: iterations={} steps={} focus_changes={} stdout_leaks={} stderr_lines={} log={}",
            self.iterations,
            total_steps,
            focus_change_steps,
            leaked_stdout_steps,
            leaked_stderr_steps,
            log_path.display()
        );

        Ok(())
    }
}

#[derive(Debug)]
struct Step {
    name: &'static str,
    args: Vec<String>,
}

fn build_steps(agent: &str, pane: &str, window: &str, base_ts: u64) -> Vec<Step> {
    vec![
        Step {
            name: "state-closed",
            args: vec![
                "state".to_string(),
                "--agent".to_string(),
                agent.to_string(),
                "--state".to_string(),
                "closed".to_string(),
                "--pane".to_string(),
                pane.to_string(),
            ],
        },
        Step {
            name: "event-busy",
            args: vec![
                "opencode-event".to_string(),
                "--agent".to_string(),
                agent.to_string(),
                "--event".to_string(),
                "session.status".to_string(),
                "--status".to_string(),
                "busy".to_string(),
                "--pane".to_string(),
                pane.to_string(),
                "--event-ts-ms".to_string(),
                (base_ts + 1).to_string(),
            ],
        },
        Step {
            name: "event-needs-input",
            args: vec![
                "opencode-event".to_string(),
                "--agent".to_string(),
                agent.to_string(),
                "--event".to_string(),
                "permission.asked".to_string(),
                "--pane".to_string(),
                pane.to_string(),
                "--event-ts-ms".to_string(),
                (base_ts + 2).to_string(),
            ],
        },
        Step {
            name: "focus-reset",
            args: vec![
                "focus-reset".to_string(),
                "--pane".to_string(),
                pane.to_string(),
                "--window".to_string(),
                window.to_string(),
            ],
        },
        Step {
            name: "event-idle",
            args: vec![
                "opencode-event".to_string(),
                "--agent".to_string(),
                agent.to_string(),
                "--event".to_string(),
                "session.status".to_string(),
                "--status".to_string(),
                "idle".to_string(),
                "--pane".to_string(),
                pane.to_string(),
                "--event-ts-ms".to_string(),
                (base_ts + 3).to_string(),
            ],
        },
        Step {
            name: "focus-reset-done",
            args: vec![
                "focus-reset".to_string(),
                "--pane".to_string(),
                pane.to_string(),
                "--window".to_string(),
                window.to_string(),
            ],
        },
        Step {
            name: "event-busy-new",
            args: vec![
                "opencode-event".to_string(),
                "--agent".to_string(),
                agent.to_string(),
                "--event".to_string(),
                "session.status".to_string(),
                "--status".to_string(),
                "busy".to_string(),
                "--pane".to_string(),
                pane.to_string(),
                "--event-ts-ms".to_string(),
                (base_ts + 5).to_string(),
            ],
        },
        Step {
            name: "event-idle-stale",
            args: vec![
                "opencode-event".to_string(),
                "--agent".to_string(),
                agent.to_string(),
                "--event".to_string(),
                "session.status".to_string(),
                "--status".to_string(),
                "idle".to_string(),
                "--pane".to_string(),
                pane.to_string(),
                "--event-ts-ms".to_string(),
                (base_ts + 4).to_string(),
            ],
        },
        Step {
            name: "event-idle-final",
            args: vec![
                "opencode-event".to_string(),
                "--agent".to_string(),
                agent.to_string(),
                "--event".to_string(),
                "session.idle".to_string(),
                "--pane".to_string(),
                pane.to_string(),
                "--event-ts-ms".to_string(),
                (base_ts + 6).to_string(),
            ],
        },
    ]
}

fn resolve_pane_id(tmux: &Tmux, pane: Option<&str>) -> Result<String> {
    if let Some(pane) = pane {
        return Ok(pane.to_string());
    }

    if let Ok(from_env) = std::env::var("TMUX_PANE")
        && !from_env.is_empty()
    {
        return Ok(from_env);
    }

    tmux.display_current("#{pane_id}")
        .context("failed to resolve current tmux pane id")
}

fn current_focus(tmux: &Tmux) -> Result<(String, String)> {
    Ok((
        tmux.display_current("#{pane_id}")?,
        tmux.display_current("#{window_id}")?,
    ))
}

fn default_log_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(home)
        .join(".local")
        .join("state")
        .join("agent-poke")
        .join("debug-stress.log")
}

fn now_epoch_millis() -> u64 {
    let Ok(duration) = SystemTime::now().duration_since(UNIX_EPOCH) else {
        return 0;
    };
    duration.as_millis() as u64
}

fn shell_escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace('\t', "\\t")
}
