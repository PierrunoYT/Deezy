//! Provider-neutral destination paths and safe file finalization.
//!
//! Both download engines (Deezer and Spotify) use these helpers so that
//! `folder_structure`, `custom_folder_template` and collision handling behave
//! identically regardless of where the audio came from.

use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::settings::FolderStructure;

pub const IN_PROGRESS_SUFFIX: &str = ".deezy.part";

/// The metadata a folder template can reference, independent of provider.
#[derive(Debug, Clone, Default)]
pub struct TrackPathMeta {
    pub artist: String,
    pub album: String,
    pub title: String,
    pub track_number: Option<u32>,
    pub disc_number: Option<u32>,
    /// ISO-8601 date (`YYYY-MM-DD`) when known.
    pub release_date: Option<String>,
}

impl TrackPathMeta {
    /// Reads the fields Deezer's `song.getData` response already provides.
    pub fn from_deezer(track_data: &serde_json::Value) -> Self {
        let release_date = track_data["PHYSICAL_RELEASE_DATE"]
            .as_str()
            .filter(|date| !date.is_empty())
            .or_else(|| {
                track_data["DIGITAL_RELEASE_DATE"]
                    .as_str()
                    .filter(|date| !date.is_empty())
            })
            .map(str::to_string);

        Self {
            artist: track_data["ART_NAME"]
                .as_str()
                .unwrap_or("Unknown")
                .to_string(),
            album: track_data["ALB_TITLE"]
                .as_str()
                .unwrap_or("Unknown")
                .to_string(),
            title: String::new(),
            track_number: parse_u32_from_value(&track_data["TRACK_NUMBER"]),
            disc_number: parse_u32_from_value(&track_data["DISK_NUMBER"]),
            release_date,
        }
    }
}

pub fn build_download_path(
    output_dir: &str,
    folder_structure: &FolderStructure,
    custom_folder_template: &str,
    meta: &TrackPathMeta,
    ext: &str,
) -> Result<PathBuf, String> {
    let base_dir = PathBuf::from(output_dir);

    if *folder_structure != FolderStructure::Custom {
        let download_dir = match folder_structure {
            FolderStructure::Flat => base_dir,
            FolderStructure::ArtistTrack => base_dir.join(sanitize_path_component(&meta.artist)),
            FolderStructure::ArtistAlbumTrack => base_dir
                .join(sanitize_path_component(&meta.artist))
                .join(sanitize_path_component(&meta.album)),
            FolderStructure::AlbumTrack => base_dir.join(sanitize_path_component(&meta.album)),
            FolderStructure::Custom => unreachable!(),
        };

        return Ok(download_dir.join(clean_filename(&format!(
            "{} - {}{}",
            meta.artist, meta.title, ext
        ))));
    }

    let release_date = meta
        .release_date
        .as_deref()
        .filter(|date| !date.is_empty())
        .unwrap_or("Unknown Date");
    let release_year: String = release_date.chars().take(4).collect();
    let track_number = meta
        .track_number
        .map(|n| format!("{:02}", n))
        .unwrap_or_else(|| "00".to_string());
    let disc_number = meta
        .disc_number
        .map(|n| n.to_string())
        .unwrap_or_else(|| "1".to_string());

    let template = custom_folder_template.trim();
    let template = if template.is_empty() {
        "{artist}/{release_date} - {album}/{track_number} - {title}"
    } else {
        template
    };

    let placeholders = regex::Regex::new(
        r"\{(artist|album|title|track_number|track|disc_number|disc|release_date|release_year|year)\}",
    )
    .map_err(|e| e.to_string())?;
    // Only template separators create directories. Metadata is substituted once
    // so names containing slashes or other placeholders stay literal.
    let mut parts = template
        .split(['/', '\\'])
        .map(|segment| {
            let rendered = placeholders.replace_all(segment, |capture: &regex::Captures<'_>| {
                match &capture[1] {
                    "artist" => meta.artist.as_str(),
                    "album" => meta.album.as_str(),
                    "title" => meta.title.as_str(),
                    "track_number" | "track" => track_number.as_str(),
                    "disc_number" | "disc" => disc_number.as_str(),
                    "release_date" => release_date,
                    "release_year" | "year" => release_year.as_str(),
                    _ => unreachable!(),
                }
                .to_string()
            });
            sanitize_path_component(&rendered)
        })
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();

    if parts.is_empty() {
        parts.push(clean_filename(&format!("{} - {}", meta.artist, meta.title)));
    }

    let file_name = parts.pop().unwrap();
    let mut path = base_dir;
    for part in parts {
        path = path.join(part);
    }

    let file_name = if file_name.ends_with(ext) {
        file_name
    } else {
        format!("{}{}", file_name, ext)
    };

    Ok(path.join(clean_filename(&file_name)))
}

