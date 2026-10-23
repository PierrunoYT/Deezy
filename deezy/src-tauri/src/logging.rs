//! Local diagnostic log.
//!
//! Lines are appended to `wavekeep-debug.log` in the app's log folder
//! (`app_log_dir`, e.g. `%LOCALAPPDATA%\com.pierr.wavekeep\logs` on Windows),
//! which the system never cleans up on its own. The file never leaves the
//! computer: nothing in the app uploads it. Users open or copy it from
//! Info & FAQ to attach it to a bug report themselves.
//!
//! The folder is only known once Tauri is set up, so lines logged before
//! `init` (the start line, an early panic) wait in memory and are written then.
//!
//! Every message is redacted before it is written: the home folder becomes `~`,
//! URL query strings (which can carry session or download tokens) are dropped,
//! and any unbroken token-like run of 100+ characters, the shape of an ARL or
//! OAuth token, is replaced. Callers must still never pass credentials.
//!
//! Track names, artists and file paths go through [`detail`], which writes
//! `[hidden]` instead when the user turned off "Include track details in the
//! log" (`log_track_details` in settings).

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

const FILE_NAME: &str = "wavekeep-debug.log";
const OLD_FILE_NAME: &str = "wavekeep-debug.old.log";
/// The current file is rotated to `OLD_FILE_NAME` past this size, so the
/// history right before a problem is still there when someone reports it.
const MAX_BYTES: u64 = 1024 * 1024;
/// Unbroken token-like runs at least this long are treated as secrets.
const MIN_SECRET_LEN: usize = 100;
/// Longest single message kept; anything longer is cut.
const MAX_MESSAGE_CHARS: usize = 4000;

/// Most lines kept in memory before `init`.
const MAX_PENDING_LINES: usize = 500;

static LOG_DIR: OnceLock<PathBuf> = OnceLock::new();
/// Serialises writes and holds the lines logged before `init`.
static PENDING: Mutex<Vec<String>> = Mutex::new(Vec::new());
/// Mirrors the `log_track_details` setting; kept in sync by `Settings`.
static TRACK_DETAILS: AtomicBool = AtomicBool::new(true);

/// Applies the "Include track details in the log" setting.
pub fn set_track_details(enabled: bool) {
    TRACK_DETAILS.store(enabled, Ordering::Relaxed);
}

