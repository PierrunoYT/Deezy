use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use futures::StreamExt;
use id3::TagLike;
use serde_json::Value;
use tauri::Emitter;
use tokio::io::AsyncWriteExt;

use super::models::DownloadProgress;
use super::models::DownloadResult;
use super::{crypto, get_quality_ext, DeezerClient};
use crate::settings::FolderStructure;

const MAX_TRACK_DOWNLOAD_BYTES: u64 = 1024 * 1024 * 1024; // 1 GiB safety cap
const IN_PROGRESS_SUFFIX: &str = ".deezy.part";

pub async fn download_track(
    client: &DeezerClient,
    track_id: &str,
    output_dir: &str,
    quality: &str,
    folder_structure: &FolderStructure,
    custom_folder_template: &str,
    app: &tauri::AppHandle,
    cancel_flag: Arc<AtomicBool>,
) -> Result<DownloadResult, String> {
    let track = client.get_track(track_id).await?;

    let track_data = if track.get("DATA").is_some() {
        &track["DATA"]
    } else {
        &track
    };

    let title = track_data["SNG_TITLE"]
        .as_str()
        .unwrap_or("Unknown")
        .to_string();
    let artist = track_data["ART_NAME"]
        .as_str()
        .unwrap_or("Unknown")
        .to_string();
    let album_title = track_data["ALB_TITLE"]
        .as_str()
        .unwrap_or("Unknown")
        .to_string();

    let album_id = extract_val(&track_data["ALB_ID"]);
    let sng_id = extract_val(&track_data["SNG_ID"]);

    let mut full_title = title.clone();
    if let Some(version) = track_data["VERSION"].as_str() {
        if !version.is_empty() {
            full_title = format!("{} {}", full_title, version);
        }
    }

    emit_progress(app, track_id, &full_title, 0.0, "resolving");

    let (url, actual_quality) = client
        .get_track_download_url(&track, quality, true)
        .await?;

    let ext = get_quality_ext(&actual_quality);
    let bf_key = crypto::get_blowfish_key(&sng_id);

    let download_path = build_download_path(
        output_dir,
        folder_structure,
        custom_folder_template,
        &artist,
        &album_title,
        &full_title,
        track_data,
        ext,
    )?;
    let download_dir = download_path
        .parent()
        .ok_or("Cannot determine download directory")?
        .to_path_buf();

    tokio::fs::create_dir_all(&download_dir)
        .await
        .map_err(|e| e.to_string())?;

    let base_stem = download_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Unknown")
        .to_string();

    emit_progress(app, track_id, &full_title, 5.0, "downloading");

    let response = client
        .http
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Download failed: {}", e.without_url()))?;

    let status = response.status();
    if !status.is_success() {
        return Err(if matches!(status.as_u16(), 401 | 403) {
            format!("Download authentication token rejected (HTTP {})", status.as_u16())
        } else {
            format!("Download server returned HTTP {}", status.as_u16())
        });
    }

    let total_size_opt = response.content_length();
    if let Some(total_size) = total_size_opt {
        if total_size == 0 {
            return Err("Download failed: empty response".to_string());
        }
        if total_size > MAX_TRACK_DOWNLOAD_BYTES {
            return Err(format!(
                "Download aborted: response too large ({} bytes)",
                total_size
            ));
        }
    }

    let total_size = total_size_opt.unwrap_or(0);
    // Atomically reserve a unique temporary file. Different tracks can resolve
    // to the same display filename, so deriving the temp path only from the
    // destination would let concurrent downloads truncate each other's data.
    let (temp_download_path, file) = create_temp_download_file(&download_path, track_id).await?;
    let mut file = tokio::io::BufWriter::new(file);
    let mut stream = response.bytes_stream();
    let mut buffer: Vec<u8> = Vec::new();
    let mut chunk_index = 0u64;
    let mut downloaded = 0u64;
    let mut received = 0u64;

    while let Some(item) = stream.next().await {
        if cancel_flag.load(Ordering::Relaxed) {
            drop(file);
            cleanup_temp_file_async(&temp_download_path).await;
            return Ok(DownloadResult {
                file_path: String::new(),
                requested_quality: quality.to_string(),
                actual_quality: actual_quality.clone(),
                status: "canceled".to_string(),
            });
        }

        let bytes = match item {
            Ok(bytes) => bytes,
            Err(e) => {
                drop(file);
                cleanup_temp_file_async(&temp_download_path).await;
                return Err(format!("Stream error: {}", e));
            }
        };
        received = received.saturating_add(bytes.len() as u64);
        if received > MAX_TRACK_DOWNLOAD_BYTES {
            drop(file);
            cleanup_temp_file_async(&temp_download_path).await;
            return Err("Download aborted: file exceeds allowed size limit".to_string());
        }
        buffer.extend_from_slice(&bytes);

        while buffer.len() >= 2048 {
            if cancel_flag.load(Ordering::Relaxed) {
                drop(file);
                cleanup_temp_file_async(&temp_download_path).await;
                return Ok(DownloadResult {
                    file_path: String::new(),
                    requested_quality: quality.to_string(),
                    actual_quality: actual_quality.clone(),
                    status: "canceled".to_string(),
                });
            }

            let chunk: Vec<u8> = buffer.drain(..2048).collect();
            if downloaded.saturating_add(chunk.len() as u64) > MAX_TRACK_DOWNLOAD_BYTES {
                drop(file);
                cleanup_temp_file_async(&temp_download_path).await;
                return Err("Download aborted: file exceeds allowed size limit".to_string());
            }

            if chunk_index.is_multiple_of(3) {
                let decrypted = match crypto::decrypt_blowfish_chunk(&chunk, &bf_key) {
                    Ok(decrypted) => decrypted,
                    Err(e) => {
                        drop(file);
                        cleanup_temp_file_async(&temp_download_path).await;
                        return Err(format!("Decryption failed: {}", e));
                    }
                };
                if let Err(e) = file.write_all(&decrypted).await {
                    drop(file);
                    cleanup_temp_file_async(&temp_download_path).await;
                    return Err(e.to_string());
                }
            } else if let Err(e) = file.write_all(&chunk).await {
                drop(file);
                cleanup_temp_file_async(&temp_download_path).await;
                return Err(e.to_string());
            }

            chunk_index += 1;
            downloaded += chunk.len() as u64;

            if total_size > 0 {
                let percent = 5.0 + ((downloaded as f64 / total_size as f64) * 85.0).min(85.0);
                emit_progress(app, track_id, &full_title, percent, "downloading");
            }
        }
    }

    if received == 0 {
        drop(file);
        cleanup_temp_file_async(&temp_download_path).await;
        return Err("Download failed: empty response".to_string());
    }

    // Handle remaining bytes (less than 2048 bytes).
    // Partial trailing chunks are never encrypted in Deezer's scheme,
    // so they are always written as-is.
    if !buffer.is_empty() {
        if cancel_flag.load(Ordering::Relaxed) {
            drop(file);
            cleanup_temp_file_async(&temp_download_path).await;
            return Ok(DownloadResult {
                file_path: String::new(),
                requested_quality: quality.to_string(),
                actual_quality: actual_quality.clone(),
                status: "canceled".to_string(),
            });
        }

        if downloaded.saturating_add(buffer.len() as u64) > MAX_TRACK_DOWNLOAD_BYTES {
            drop(file);
            cleanup_temp_file_async(&temp_download_path).await;
            return Err("Download aborted: file exceeds allowed size limit".to_string());
        }
        if let Err(e) = file.write_all(&buffer).await {
            drop(file);
            cleanup_temp_file_async(&temp_download_path).await;
            return Err(e.to_string());
        }
    }
    if let Err(e) = file.flush().await {
        drop(file);
        cleanup_temp_file_async(&temp_download_path).await;
        return Err(e.to_string());
    }
    drop(file);

    emit_progress(app, track_id, &full_title, 92.0, "tagging");

    let tag_result = if ext == ".mp3" {
        write_mp3_tags(&temp_download_path, &full_title, &artist, &album_title, track_data, client, &album_id).await
    } else if ext == ".flac" {
        write_flac_tags(&temp_download_path, &full_title, &artist, &album_title, track_data, client, &album_id).await
    } else {
        Ok(())
    };

    if let Err(e) = tag_result {
        // Tag writing failed — the audio file itself is intact and usable,
        // so we emit a warning event rather than failing the whole download.
        eprintln!("Warning: failed to write tags: {}", e);
        let _ = app.emit("tag-writing-error", serde_json::json!({
            "track_id": track_id,
            "title": full_title,
            "error": e.to_string()
        }));
    }

    if cancel_flag.load(Ordering::Relaxed) {
        cleanup_temp_file_async(&temp_download_path).await;
        return Ok(DownloadResult {
            file_path: String::new(),
            requested_quality: quality.to_string(),
            actual_quality: actual_quality.clone(),
            status: "canceled".to_string(),
        });
    }

    // Hard-linking is an atomic no-overwrite operation. If another concurrent
    // download claimed the preferred name first, retry with a numbered name.
    let finalize_temp_path = temp_download_path.clone();
    let preferred_path = download_path.clone();
    let finalize_dir = download_dir.clone();
    let finalize_stem = base_stem.clone();
    let finalize_ext = ext.to_string();
    let finalize_cancel_flag = cancel_flag.clone();
    let finalize_task = tokio::task::spawn_blocking(move || {
        finalize_download_file(
            &finalize_temp_path,
            &preferred_path,
            &finalize_dir,
            &finalize_stem,
            &finalize_ext,
            &finalize_cancel_flag,
        )
    });
    let finalize_result = match finalize_task.await {
        Ok(result) => result,
        Err(e) => {
            cleanup_temp_file_async(&temp_download_path).await;
            return Err(format!("File finalization task failed: {}", e));
        }
    };

    let download_path = match finalize_result {
        Ok(path) => path,
        Err(e) => {
            cleanup_temp_file_async(&temp_download_path).await;
            return Err(e);
        }
    };

    // Publication is the commit point. Cancellation after this point is too
    // late and must not remove a valid completed file.
    emit_progress(app, track_id, &full_title, 100.0, "complete");

    Ok(DownloadResult {
        file_path: download_path.to_string_lossy().to_string(),
        requested_quality: quality.to_string(),
        actual_quality,
        status: "complete".to_string(),
    })
}