/// Returns the path of a file already saved for this track, if any.
pub async fn find_existing_download(
    output_dir: &str,
    folder_structure: &FolderStructure,
    custom_folder_template: &str,
    meta: &TrackPathMeta,
    extensions: &[&str],
) -> Result<Option<PathBuf>, String> {
    for ext in extensions {
        let candidate = build_download_path(
            output_dir,
            folder_structure,
            custom_folder_template,
            meta,
            ext,
        )?;
        if tokio::fs::metadata(&candidate)
            .await
            .is_ok_and(|metadata| metadata.is_file() && metadata.len() > 0)
        {
            return Ok(Some(candidate));
        }
    }
    Ok(None)
}

pub fn clean_filename(name: &str) -> String {
    let sanitized = sanitize_path_component(name);
    if sanitized.is_empty() {
        "_".to_string()
    } else {
        sanitized
    }
}

pub fn sanitize_path_component(name: &str) -> String {
    let sanitized = name
        .chars()
        .filter(|c| !c.is_control())
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            _ => c,
        })
        .collect::<String>()
        .trim()
        .trim_matches([' ', '.'])
        .to_string();
    let stem = sanitized
        .split('.')
        .next()
        .unwrap_or("")
        .trim_end()
        .to_uppercase();
    let device_number = |prefix: &str| {
        stem.strip_prefix(prefix).is_some_and(|suffix| {
            matches!(suffix, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³")
        })
    };
    if matches!(
        stem.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
    ) || device_number("COM")
        || device_number("LPT")
    {
        format!("_{}", sanitized)
    } else {
        sanitized
    }
}

pub fn parse_u32_from_value(val: &serde_json::Value) -> Option<u32> {
    val.as_str()
        .and_then(|s| s.parse().ok())
        .or_else(|| val.as_u64().and_then(|n| u32::try_from(n).ok()))
}

pub async fn create_temp_download_file(
    download_path: &Path,
    track_id: &str,
) -> Result<(PathBuf, tokio::fs::File), String> {
    let parent = download_path
        .parent()
        .ok_or("Cannot determine download directory")?;
    let file_name = download_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("download");
    let safe_track_id = sanitize_path_component(track_id);

    for counter in 0..1000 {
        let temp_name = format!(
            "{}.{}.{}{}",
            file_name, safe_track_id, counter, IN_PROGRESS_SUFFIX
        );
        let temp_path = parent.join(temp_name);
        match tokio::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)
            .await
        {
            Ok(file) => return Ok((temp_path, file)),
            Err(e) if e.kind() == ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(format!("Cannot create temporary file: {}", e)),
        }
    }

    Err("Too many temporary files for this track".to_string())
}