/// A personal detail for a log line (track name, artist, file path): the value
/// itself, or `[hidden]` when the user keeps track details out of the log.
pub fn detail(value: impl std::fmt::Display) -> String {
    if TRACK_DETAILS.load(Ordering::Relaxed) {
        value.to_string()
    } else {
        "[hidden]".to_string()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Level {
    Debug,
    Info,
    Warn,
    Error,
}

impl Level {
    fn label(self) -> &'static str {
        match self {
            Level::Debug => "DEBUG",
            Level::Info => "INFO",
            Level::Warn => "WARN",
            Level::Error => "ERROR",
        }
    }

    /// Parses the level names the frontend sends; anything else is info.
    pub fn parse(name: &str) -> Level {
        match name.to_ascii_lowercase().as_str() {
            "debug" => Level::Debug,
            "warn" | "warning" => Level::Warn,
            "error" => Level::Error,
            _ => Level::Info,
        }
    }
}

pub fn debug(area: &str, message: impl AsRef<str>) {
    write(Level::Debug, area, message.as_ref());
}

pub fn info(area: &str, message: impl AsRef<str>) {
    write(Level::Info, area, message.as_ref());
}

pub fn warn(area: &str, message: impl AsRef<str>) {
    write(Level::Warn, area, message.as_ref());
}

pub fn error(area: &str, message: impl AsRef<str>) {
    write(Level::Error, area, message.as_ref());
}

/// Sets the log folder and writes the lines logged so far. Called first in
/// the setup hook; later calls are ignored.
pub fn init(dir: PathBuf) {
    let mut pending = PENDING.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let _ = fs::create_dir_all(&dir);
    if LOG_DIR.set(dir).is_err() {
        return;
    }
    if let Some(path) = log_path() {
        append_lines(&path, &pending);
    }
    pending.clear();
    pending.shrink_to_fit();
}

/// Where the log lives, once `init` has run.
pub fn log_path() -> Option<PathBuf> {
    LOG_DIR.get().map(|dir| dir.join(FILE_NAME))
}

pub fn write(level: Level, area: &str, message: &str) {
    let line = format_line(level, area, &redact(message), unix_now());
    // Not eprint!: it panics when stderr is closed (a pipe whose reader exited),
    // which would turn every log call, even the panic hook's, into a panic.
    let _ = std::io::stderr().write_all(line.as_bytes());
    // Tests only print: they must not add noise to the user's real log.
    if cfg!(test) {
        return;
    }

    let mut pending = PENDING.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    match log_path() {
        Some(path) => append_lines(&path, std::slice::from_ref(&line)),
        None if pending.len() < MAX_PENDING_LINES => pending.push(line),
        None => {}
    }
}

/// Appends whole lines, rotating the file to `OLD_FILE_NAME` once it is
/// larger than `MAX_BYTES`.
fn append_lines(path: &Path, lines: &[String]) {
    if lines.is_empty() {
        return;
    }
    let old = path.with_file_name(OLD_FILE_NAME);
    if fs::metadata(path).is_ok_and(|meta| meta.len() > MAX_BYTES) {
        let _ = fs::remove_file(&old);
        let _ = fs::rename(path, &old);
    }
    if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = file.write_all(lines.concat().as_bytes());
    }
}

/// The last `max_lines` lines, reaching into the rotated file when the current
/// one is short. Used for "Copy recent log".
pub fn tail(max_lines: usize) -> String {
    let _guard = PENDING.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let Some(path) = log_path() else {
        return String::new();
    };
    let current = fs::read_to_string(&path).unwrap_or_default();
    let old = if current.lines().count() < max_lines {
        fs::read_to_string(path.with_file_name(OLD_FILE_NAME)).unwrap_or_default()
    } else {
        String::new()
    };
    let lines: Vec<&str> = old.lines().chain(current.lines()).collect();
    let start = lines.len().saturating_sub(max_lines);
    lines[start..].join("\n")
}

/// Deletes the current and the rotated log ("Delete log" in Info & FAQ).
pub fn delete_logs() -> Result<(), String> {
    let _guard = PENDING.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let Some(path) = log_path() else {
        return Ok(());
    };
    for file in [path.with_file_name(OLD_FILE_NAME), path] {
        match fs::remove_file(&file) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(format!("Could not delete the log: {}", e)),
        }
    }
    Ok(())
}

fn format_line(level: Level, area: &str, message: &str, unix: u64) -> String {
    let message: String = message.chars().take(MAX_MESSAGE_CHARS).collect();
    format!(
        "{} {:<5} {:<9} {}\n",
        utc_timestamp(unix),
        level.label(),
        area,
        single_line(message.trim())
    )
}

/// Keeps a message on one line, so text from the renderer or a remote service
/// can't forge log lines of its own. Line breaks become ` ⏎ ` (dropping the
/// indentation after them, which keeps backtraces readable) and other control
/// characters a space.
fn single_line(message: &str) -> String {
    let is_break = |c: char| matches!(c, '\n' | '\r' | '\u{0b}' | '\u{0c}' | '\u{85}' | '\u{2028}' | '\u{2029}');
    let mut out = String::with_capacity(message.len());
    let mut chars = message.chars().peekable();
    while let Some(c) = chars.next() {
        if is_break(c) {
            while chars.next_if(|&next| is_break(next) || next == ' ' || next == '\t').is_some() {}
            out.truncate(out.trim_end().len());
            out.push_str(" ⏎ ");
        } else if c.is_control() {
            out.push(' ');
        } else {
            out.push(c);
        }
    }
    out
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

/// `YYYY-MM-DDTHH:MM:SSZ` without a date crate (civil-from-days algorithm).
fn utc_timestamp(unix: u64) -> String {
    let days = (unix / 86_400) as i64;
    let seconds = unix % 86_400;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year,
        month,
        day,
        seconds / 3_600,
        seconds % 3_600 / 60,
        seconds % 60
    )
}

