use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use librespot_audio::AudioFile;
use librespot_core::{FileId, SpotifyUri};
use librespot_metadata::audio::file::{AudioFileFormat, AudioFiles};
use librespot_metadata::{Metadata, Track as ApTrack};
use tauri::Emitter;
use tokio::io::AsyncWriteExt;

use super::auth::SpotifyClient;
use super::{bare_track_id, tags};
use crate::deezer::bandwidth::BandwidthLimiter;
use crate::deezer::models::{DownloadProgress, DownloadResult};
use crate::paths::{
    build_download_path, cleanup_temp_file_async, create_temp_download_file,
    finalize_download_file, find_existing_download, TrackPathMeta,
};
use crate::settings::FolderStructure;

const READ_CHUNK: usize = 128 * 1024;
/// Ask librespot to fetch well ahead of playback speed; we are not playing.
const FETCH_RATE: usize = 10 * 1024 * 1024;
const MAX_COVER_BYTES: usize = 20 * 1024 * 1024;
/// Formats that count as "already downloaded" at the destination.
const AUDIO_EXTENSIONS: [&str; 4] = [".ogg", ".flac", ".mp3", ".m4a"];

#[allow(clippy::too_many_arguments)]
pub async fn download_track(
    client: &SpotifyClient,
    track_key: &str,
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
    let id = bare_track_id(track_key);
    let track = client.track(id).await?;

    emit_progress(app, track_key, &track.title, 0.0, "resolving");

    // The Web API gives catalog metadata; the AP session gives the file IDs
    // and the AES key used by librespot to decrypt the stream.
    let uri = SpotifyUri::from_uri(&format!("spotify:track:{}", id))
        .map_err(|e| format!("Invalid Spotify track id: {}", e))?;
    let ap_track = ApTrack::get(&client.session, &uri)
        .await
        .map_err(|e| format!("Spotify metadata failed: {}", e))?;

    let (file_id, ext, actual_quality) = select_file(&ap_track.files, quality)?;

    let path_meta = TrackPathMeta {
        artist: track.artist.clone(),
        album: track.album.clone(),
        title: track.title.clone(),
        track_number: (track.track_number > 0).then_some(track.track_number),
        disc_number: (track.disc_number > 0).then_some(track.disc_number),
        release_date: track.release_date.clone(),
    };

    if skip_existing {
        if let Some(existing) = find_existing_download(
            output_dir,
            folder_structure,
            custom_folder_template,
            &path_meta,
            &AUDIO_EXTENSIONS,
        )
        .await?
        {
            emit_progress(app, track_key, &track.title, 100.0, "complete");
            return Ok(DownloadResult {
                file_path: existing.to_string_lossy().to_string(),
                requested_quality: requested_label(quality).to_string(),
                actual_quality: actual_quality.to_string(),
                status: "skipped".to_string(),
            });
        }
    }

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

    emit_progress(app, track_key, &track.title, 5.0, "downloading");

    let (temp_path, mut file) = create_temp_download_file(&download_path, track_key).await?;

    let mut audio = match AudioFile::open(&client.session, file_id, FETCH_RATE).await {
        Ok(audio) => audio,
        Err(e) => {
            drop(file);
            cleanup_temp_file_async(&temp_path).await;
            return Err(format!("Spotify audio fetch failed: {}", e));
        }
    };

    let total_size = audio
        .get_stream_loader_controller()
        .map(|controller| controller.len() as u64)
        .unwrap_or(0);

    let mut buffer = vec![0u8; READ_CHUNK];
    let mut downloaded = 0u64;

    loop {
        if cancel_flag.load(Ordering::Relaxed) {
            drop(file);
            cleanup_temp_file_async(&temp_path).await;
            return Ok(canceled_result(quality, &actual_quality));
        }

        // `AudioFile` is a blocking reader backed by librespot's stream loader.
        let read = tokio::task::block_in_place(|| audio.read(&mut buffer));
        let read = match read {
            Ok(0) => break,
            Ok(read) => read,
            Err(e) => {
                drop(file);
                cleanup_temp_file_async(&temp_path).await;
                return Err(format!("Spotify stream read failed: {}", e));
            }
        };

        bandwidth
            .acquire(read, speed_limit_kbps, &cancel_flag)
            .await;

        if cancel_flag.load(Ordering::Relaxed) {
            drop(file);
            cleanup_temp_file_async(&temp_path).await;
            return Ok(canceled_result(quality, &actual_quality));
        }

        if let Err(e) = file.write_all(&buffer[..read]).await {
            drop(file);
            cleanup_temp_file_async(&temp_path).await;
            return Err(e.to_string());
        }

        downloaded += read as u64;
        if total_size > 0 {
            let percent = 5.0 + ((downloaded as f64 / total_size as f64) * 85.0).min(85.0);
            emit_progress(app, track_key, &track.title, percent, "downloading");
        }
    }

    if let Err(e) = file.flush().await {
        drop(file);
        cleanup_temp_file_async(&temp_path).await;
        return Err(e.to_string());
    }
    drop(file);

    if downloaded == 0 {
        cleanup_temp_file_async(&temp_path).await;
        return Err("Spotify returned no audio".to_string());
    }

    emit_progress(app, track_key, &track.title, 92.0, "tagging");

    let cover = fetch_cover(client, &track.cover_medium).await;
    let tag_path = temp_path.clone();
    let tag_track = track.clone();
    let tag_ext = ext.to_string();
    let tag_result = tokio::task::spawn_blocking(move || {
        tags::write_tags(&tag_path, &tag_track, cover, &tag_ext)
    })
    .await
    .map_err(|e| format!("Tagging task failed: {}", e))?;

    if let Err(e) = tag_result {
        // Tagging is non-fatal, exactly like the Deezer engine.
        eprintln!("Warning: failed to write Spotify tags: {}", e);
        let _ = app.emit(
            "tag-writing-error",
            serde_json::json!({
                "track_id": track_key,
                "title": track.title,
                "error": e,
            }),
        );
    }

    if cancel_flag.load(Ordering::Relaxed) {
        cleanup_temp_file_async(&temp_path).await;
        return Ok(canceled_result(quality, &actual_quality));
    }

    let finalize_temp = temp_path.clone();
    let preferred_path = download_path.clone();
    let finalize_dir = download_dir.clone();
    let finalize_stem = base_stem.clone();
    let finalize_ext = ext.to_string();
    let finalize_cancel = cancel_flag.clone();
    let finalized = tokio::task::spawn_blocking(move || {
        finalize_download_file(
            &finalize_temp,
            &preferred_path,
            &finalize_dir,
            &finalize_stem,
            &finalize_ext,
            &finalize_cancel,
        )
    })
    .await
    .map_err(|e| format!("File finalization task failed: {}", e))?;

    let download_path = match finalized {
        Ok(path) => path,
        Err(e) => {
            cleanup_temp_file_async(&temp_path).await;
            return Err(e);
        }
    };

    emit_progress(app, track_key, &track.title, 100.0, "complete");

    Ok(DownloadResult {
        file_path: download_path.to_string_lossy().to_string(),
        requested_quality: requested_label(quality).to_string(),
        actual_quality,
        status: "complete".to_string(),
    })
}

