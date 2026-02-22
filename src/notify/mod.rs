use anyhow::Result;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct NotifyOptions {
    pub persistent: bool,
    pub urgency: String,
    pub sound: bool,
    pub sound_name: Option<String>,
    pub replace_tag: Option<String>,
}

pub fn notify_system(title: &str, body: &str, options: &NotifyOptions) -> Result<Option<String>> {
    match std::env::consts::OS {
        "macos" => {
            notify_macos(title, body)?;
            Ok(None)
        }
        "linux" => Ok(notify_linux(title, body, options)),
        _ => Ok(None),
    }
}

pub fn clear_system_notification(notification_id: &str) {
    if notification_id.is_empty() {
        return;
    }

    if std::env::consts::OS != "linux" {
        return;
    }

    let _ = Command::new("gdbus")
        .args([
            "call",
            "--session",
            "--dest",
            "org.freedesktop.Notifications",
            "--object-path",
            "/org/freedesktop/Notifications",
            "--method",
            "org.freedesktop.Notifications.CloseNotification",
            notification_id,
        ])
        .output();
}

fn notify_macos(title: &str, body: &str) -> Result<()> {
    let escaped_title = title.replace('"', "\\\"");
    let escaped_body = body.replace('"', "\\\"");
    let script = format!("display notification \"{escaped_body}\" with title \"{escaped_title}\"");

    let _ = Command::new("osascript").arg("-e").arg(script).output()?;
    Ok(())
}

fn notify_linux(title: &str, body: &str, options: &NotifyOptions) -> Option<String> {
    let mut command = Command::new("notify-send");
    command.arg("--print-id");

    if options.persistent {
        command.arg("--expire-time=0");
        command.arg("--hint=boolean:resident:true");
    } else {
        command.arg("--transient");
    }

    let urgency = match options.urgency.as_str() {
        "low" | "normal" | "critical" => options.urgency.as_str(),
        _ => "normal",
    };
    command.arg(format!("--urgency={urgency}"));

    if let Some(tag) = options.replace_tag.as_deref()
        && !tag.is_empty()
    {
        command.arg(format!(
            "--hint=string:x-canonical-private-synchronous:{tag}"
        ));
    }

    if options.sound {
        let sound_name = options
            .sound_name
            .as_deref()
            .filter(|value| !value.is_empty())
            .unwrap_or("dialog-information");
        command.arg(format!("--hint=string:sound-name:{sound_name}"));
    }

    command.arg(title).arg(body);

    let Ok(output) = command.output() else {
        return None;
    };
    if !output.status.success() {
        return None;
    }

    let id = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if id.is_empty() { None } else { Some(id) }
}