/// Removes personal and secret-bearing parts of a message.
fn redact(message: &str) -> String {
    let mut text = mask_long_tokens(&strip_url_queries(message));
    for home in home_dirs() {
        text = text.replace(&home, "~");
    }
    text
}

fn home_dirs() -> Vec<String> {
    let mut homes = Vec::new();
    for var in ["USERPROFILE", "HOME"] {
        if let Ok(home) = std::env::var(var) {
            let home = home.trim_end_matches(['\\', '/']).to_string();
            // Avoid turning every "/" into "~" for an empty or root home.
            if home.len() > 3 {
                homes.push(home.replace('/', "\\"));
                homes.push(home.replace('\\', "/"));
            }
        }
    }
    homes.sort();
    homes.dedup();
    homes
}

/// Replaces runs of token characters long enough to be a credential (an ARL
/// is 192 hex characters). Paths and sentences never form such runs because
/// they contain separators.
fn mask_long_tokens(message: &str) -> String {
    let is_token_char = |c: char| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '+' | '=');
    let mut out = String::with_capacity(message.len());
    let mut run = String::new();
    let flush = |run: &mut String, out: &mut String| {
        if run.chars().count() >= MIN_SECRET_LEN {
            out.push_str("[redacted]");
        } else {
            out.push_str(run);
        }
        run.clear();
    };
    for c in message.chars() {
        if is_token_char(c) {
            run.push(c);
        } else {
            flush(&mut run, &mut out);
            out.push(c);
        }
    }
    flush(&mut run, &mut out);
    out
}

