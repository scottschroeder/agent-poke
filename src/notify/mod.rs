use anyhow::Result;
use std::process::Command;

pub fn notify_system(title: &str, body: &str) -> Result<()> {
    match std::env::consts::OS {
        "macos" => notify_macos(title, body),
        "linux" => {
            notify_linux(title, body);
            Ok(())
        }
        _ => Ok(()),
    }
}

fn notify_macos(title: &str, body: &str) -> Result<()> {
    let escaped_title = title.replace('"', "\\\"");
    let escaped_body = body.replace('"', "\\\"");
    let script = format!("display notification \"{escaped_body}\" with title \"{escaped_title}\"");

    let _ = Command::new("osascript").arg("-e").arg(script).status()?;
    Ok(())
}

fn notify_linux(title: &str, body: &str) {
    let _ = Command::new("notify-send").arg(title).arg(body).status();
}
