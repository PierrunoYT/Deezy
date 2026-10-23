use super::*;

const MAX_HISTORY_BYTES: u64 = 10 * 1024 * 1024;
const MAX_HISTORY_ENTRIES: usize = 10_000;

fn validate_history(history: &[serde_json::Value]) -> Result<(), String> {
    if history.len() > MAX_HISTORY_ENTRIES {
        return Err(format!(
            "Download history exceeds the limit of {} entries",
            MAX_HISTORY_ENTRIES
        ));
    }
    Ok(())
}

#[tauri::command]
pub async fn save_download_history(history: Vec<serde_json::Value>, app: AppHandle) -> Result<(), String> {
    validate_history(&history)?;
    run_blocking(move || {
        let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let path = dir.join("download_history.json");
        let data = serde_json::to_string_pretty(&history).map_err(|e| e.to_string())?;
        if data.len() as u64 > MAX_HISTORY_BYTES {
            return Err("Download history is too large to save".to_string());
        }
        crate::settings::write_private(&path, data.as_bytes())
    }).await
}

#[tauri::command]
pub async fn load_download_history(app: AppHandle) -> Result<Vec<serde_json::Value>, String> {
    run_blocking(move || {
        let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
        let path = dir.join("download_history.json");
        if path.exists() {
            let data = read_history_file(&path)?;
            let history: Vec<serde_json::Value> =
                serde_json::from_str(&data).map_err(|e| e.to_string())?;
            validate_history(&history)?;
            Ok(history)
        } else {
            Ok(vec![])
        }
    }).await
}

/// Reads the history file, enforcing the size limit on the bytes actually read
/// so a file that grows after a size check can't slip past it.
fn read_history_file(path: &std::path::Path) -> Result<String, String> {
    use std::io::Read;

    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut data = String::new();
    file.take(MAX_HISTORY_BYTES + 1)
        .read_to_string(&mut data)
        .map_err(|e| e.to_string())?;
    if data.len() as u64 > MAX_HISTORY_BYTES {
        return Err("Download history file is too large".to_string());
    }
    Ok(data)
}

#[tauri::command]
pub async fn export_download_history(
    history: Vec<serde_json::Value>,
    format: String,
    app: AppHandle,
) -> Result<String, String> {
    validate_history(&history)?;
    let extension = match format.as_str() {
        "csv" => "csv",
        "json" => "json",
        _ => return Err("Invalid format. Use 'csv' or 'json'.".to_string()),
    };

    let (tx, rx) = tokio::sync::oneshot::channel();

    app.dialog()
        .file()
        .set_title("Export Download History")
        .add_filter(format.to_uppercase(), &[extension])
        .set_file_name(format!("wavekeep_download_history.{}", extension))
        .save_file(move |file_path| {
            let _ = tx.send(
                file_path.and_then(|p| p.as_path().map(|path| path.to_string_lossy().to_string())),
            );
        });

    let file_path = match rx.await {
        Ok(Some(path)) => path,
        Ok(None) => return Err("Export cancelled".to_string()),
        Err(_) => return Err("Failed to get file path".to_string()),
    };

    let output_path = file_path.clone();
    run_blocking(move || {
        let content = if format == "csv" {
            generate_csv(&history)?
        } else {
            serde_json::to_string_pretty(&history).map_err(|e| e.to_string())?
        };
        std::fs::write(&output_path, content).map_err(|e| e.to_string())
    }).await?;
    Ok(file_path)
}

fn generate_csv(history: &[serde_json::Value]) -> Result<String, String> {
    let mut csv = String::from("Title,Artist,Album,Status,Progress,Timestamp,File Path,Error Message\n");

    for item in history {
        let title = sanitize_csv_field(item["title"].as_str().unwrap_or(""));
        let artist = sanitize_csv_field(item["artist"].as_str().unwrap_or(""));
        let album = sanitize_csv_field(item["album"].as_str().unwrap_or(""));
        let status = sanitize_csv_field(item["status"].as_str().unwrap_or(""));
        let percent = format!("{:.1}%", item["percent"].as_f64().unwrap_or(0.0));
        let timestamp = sanitize_csv_field(item["timestamp"].as_str().unwrap_or(""));
        let file_path = sanitize_csv_field(item["filePath"].as_str().unwrap_or(""));
        let error_msg = sanitize_csv_field(item["errorMsg"].as_str().unwrap_or(""));

        csv.push_str(&format!(
            "\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\"\n",
            title, artist, album, status, percent, timestamp, file_path, error_msg
        ));
    }

    Ok(csv)
}

/// Escapes quotes and neutralises values a spreadsheet would run as a formula:
/// those starting with a line break or tab, or whose first non-blank character
/// is `=`, `+`, `-` or `@` (spreadsheets skip leading blanks).
fn sanitize_csv_field(value: &str) -> String {
    let escaped = value.replace("\"", "\"\"");
    let starts_with_control = escaped.starts_with(['\t', '\r', '\n']);
    let starts_with_formula = escaped.trim_start().starts_with(['=', '+', '-', '@']);
    if starts_with_control || starts_with_formula {
        return format!("'{}", escaped);
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::{sanitize_csv_field, validate_history, MAX_HISTORY_ENTRIES};

    #[test]
    fn rejects_excessive_history_entries() {
        let history = vec![serde_json::Value::Null; MAX_HISTORY_ENTRIES + 1];
        assert!(validate_history(&history).is_err());
    }

    #[test]
    fn neutralises_spreadsheet_formulas() {
        for formula in ["=1+1", "+1", "-1", "@SUM(A1)", " =HYPERLINK(\"x\")", "\t=1", "\nplain", "\r\n=1"] {
            assert!(sanitize_csv_field(formula).starts_with('\''), "{:?}", formula);
        }
        assert_eq!(sanitize_csv_field("Song \"Live\""), "Song \"\"Live\"\"");
        assert_eq!(sanitize_csv_field("Daft Punk - One More Time"), "Daft Punk - One More Time");
        assert_eq!(sanitize_csv_field(""), "");
    }
}
