use super::*;
use crate::logging;

/// Longest message the frontend may log in one call.
const MAX_FRONTEND_MESSAGE: usize = 2000;
/// Lines returned by "Copy recent log" unless the caller asks for fewer.
const DEFAULT_TAIL_LINES: usize = 300;
const MAX_TAIL_LINES: usize = 2000;

/// Lets the frontend add a line to the local diagnostic log (it never leaves
/// the computer). The area is reduced to a short `ui-<name>` tag.
#[tauri::command]
pub async fn write_log(level: String, area: String, message: String) -> Result<(), String> {
    let area: String = area
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
        .take(12)
        .collect();
    let area = if area.is_empty() { "ui".to_string() } else { format!("ui-{}", area) };
    let message = cut_at_word(&message, MAX_FRONTEND_MESSAGE);
    logging::write(logging::Level::parse(&level), &area, &message);
    Ok(())
}

/// The most recent log lines, already redacted, for copying into a report.
#[tauri::command]
pub async fn read_recent_log(lines: Option<usize>) -> Result<String, String> {
    let lines = lines.unwrap_or(DEFAULT_TAIL_LINES).clamp(1, MAX_TAIL_LINES);
    run_blocking(move || Ok(logging::tail(lines))).await
}

/// Deletes the log and its rotated copy ("Delete log" in Info & FAQ).
#[tauri::command]
pub async fn delete_log() -> Result<(), String> {
    run_blocking(logging::delete_logs).await
}

/// Opens the folder that holds the log, with the log file selected where the
/// platform supports it.
#[tauri::command]
pub async fn open_log_folder() -> Result<(), String> {
    let path = logging::log_path().ok_or("The log folder is not ready yet")?;
    if !path.exists() {
        // Creates the file, so there is always something to show.
        logging::info("app", "Log folder opened");
    }

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        // Quote only the path: Explorer does not parse a quoted "/select,...".
        spawn_and_reap(Command::new("explorer").raw_arg(format!("/select,\"{}\"", path.display())))
            .map_err(|e| format!("Failed to open Explorer: {}", e))?;
    }

    #[cfg(target_os = "macos")]
    {
        spawn_and_reap(Command::new("open").arg("-R").arg(&path))
            .map_err(|e| format!("Failed to reveal the log in Finder: {}", e))?;
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let parent = path.parent().ok_or("Failed to resolve the log folder")?;
        spawn_and_reap(Command::new("xdg-open").arg(parent))
            .map_err(|e| format!("Failed to open file manager: {}", e))?;
    }

    Ok(())
}

/// Shortens `text` to at most `max` characters, ending at a whitespace so a
/// token is never split (redaction only recognises whole tokens).
fn cut_at_word(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let cut: String = text.chars().take(max).collect();
    match cut.rfind(char::is_whitespace) {
        Some(end) => format!("{} …", &cut[..end]),
        None => "[message too long to log]".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::cut_at_word;

    #[test]
    fn never_splits_a_word() {
        assert_eq!(cut_at_word("short", 10), "short");
        assert_eq!(cut_at_word("one two three", 9), "one two …");
        assert_eq!(cut_at_word(&"x".repeat(50), 10), "[message too long to log]");
    }
}