async fn write_mp3_tags(
    path: &Path,
    title: &str,
    artist: &str,
    album: &str,
    track_data: &Value,
    client: &DeezerClient,
    album_id: &str,
) -> Result<(), String> {
    let mut tag = id3::Tag::new();

    tag.set_title(title);
    tag.set_artist(artist);
    tag.set_album(album);

    if let Some(album_artist) = track_data["ART_NAME"].as_str() {
        tag.set_album_artist(album_artist);
    }

    if let Some(date) = track_data["PHYSICAL_RELEASE_DATE"].as_str() {
        if date.len() >= 4 {
            if let Ok(year) = date[..4].parse::<i32>() {
                tag.set_year(year);
            }
        }
    }

    if let Some(n) = parse_u32_from_value(&track_data["TRACK_NUMBER"]) {
        tag.set_track(n);
    }
    if let Some(n) = parse_u32_from_value(&track_data["DISK_NUMBER"]) {
        tag.set_disc(n);
    }

    if !album_id.is_empty() && album_id != "0" {
        if let Ok(album_data) = client.get_album(album_id).await {
            if let Some(cover_small) = album_data["cover_small"].as_str() {
                let cover_id = cover_small
                    .split("cover/")
                    .nth(1)
                    .and_then(|s| s.split('/').next())
                    .unwrap_or("");

                if !cover_id.is_empty() {
                    if let Ok(cover_bytes) = client.get_album_cover(cover_id, 1000).await {
                        tag.add_frame(id3::Frame::with_content(
                            "APIC",
                            id3::Content::Picture(id3::frame::Picture {
                                mime_type: "image/jpeg".to_string(),
                                picture_type: id3::frame::PictureType::CoverFront,
                                description: String::new(),
                                data: cover_bytes,
                            }),
                        ));
                    }
                }
            }

            if let Some(genres) = album_data["genres"]["data"].as_array() {
                if let Some(first) = genres.first() {
                    if let Some(name) = first["name"].as_str() {
                        tag.set_genre(name);
                    }
                }
            }

            if let Some(label) = album_data["label"].as_str() {
                tag.add_frame(id3::Frame::with_content(
                    "TPUB",
                    id3::Content::Text(label.to_string()),
                ));
            }
        }
    }

    let path = path.to_path_buf();
    tokio::task::spawn_blocking(move || {
        tag.write_to_path(path, id3::Version::Id3v24)
            .map_err(|e| format!("Tag write error: {}", e))
    })
    .await
    .map_err(|e| format!("MP3 tagging task failed: {}", e))??;

    Ok(())
}