/// Drops the query string of every `http(s)://` or `ws(s)://` URL, whatever the
/// case of its scheme, which is where download and API tokens travel.
fn strip_url_queries(message: &str) -> String {
    // ASCII lower-casing changes no byte offsets, so a position found in
    // `lower` is a char boundary in `message` too.
    let lower = message.to_ascii_lowercase();
    let next_url = |from: usize| {
        ["http://", "https://", "ws://", "wss://"]
            .iter()
            .filter_map(|scheme| lower[from..].find(scheme))
            .min()
            .map(|offset| from + offset)
    };
    let mut out = String::with_capacity(message.len());
    let mut position = 0;
    while let Some(start) = next_url(position) {
        out.push_str(&message[position..start]);
        let url_and_after = &message[start..];
        let end = url_and_after
            .find(|c: char| c.is_whitespace() || matches!(c, ')' | '"' | '\'' | '<' | '>'))
            .unwrap_or(url_and_after.len());
        let url = &url_and_after[..end];
        match url.find('?') {
            Some(query) => {
                out.push_str(&url[..query]);
                out.push_str("?[redacted]");
            }
            None => out.push_str(url),
        }
        position = start + end;
    }
    out.push_str(&message[position..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_utc_timestamps() {
        assert_eq!(utc_timestamp(0), "1970-01-01T00:00:00Z");
        assert_eq!(utc_timestamp(951_782_400), "2000-02-29T00:00:00Z");
        assert_eq!(utc_timestamp(1_700_000_000), "2023-11-14T22:13:20Z");
    }

    #[test]
    fn strips_url_queries_but_keeps_the_rest() {
        let message = "request failed for (https://media.deezer.com/v1/get_url?token=abc&x=1): timeout; see http://a.b/c";
        assert_eq!(
            strip_url_queries(message),
            "request failed for (https://media.deezer.com/v1/get_url?[redacted]): timeout; see http://a.b/c"
        );
        assert_eq!(strip_url_queries("no urls here?"), "no urls here?");
    }

    #[test]
    fn strips_queries_whatever_the_scheme_case() {
        assert_eq!(
            strip_url_queries("HTTPS://Media.Deezer.com/x?TOKEN=abc and Http://a.b/?k=v"),
            "HTTPS://Media.Deezer.com/x?[redacted] and Http://a.b/?[redacted]"
        );
        assert_eq!(strip_url_queries("WSS://dealer.example/socket?access_token=t"), "WSS://dealer.example/socket?[redacted]");
        // Multi-byte text before and inside the URL keeps its characters intact.
        assert_eq!(
            strip_url_queries("Ärger: HTTPS://ä.example/ö?q=ü – ende"),
            "Ärger: HTTPS://ä.example/ö?[redacted] – ende"
        );
    }

    #[test]
    fn one_call_writes_exactly_one_line() {
        let forged = "Login ok\n2026-10-23T20:44:32Z ERROR deezer    Visit evil.example\r\nand\u{2028}more\u{1b}[31m\ttab";
        let line = format_line(Level::Info, "ui-console", forged, 0);
        assert_eq!(line.matches('\n').count(), 1);
        assert!(line.ends_with('\n'));
        assert!(!line.trim_end_matches('\n').chars().any(char::is_control), "{:?}", line);
        assert!(line.contains("Login ok ⏎ 2026-10-23T20:44:32Z ERROR deezer"), "{}", line);

        // A backtrace keeps one frame per marker without its indentation.
        let panic = "PANIC: boom\n   0: wavekeep::run\n             at src/lib.rs:40\n";
        assert_eq!(
            single_line(panic.trim()),
            "PANIC: boom ⏎ 0: wavekeep::run ⏎ at src/lib.rs:40"
        );
    }

    #[test]
    fn appends_and_rotates_past_the_size_limit() {
        let dir = std::env::temp_dir().join(format!("wavekeep-log-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join(FILE_NAME);
        fs::write(&path, "x".repeat(MAX_BYTES as usize + 1)).unwrap();

        append_lines(&path, &["one
".to_string(), "two
".to_string()]);

        assert_eq!(fs::read_to_string(&path).unwrap(), "one
two
");
        assert_eq!(fs::metadata(dir.join(OLD_FILE_NAME)).unwrap().len(), MAX_BYTES + 1);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn hides_track_details_when_turned_off() {
        assert_eq!(detail("Artist - Title"), "Artist - Title");
        set_track_details(false);
        assert_eq!(detail("Artist - Title"), "[hidden]");
        set_track_details(true);
        assert_eq!(detail(42), "42");
    }

    #[test]
    fn masks_credential_shaped_tokens_only() {
        let arl = "a1".repeat(96);
        assert_eq!(redact(&format!("bad arl {} here", arl)), "bad arl [redacted] here");
        let hash = "f".repeat(64);
        assert_eq!(mask_long_tokens(&format!("hash {}", hash)), format!("hash {}", hash));
    }

    #[test]
    fn replaces_the_home_folder_with_a_tilde() {
        let home = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")).unwrap();
        let redacted = redact(&format!("saved {}{}Music{}a.ogg", home, std::path::MAIN_SEPARATOR, std::path::MAIN_SEPARATOR));
        assert!(redacted.starts_with("saved ~"), "{}", redacted);
        assert!(!redacted.contains(&home), "{}", redacted);
    }

    #[test]
    fn lines_have_time_level_area_and_a_length_cap() {
        let line = format_line(Level::Warn, "spotify", &"x".repeat(5000), 0);
        assert!(line.starts_with("1970-01-01T00:00:00Z WARN  spotify   x"));
        assert!(line.ends_with("x\n"));
        assert!(line.len() < 4100);
        assert_eq!(Level::parse("ERROR"), Level::Error);
        assert_eq!(Level::parse("anything"), Level::Info);
    }
}
