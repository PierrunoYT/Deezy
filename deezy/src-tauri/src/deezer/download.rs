use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use futures::StreamExt;
use id3::TagLike;
use serde_json::Value;
use tauri::Emitter;
use tokio::io::AsyncWriteExt;

use super::bandwidth::BandwidthLimiter;
use super::models::DownloadProgress;
use super::models::DownloadResult;
use super::{crypto, get_quality_ext, DeezerClient};
use crate::paths::{
    build_download_path, cleanup_temp_file_async, create_temp_download_file,
    finalize_download_file, find_existing_download, parse_u32_from_value, TrackPathMeta,
};
use crate::settings::FolderStructure;

const MAX_TRACK_DOWNLOAD_BYTES: u64 = 1024 * 1024 * 1024; // 1 GiB safety cap
/// Extensions a track can be saved with. A file with any of them at the
/// destination counts as already downloaded, whatever quality was requested,
/// since the delivered quality can differ from the requested one.
const AUDIO_EXTENSIONS: [&str; 2] = [".mp3", ".flac"];

pub async fn download_track(
    client: &DeezerClient,
    track_id: &str,
    output_dir: &str,
    quality: &str,
    folder_structure: &FolderStructure,
    custom_folder_template: &str,
    skip_existing: bool,
    bandwidth: &BandwidthLimiter,
    speed_limit_kbps: u32,
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

    let mut path_meta = TrackPathMeta::from_deezer(track_data);
    path_meta.title = full_title.clone();

    if skip_existing {
        let existing = find_existing_download(
            output_dir,
            folder_structure,
            custom_folder_template,
            &path_meta,
            &AUDIO_EXTENSIONS,
        )
        .await?;
        if let Some(existing_path) = existing {
            emit_progress(app, track_id, &full_title, 100.0, "complete");
            return Ok(DownloadResult {
                file_path: existing_path.to_string_lossy().to_string(),
                requested_quality: quality.to_string(),
                actual_quality: String::new(),
                status: "skipped".to_string(),
            });
        }
    }

    let (url, actual_quality) = client
        .get_track_download_url(&track, quality, true)
        .await?;

    let ext = get_quality_ext(&actual_quality);
    let bf_key = crypto::get_blowfish_key(&sng_id);

    let download_path = build_download_path(
        output_dir,
        folder_structure,
        custom_folder_template,
        &path_meta,
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
        // Not reading the stream while waiting lets TCP flow control slow the
        // server down. A cancellation is picked up by the chunk loop below.
        bandwidth
            .acquire(bytes.len(), speed_limit_kbps, &cancel_flag)
            .await;
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
        crate::logging::warn("deezer", format!("Tags not written: {}", e));
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

fn extract_val(val: &Value) -> String {
    val.as_str()
        .map(|s| s.to_string())
        .or_else(|| val.as_u64().map(|n| n.to_string()))
        .or_else(|| val.as_i64().map(|n| n.to_string()))
        .unwrap_or_default()
}