async fn write_flac_tags(
    path: &Path,
    title: &str,
    artist: &str,
    album: &str,
    track_data: &Value,
    client: &DeezerClient,
    album_id: &str,
) -> Result<(), String> {
    let read_path = path.to_path_buf();
    let mut tag = tokio::task::spawn_blocking(move || {
        metaflac::Tag::read_from_path(read_path).map_err(|e| format!("FLAC read error: {}", e))
    })
    .await
    .map_err(|e| format!("FLAC tagging task failed: {}", e))??;

    tag.set_vorbis("TITLE", vec![title]);
    tag.set_vorbis("ARTIST", vec![artist]);
    tag.set_vorbis("ALBUM", vec![album]);

    if let Some(album_artist) = track_data["ART_NAME"].as_str() {
        tag.set_vorbis("ALBUMARTIST", vec![album_artist]);
    }

    if let Some(date) = track_data["PHYSICAL_RELEASE_DATE"].as_str() {
        if date.len() >= 4 {
            tag.set_vorbis("DATE", vec![&date[..4]]);
        }
    }

    if let Some(n) = parse_u32_from_value(&track_data["TRACK_NUMBER"]) {
        tag.set_vorbis("TRACKNUMBER", vec![n.to_string()]);
    }
    if let Some(n) = parse_u32_from_value(&track_data["DISK_NUMBER"]) {
        tag.set_vorbis("DISCNUMBER", vec![n.to_string()]);
    }

    if !album_id.is_empty() && album_id != "0" {
        if let Ok(album_data) = client.get_album(album_id).await {
            if let Some(cover_small) = album_data["cover_small"].as_str() {
                let cover_id = cover_small
                    .split("cover/")
                    .nth(1)
                    .and_then(|s| s.split('/').next())
                    .unwrap_or("");

                if !cover_id.is_empty() {
                    if let Ok(cover_bytes) = client.get_album_cover(cover_id, 1000).await {
                        tag.add_picture(
                            "image/jpeg",
                            metaflac::block::PictureType::CoverFront,
                            cover_bytes,
                        );
                    }
                }
            }

            if let Some(genres) = album_data["genres"]["data"].as_array() {
                if let Some(first) = genres.first() {
                    if let Some(name) = first["name"].as_str() {
                        tag.set_vorbis("GENRE", vec![name]);
                    }
                }
            }

            if let Some(label) = album_data["label"].as_str() {
                tag.set_vorbis("LABEL", vec![label]);
            }
        }
    }

    let write_path = path.to_path_buf();
    tokio::task::spawn_blocking(move || {
        tag.write_to_path(write_path)
            .map_err(|e| format!("FLAC tag write error: {}", e))
    })
    .await
    .map_err(|e| format!("FLAC tagging task failed: {}", e))??;

    Ok(())
}