pub fn finalize_download_file(
    temp_path: &Path,
    preferred_path: &Path,
    download_dir: &Path,
    base_stem: &str,
    ext: &str,
    cancel_flag: &AtomicBool,
) -> Result<PathBuf, String> {
    for counter in 0..1000 {
        if cancel_flag.load(Ordering::Relaxed) {
            return Err("Download canceled during file finalization".to_string());
        }
        let candidate = if counter == 0 {
            preferred_path.to_path_buf()
        } else {
            download_dir.join(format!("{} ({}){}", base_stem, counter, ext))
        };

        match std::fs::hard_link(temp_path, &candidate) {
            Ok(()) => {
                cleanup_temp_file(temp_path);
                return Ok(candidate);
            }
            Err(e) if e.kind() == ErrorKind::AlreadyExists => continue,
            // Volumes without hard links (e.g. FAT/exFAT). The temp file is in
            // the same directory, so a rename is atomic: the final name never
            // holds a partial file, unlike copying into it.
            Err(link_error) => match rename_noclobber(temp_path, &candidate) {
                Ok(()) => return Ok(candidate),
                Err(e) if e.kind() == ErrorKind::AlreadyExists => continue,
                Err(rename_error) => {
                    return Err(format!(
                        "Failed to finalize download file (link: {}; rename fallback: {})",
                        link_error, rename_error
                    ));
                }
            },
        }
    }

    Err("Too many files with the same name".to_string())
}

