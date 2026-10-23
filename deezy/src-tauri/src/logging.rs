//! Local diagnostic log.
//!
//! Lines are appended to `wavekeep-debug.log` in the system temp folder. The
//! file never leaves the computer: nothing in the app uploads it. Users open or
//! copy it from Info & FAQ to attach it to a bug report themselves.
//!
//! Every message is redacted before it is written: the home folder becomes `~`,
//! URL query strings (which can carry session or download tokens) are dropped,
//! and any unbroken token-like run of 100+ characters, the shape of an ARL or
//! OAuth token, is replaced. Callers must still never pass credentials.

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

const FILE_NAME: &str = "wavekeep-debug.log";
const OLD_FILE_NAME: &str = "wavekeep-debug.old.log";
/// The current file is rotated to `OLD_FILE_NAME` past this size, so the
/// history right before a problem is still there when someone reports it.
const MAX_BYTES: u64 = 1024 * 1024;
/// Unbroken token-like runs at least this long are treated as secrets.
const MIN_SECRET_LEN: usize = 100;
/// Longest single message kept; anything longer is cut.
const MAX_MESSAGE_CHARS: usize = 4000;

static WRITE_LOCK: Mutex<()> = Mutex::new(());

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

/// Where the log lives.
pub fn log_path() -> PathBuf {
    std::env::temp_dir().join(FILE_NAME)
}

fn old_log_path() -> PathBuf {
    std::env::temp_dir().join(OLD_FILE_NAME)
}

pub fn write(level: Level, area: &str, message: &str) {
    let line = format_line(level, area, &redact(message), unix_now());
    eprint!("{}", line);
    // Tests only print: they must not add noise to the user's real log.
    if cfg!(test) {
        return;
    }

    let _guard = WRITE_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let path = log_path();
    if fs::metadata(&path).is_ok_and(|meta| meta.len() > MAX_BYTES) {
        let _ = fs::remove_file(old_log_path());
        let _ = fs::rename(&path, old_log_path());
    }
    if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(&path) {
        let _ = file.write_all(line.as_bytes());
    }
}

/// The last `max_lines` lines, reaching into the rotated file when the current
/// one is short. Used for "Copy recent log".
pub fn tail(max_lines: usize) -> String {
    let _guard = WRITE_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let current = fs::read_to_string(log_path()).unwrap_or_default();
    let old = if current.lines().count() < max_lines {
        fs::read_to_string(old_log_path()).unwrap_or_default()
    } else {
        String::new()
    };
    let lines: Vec<&str> = old.lines().chain(current.lines()).collect();
    let start = lines.len().saturating_sub(max_lines);
    lines[start..].join("\n")
}

fn format_line(level: Level, area: &str, message: &str, unix: u64) -> String {
    let message: String = message.chars().take(MAX_MESSAGE_CHARS).collect();
    format!(
        "{} {:<5} {:<9} {}\n",
        utc_timestamp(unix),
        level.label(),
        area,
        message.trim_end()
    )
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

/// Drops the query string of every `http(s)://` URL, which is where download
/// and API tokens travel.
fn strip_url_queries(message: &str) -> String {
    let mut out = String::with_capacity(message.len());
    let mut rest = message;
    let next_url = |text: &str| [text.find("http://"), text.find("https://")].into_iter().flatten().min();
    while let Some(start) = next_url(rest) {
        out.push_str(&rest[..start]);
        let url_and_after = &rest[start..];
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
        rest = &url_and_after[end..];
    }
    out.push_str(rest);
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