fn build_download_path(
    output_dir: &str,
    folder_structure: &FolderStructure,
    custom_folder_template: &str,
    artist: &str,
    album_title: &str,
    full_title: &str,
    track_data: &Value,
    ext: &str,
) -> Result<PathBuf, String> {
    let base_dir = PathBuf::from(output_dir);

    if *folder_structure != FolderStructure::Custom {
        let download_dir = match folder_structure {
            FolderStructure::Flat => base_dir,
            FolderStructure::ArtistTrack => base_dir.join(sanitize_path_component(artist)),
            FolderStructure::ArtistAlbumTrack => base_dir
                .join(sanitize_path_component(artist))
                .join(sanitize_path_component(album_title)),
            FolderStructure::AlbumTrack => base_dir.join(sanitize_path_component(album_title)),
            FolderStructure::Custom => unreachable!(),
        };

        return Ok(download_dir.join(clean_filename(&format!("{} - {}{}", artist, full_title, ext))));
    }

    let release_date = track_data["PHYSICAL_RELEASE_DATE"]
        .as_str()
        .filter(|date| !date.is_empty())
        .or_else(|| track_data["DIGITAL_RELEASE_DATE"].as_str())
        .unwrap_or("Unknown Date");
    let release_year: String = release_date.chars().take(4).collect();
    let track_number = parse_u32_from_value(&track_data["TRACK_NUMBER"])
        .map(|n| format!("{:02}", n))
        .unwrap_or_else(|| "00".to_string());
    let disc_number = parse_u32_from_value(&track_data["DISK_NUMBER"])
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
    ).map_err(|e| e.to_string())?;
    // Only template separators create directories. Metadata is substituted once
    // so names containing slashes or other placeholders stay literal.
    let mut parts = template
        .split(['/', '\\'])
        .map(|segment| {
            let rendered = placeholders.replace_all(segment, |capture: &regex::Captures<'_>| {
                match &capture[1] {
                    "artist" => artist,
                    "album" => album_title,
                    "title" => full_title,
                    "track_number" | "track" => &track_number,
                    "disc_number" | "disc" => &disc_number,
                    "release_date" => release_date,
                    "release_year" | "year" => &release_year,
                    _ => unreachable!(),
                }.to_string()
            });
            sanitize_path_component(&rendered)
        })
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();

    if parts.is_empty() {
        parts.push(clean_filename(&format!("{} - {}", artist, full_title)));
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

fn emit_progress(
    app: &tauri::AppHandle,
    track_id: &str,
    title: &str,
    percent: f64,
    status: &str,
) {
    let _ = app.emit(
        "download-progress",
        DownloadProgress {
            track_id: track_id.to_string(),
            title: title.to_string(),
            percent,
            status: status.to_string(),
        },
    );
}

fn clean_filename(name: &str) -> String {
    let sanitized = sanitize_path_component(name);
    if sanitized.is_empty() { "_".to_string() } else { sanitized }
}

fn sanitize_path_component(name: &str) -> String {
    let sanitized = name.chars()
        .filter(|c| !c.is_control())
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            _ => c,
        })
        .collect::<String>()
        .trim()
        .trim_matches([' ', '.'])
        .to_string();
    let stem = sanitized.split('.').next().unwrap_or("").trim_end().to_uppercase();
    let device_number = |prefix: &str| stem.strip_prefix(prefix)
        .is_some_and(|suffix| matches!(suffix, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"));
    if matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$")
        || device_number("COM") || device_number("LPT") {
        format!("_{}", sanitized)
    } else {
        sanitized
    }
}

fn extract_val(val: &Value) -> String {
    val.as_str()
        .map(|s| s.to_string())
        .or_else(|| val.as_u64().map(|n| n.to_string()))
        .or_else(|| val.as_i64().map(|n| n.to_string()))
        .unwrap_or_default()
}

fn parse_u32_from_value(val: &Value) -> Option<u32> {
    val.as_str()
        .and_then(|s| s.parse().ok())
        .or_else(|| val.as_u64().and_then(|n| u32::try_from(n).ok()))
}

async fn create_temp_download_file(
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

fn finalize_download_file(
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

    let source_wide: Vec<u16> = source_path.as_os_str().encode_wide().chain(Some(0)).collect();
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
        return Err(io::Error::new(ErrorKind::AlreadyExists, "destination exists"));
    }
    std::fs::rename(source_path, destination_path)
}

fn cleanup_temp_file(path: &Path) {
    let _ = std::fs::remove_file(path);
}

async fn cleanup_temp_file_async(path: &Path) {
    let _ = tokio::fs::remove_file(path).await;
}

#[cfg(test)]
mod tests {
    use super::{build_download_path, rename_noclobber, finalize_download_file, parse_u32_from_value, sanitize_path_component};
    use crate::settings::FolderStructure;
    use std::io::ErrorKind;
    use std::path::PathBuf;
    use std::sync::atomic::AtomicBool;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn custom_paths_keep_metadata_separators_and_placeholders_literal() {
        let path = build_download_path(
            "output", &FolderStructure::Custom, "{artist}/{album}/{track} - {title}",
            "AC/DC", "An {title} album", "A/B", &serde_json::json!({"TRACK_NUMBER": 2}), ".mp3",
        ).unwrap();
        assert_eq!(path, PathBuf::from("output").join("AC_DC").join("An {title} album").join("02 - A_B.mp3"));
    }

    #[test]
    fn custom_paths_accept_multibyte_release_dates_without_panicking() {
        let path = build_download_path(
            "output", &FolderStructure::Custom, "{year}/{title}",
            "Artist", "Album", "Song", &serde_json::json!({"PHYSICAL_RELEASE_DATE": "未定日期"}), ".mp3",
        ).unwrap();
        assert_eq!(path, PathBuf::from("output").join("未定日期").join("Song.mp3"));
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
        assert_eq!(parse_u32_from_value(&serde_json::json!(4_294_967_296u64)), None);
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