/// Rename `source_path` to `destination_path`, failing with `AlreadyExists`
/// instead of replacing an existing file.
#[cfg(windows)]
fn rename_noclobber(source_path: &Path, destination_path: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::MoveFileExW;

    let source_wide: Vec<u16> = source_path
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    let destination_wide: Vec<u16> = destination_path
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    // Without MOVEFILE_REPLACE_EXISTING the move fails if the destination exists.
    let result = unsafe { MoveFileExW(source_wide.as_ptr(), destination_wide.as_ptr(), 0) };
    if result == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

/// Rename `source_path` to `destination_path`, failing with `AlreadyExists`
/// instead of replacing an existing file. Only used where hard links are
/// unsupported, which also rules out RENAME_NOREPLACE on most such volumes, so
/// the existence check and rename are separate steps.
#[cfg(not(windows))]
fn rename_noclobber(source_path: &Path, destination_path: &Path) -> io::Result<()> {
    if std::fs::symlink_metadata(destination_path).is_ok() {
        return Err(io::Error::new(
            ErrorKind::AlreadyExists,
            "destination exists",
        ));
    }
    std::fs::rename(source_path, destination_path)
}

pub fn cleanup_temp_file(path: &Path) {
    let _ = std::fs::remove_file(path);
}

pub async fn cleanup_temp_file_async(path: &Path) {
    let _ = tokio::fs::remove_file(path).await;
}

#[cfg(test)]
mod tests {
    use super::{
        build_download_path, finalize_download_file, find_existing_download, parse_u32_from_value,
        rename_noclobber, sanitize_path_component, TrackPathMeta,
    };
    use crate::settings::FolderStructure;
    use std::io::ErrorKind;
    use std::path::PathBuf;
    use std::sync::atomic::AtomicBool;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn meta(artist: &str, album: &str, title: &str) -> TrackPathMeta {
        TrackPathMeta {
            artist: artist.to_string(),
            album: album.to_string(),
            title: title.to_string(),
            ..TrackPathMeta::default()
        }
    }

    #[test]
    fn custom_paths_keep_metadata_separators_and_placeholders_literal() {
        let mut m = meta("AC/DC", "An {title} album", "A/B");
        m.track_number = Some(2);
        let path = build_download_path(
            "output",
            &FolderStructure::Custom,
            "{artist}/{album}/{track} - {title}",
            &m,
            ".mp3",
        )
        .unwrap();
        assert_eq!(
            path,
            PathBuf::from("output")
                .join("AC_DC")
                .join("An {title} album")
                .join("02 - A_B.mp3")
        );
    }

    #[test]
    fn custom_paths_accept_multibyte_release_dates_without_panicking() {
        let mut m = meta("Artist", "Album", "Song");
        m.release_date = Some("未定日期".to_string());
        let path = build_download_path(
            "output",
            &FolderStructure::Custom,
            "{year}/{title}",
            &m,
            ".mp3",
        )
        .unwrap();
        assert_eq!(
            path,
            PathBuf::from("output").join("未定日期").join("Song.mp3")
        );
    }

    #[test]
    fn path_components_reject_windows_device_names_and_control_characters() {
        for name in ["CON", "nul.mp3", "LPT1", "COM¹.flac"] {
            assert!(sanitize_path_component(name).starts_with('_'));
        }
        assert_eq!(sanitize_path_component(" Song\n\t. "), "Song");
        assert_eq!(sanitize_path_component("COM10"), "COM10");
    }

    #[test]
    fn numeric_metadata_does_not_wrap_on_overflow() {
        assert_eq!(
            parse_u32_from_value(&serde_json::json!(4_294_967_296u64)),
            None
        );
        assert_eq!(parse_u32_from_value(&serde_json::json!("12")), Some(12));
    }

    fn test_dir(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after the Unix epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "deezy-{}-{}-{}",
            name,
            std::process::id(),
            unique
        ));
        std::fs::create_dir_all(&dir).expect("test directory should be created");
        dir
    }

    #[tokio::test]
    async fn existing_downloads_are_found_in_either_format() {
        let dir = test_dir("existing-download");
        let output_dir = dir.to_string_lossy().to_string();
        let m = meta("Artist", "Album", "Song");
        let find = || {
            find_existing_download(
                &output_dir,
                &FolderStructure::ArtistTrack,
                "",
                &m,
                &[".mp3", ".flac"],
            )
        };

        assert_eq!(find().await.unwrap(), None);

        let artist_dir = dir.join("Artist");
        std::fs::create_dir_all(&artist_dir).expect("artist directory should be created");
        let flac_path = artist_dir.join("Artist - Song.flac");
        std::fs::write(&flac_path, b"").expect("empty file should be written");
        assert_eq!(find().await.unwrap(), None, "an empty file is not a download");

        std::fs::write(&flac_path, b"audio").expect("audio should be written");
        assert_eq!(find().await.unwrap(), Some(flac_path));
        std::fs::remove_dir_all(dir).expect("test directory should be removed");
    }

    #[test]
    fn finalization_uses_a_numbered_name_without_overwriting() {
        let dir = test_dir("finalize-collision");
        let temp_path = dir.join("track.mp3.123.0.deezy.part");
        let preferred_path = dir.join("Artist - Track.mp3");
        std::fs::write(&temp_path, b"new audio").expect("temp audio should be written");
        std::fs::write(&preferred_path, b"existing audio")
            .expect("existing audio should be written");

        let result = finalize_download_file(
            &temp_path,
            &preferred_path,
            &dir,
            "Artist - Track",
            ".mp3",
            &AtomicBool::new(false),
        )
        .expect("finalization should succeed");

        assert_eq!(result, dir.join("Artist - Track (1).mp3"));
        assert_eq!(std::fs::read(&preferred_path).unwrap(), b"existing audio");
        assert_eq!(std::fs::read(&result).unwrap(), b"new audio");
        assert!(!temp_path.exists());
        std::fs::remove_dir_all(dir).expect("test directory should be removed");
    }

    #[test]
    fn rename_fallback_never_overwrites_an_existing_file() {
        let dir = test_dir("rename-no-clobber");
        let source_path = dir.join("source.part");
        let destination_path = dir.join("destination.mp3");
        std::fs::write(&source_path, b"new audio").expect("source should be written");
        std::fs::write(&destination_path, b"existing audio")
            .expect("destination should be written");

        let error = rename_noclobber(&source_path, &destination_path)
            .expect_err("rename should reject an existing destination");

        assert_eq!(error.kind(), ErrorKind::AlreadyExists);
        assert_eq!(std::fs::read(&destination_path).unwrap(), b"existing audio");
        assert_eq!(std::fs::read(&source_path).unwrap(), b"new audio");
        std::fs::remove_dir_all(dir).expect("test directory should be removed");
    }

    #[test]
    fn rename_fallback_moves_to_a_free_name() {
        let dir = test_dir("rename-free");
        let source_path = dir.join("source.part");
        let destination_path = dir.join("destination.mp3");
        std::fs::write(&source_path, b"new audio").expect("source should be written");

        rename_noclobber(&source_path, &destination_path).expect("rename should succeed");

        assert_eq!(std::fs::read(&destination_path).unwrap(), b"new audio");
        assert!(!source_path.exists());
        std::fs::remove_dir_all(dir).expect("test directory should be removed");
    }
}