fn canceled_result(quality: &str, actual_quality: &str) -> DownloadResult {
    DownloadResult {
        file_path: String::new(),
        requested_quality: requested_label(quality).to_string(),
        actual_quality: actual_quality.to_string(),
        status: "canceled".to_string(),
    }
}

/// Maps the settings value to the label the downloads view shows.
fn requested_label(quality: &str) -> &str {
    match quality {
        "normal" => "SPOTIFY_96",
        "high" => "SPOTIFY_160",
        "very-high" => "SPOTIFY_320",
        "lossless" => "FLAC",
        other => other,
    }
}

/// Picks the best delivered format for the requested quality and maps it to a
/// container extension and a label the UI can show.
fn select_file(
    files: &AudioFiles,
    quality: &str,
) -> Result<(FileId, &'static str, String), String> {
    use AudioFileFormat::*;

    let order: &[AudioFileFormat] = match quality {
        "lossless" => &[
            FLAC_FLAC_24BIT,
            FLAC_FLAC,
            OGG_VORBIS_320,
            MP3_320,
            OGG_VORBIS_160,
            MP3_160,
            OGG_VORBIS_96,
            MP3_96,
        ],
        "high" => &[
            OGG_VORBIS_160,
            MP3_160,
            OGG_VORBIS_96,
            MP3_96,
            OGG_VORBIS_320,
            MP3_320,
        ],
        _ => &[
            OGG_VORBIS_320,
            MP3_320,
            OGG_VORBIS_160,
            MP3_160,
            OGG_VORBIS_96,
            MP3_96,
        ],
    };

    for format in order {
        if let Some(file_id) = files.get(format) {
            return Ok((file_id.clone(), extension_for(*format), label_for(*format)));
        }
    }

    files
        .iter()
        .next()
        .map(|(format, file_id)| {
            (
                file_id.clone(),
                extension_for(*format),
                label_for(*format),
            )
        })
        .ok_or_else(|| "Spotify has no playable file for this track".to_string())
}

fn extension_for(format: AudioFileFormat) -> &'static str {
    use AudioFileFormat::*;
    match format {
        FLAC_FLAC | FLAC_FLAC_24BIT => ".flac",
        OGG_VORBIS_96 | OGG_VORBIS_160 | OGG_VORBIS_320 => ".ogg",
        MP3_96 | MP3_160 | MP3_256 | MP3_320 | MP3_160_ENC => ".mp3",
        _ => ".m4a",
    }
}

fn label_for(format: AudioFileFormat) -> String {
    use AudioFileFormat::*;
    match format {
        FLAC_FLAC | FLAC_FLAC_24BIT => "FLAC".to_string(),
        OGG_VORBIS_320 | MP3_320 | AAC_320 => "SPOTIFY_320".to_string(),
        OGG_VORBIS_160 | MP3_160 | MP3_160_ENC | AAC_160 => "SPOTIFY_160".to_string(),
        OGG_VORBIS_96 | MP3_96 => "SPOTIFY_96".to_string(),
        _ => "SPOTIFY_AAC".to_string(),
    }
}

async fn fetch_cover(client: &SpotifyClient, url: &str) -> Option<Vec<u8>> {
    if url.is_empty() {
        return None;
    }

    let response = client.http.get(url).send().await.ok()?;
    if !response.status().is_success() {
        return None;
    }
    if response
        .content_length()
        .is_some_and(|length| length > MAX_COVER_BYTES as u64)
    {
        return None;
    }

    let bytes = response.bytes().await.ok()?;
    if bytes.is_empty() || bytes.len() > MAX_COVER_BYTES {
        None
    } else {
        Some(bytes.to_vec())
    }
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
