use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use futures::StreamExt;
use librespot_audio::{AudioDecrypt, AudioFile};
use librespot_core::{FileId, SpotifyId};
use librespot_metadata::audio::file::{AudioFileFormat, AudioFiles};
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
const COVER_TIMEOUT: Duration = Duration::from_secs(30);
/// Length of the header Spotify puts in front of the audio (librespot's
/// `SPOTIFY_OGG_HEADER_END`).
const SPOTIFY_HEADER_LEN: usize = 0xa7;
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
    crate::logging::info("spotify", format!("download start: {}", track_key));
    // One AP request gives both the catalog metadata and the file IDs; the AP
    // session also holds the AES key librespot uses to decrypt the stream.
    let (track, ap_track) = super::ap::track_with_files(&client.session, id)
        .await
        .map_err(|e| {
            crate::logging::warn("spotify", format!("track metadata failed: {}", e));
            e
        })?;
    crate::logging::debug(
        "spotify",
        format!("track metadata ok: {}", crate::logging::detail(format!("{} - {}", track.artist, track.title))),
    );

    emit_progress(app, track_key, &track.title, 0.0, "resolving");

    crate::logging::debug("spotify", format!(
        "available formats: {:?}",
        ap_track.files.keys().copied().collect::<Vec<_>>()
    ));

    let (file_id, ext, actual_quality) =
        select_file(&ap_track.files, quality).map_err(|e| {
            crate::logging::warn("spotify", format!("select_file failed: {}", e));
            e
        })?;
    crate::logging::debug("spotify", format!("selected {} ({}) for quality '{}'", actual_quality, ext, quality));

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

    // The CDN stream is AES-CTR encrypted; the key comes from the AP session.
    let spotify_id = SpotifyId::from_base62(id)
        .map_err(|e| format!("Invalid Spotify track id: {:?}", e))?;
    let audio_key = client
        .session
        .audio_key()
        .request(spotify_id, file_id)
        .await
        .map_err(|e| {
            crate::logging::warn("spotify", format!("audio key request failed: {:?}", e));
            format!("Spotify audio key failed: {}", e)
        })?;
    crate::logging::debug("spotify", "audio key received");

    // The guard removes the temp file on any early return or panic until the
    // file is published.
    let (mut temp_guard, mut file) = create_temp_download_file(&download_path, track_key).await?;
    let temp_path = temp_guard.path().to_path_buf();
    crate::logging::debug(
        "spotify",
        format!("opening audio file, temp: {}", crate::logging::detail(temp_path.display())),
    );

    let encrypted = match AudioFile::open(&client.session, file_id, FETCH_RATE).await {
        Ok(audio) => audio,
        Err(e) => {
            crate::logging::warn("spotify", format!("AudioFile::open failed: {:?}", e));
            drop(file);
            cleanup_temp_file_async(&temp_path).await;
            return Err(format!("Spotify audio fetch failed: {}", e));
        }
    };

    let total_size = encrypted
        .get_stream_loader_controller()
        .map(|controller| controller.len() as u64)
        .unwrap_or(0);
    crate::logging::debug("spotify", format!("audio opened, total size: {} bytes", total_size));
    let mut audio = AudioDecrypt::new(Some(audio_key), encrypted);

    let mut buffer = vec![0u8; READ_CHUNK];
    let mut downloaded = 0u64;

    // Spotify prepends a short non-standard header to the decrypted stream
    // (librespot skips the same number of bytes before decoding). Read enough
    // to find where the real audio starts, then write from there.
    // Eight bytes past the header are enough to recognise every format,
    // including MP4's "ftyp" at offset 4.
    let mut head = vec![0u8; SPOTIFY_HEADER_LEN + 8];
    if let Err(e) = tokio::task::block_in_place(|| audio.read_exact(&mut head)) {
        crate::logging::warn("spotify", format!("stream header read failed: {:?}", e));
        drop(file);
        cleanup_temp_file_async(&temp_path).await;
        return Err(format!("Spotify stream read failed: {}", e));
    }
    let start = audio_start(&head, ext);
    crate::logging::debug("spotify", format!(
        "decrypted head: {:02x?}, audio starts at byte {}",
        &head[..8],
        start
    ));
    // A wrong key or an unexpected container would otherwise be saved and
    // reported as a finished download.
    if !looks_like(ext, &head[start..]) {
        crate::logging::warn(
            "spotify",
            format!("decrypted stream is not {} audio ({})", ext, actual_quality),
        );
        drop(file);
        cleanup_temp_file_async(&temp_path).await;
        return Err(format!(
            "Spotify stream is not valid {} audio; the file could not be decrypted",
            ext.trim_start_matches('.')
        ));
    }
    if let Err(e) = file.write_all(&head[start..]).await {
        drop(file);
        cleanup_temp_file_async(&temp_path).await;
        return Err(e.to_string());
    }
    downloaded += (head.len() - start) as u64;

    loop {
        if cancel_flag.load(Ordering::Relaxed) {
            drop(file);
            cleanup_temp_file_async(&temp_path).await;
            return Ok(canceled_result(quality, &actual_quality));
        }

        // `AudioDecrypt` wraps a blocking `AudioFile` backed by librespot's stream loader.
        let read = tokio::task::block_in_place(|| audio.read(&mut buffer));
        let read = match read {
            Ok(0) => break,
            Ok(read) => read,
            Err(e) => {
                crate::logging::warn("spotify", format!("stream read failed after {} bytes: {:?}", downloaded, e));
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

    crate::logging::info("spotify", format!("stream complete: {} bytes, tagging", downloaded));
    emit_progress(app, track_key, &track.title, 92.0, "tagging");

    let cover = fetch_cover(client, &track.cover_medium).await;
    let tag_path = temp_path.clone();
    let tag_track = track.clone();
    let tag_ext = ext.to_string();
    // A panicking tagger (lofty on malformed audio) is a tag failure like any
    // other: the audio is intact.
    let tag_result = tokio::task::spawn_blocking(move || {
        tags::write_tags(&tag_path, &tag_track, cover, &tag_ext)
    })
    .await
    .unwrap_or_else(|e| Err(format!("Tagging task failed: {}", e)));

    if let Err(e) = tag_result {
        // Tagging is non-fatal, exactly like the Deezer engine.
        crate::logging::warn("spotify", format!("Tags not written: {}", e));
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
    let finalized = match tokio::task::spawn_blocking(move || {
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
    {
        Ok(result) => result,
        Err(e) => Err(format!("File finalization task failed: {}", e)),
    };

    let download_path = match finalized {
        Ok(path) => path,
        Err(e) => {
            cleanup_temp_file_async(&temp_path).await;
            return Err(e);
        }
    };
    temp_guard.disarm();

    crate::logging::info(
        "spotify",
        format!("download complete: {}", crate::logging::detail(download_path.display())),
    );
    emit_progress(app, track_key, &track.title, 100.0, "complete");

    Ok(DownloadResult {
        file_path: download_path.to_string_lossy().to_string(),
        requested_quality: requested_label(quality).to_string(),
        actual_quality,
        status: "complete".to_string(),
    })
}

/// Whether `bytes` begins like an OGG, FLAC, MP3 or MP4 stream.
fn is_audio_magic(bytes: &[u8]) -> bool {
    bytes.starts_with(b"OggS")
        || bytes.starts_with(b"fLaC")
        || bytes.starts_with(b"ID3")
        || (bytes.len() >= 2 && bytes[0] == 0xff && bytes[1] & 0xe0 == 0xe0)
        || (bytes.len() >= 8 && &bytes[4..8] == b"ftyp")
}

/// Whether `bytes` begins like audio of the container `ext` names.
fn looks_like(ext: &str, bytes: &[u8]) -> bool {
    match ext {
        ".ogg" => bytes.starts_with(b"OggS"),
        ".flac" => bytes.starts_with(b"fLaC"),
        ".mp3" => {
            bytes.starts_with(b"ID3") || (bytes.len() >= 2 && bytes[0] == 0xff && bytes[1] & 0xe0 == 0xe0)
        }
        ".m4a" => bytes.len() >= 8 && &bytes[4..8] == b"ftyp",
        _ => false,
    }
}

/// Where the real audio begins in `head` (the first bytes of the decrypted
/// stream). OGG files carry Spotify's header first, so that offset is tried
/// first there; other formats are tried at 0 first. Falls back by extension
/// when neither offset looks like audio.
fn audio_start(head: &[u8], ext: &str) -> usize {
    let after_header = head
        .get(SPOTIFY_HEADER_LEN..)
        .is_some_and(is_audio_magic);
    let at_start = is_audio_magic(head);

    if ext == ".ogg" {
        if after_header {
            SPOTIFY_HEADER_LEN
        } else if at_start {
            0
        } else {
            SPOTIFY_HEADER_LEN
        }
    } else if at_start {
        0
    } else if after_header {
        SPOTIFY_HEADER_LEN
    } else {
        0
    }
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

/// Formats to try, best match first, for a settings value. Each setting starts
/// at its own bitrate (`normal` 96, `high` 160, `very-high` 320) and only falls
/// back to the others when Spotify does not deliver that one.
fn format_order(quality: &str) -> &'static [AudioFileFormat] {
    use AudioFileFormat::*;

    match quality {
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
        "normal" => &[
            OGG_VORBIS_96,
            MP3_96,
            OGG_VORBIS_160,
            MP3_160,
            OGG_VORBIS_320,
            MP3_320,
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
    }
}

/// Formats tried, in this order, when none of the quality setting's own
/// formats exist. Each one can be named and tagged. `MP3_160_ENC` (an extra
/// encryption layer) and `OTHER5` (unknown) are never selected.
const FALLBACK_FORMATS: [AudioFileFormat; 11] = [
    AudioFileFormat::MP3_256,
    AudioFileFormat::AAC_320,
    AudioFileFormat::AAC_160,
    AudioFileFormat::MP4_128,
    AudioFileFormat::AAC_48,
    AudioFileFormat::AAC_24,
    AudioFileFormat::FLAC_FLAC,
    AudioFileFormat::FLAC_FLAC_24BIT,
    AudioFileFormat::XHE_AAC_24,
    AudioFileFormat::XHE_AAC_16,
    AudioFileFormat::XHE_AAC_12,
];

/// Picks the best delivered format for the requested quality and maps it to a
/// container extension and a label the UI can show. The choice never depends
/// on `AudioFiles`' (hash map) iteration order.
fn select_file(
    files: &AudioFiles,
    quality: &str,
) -> Result<(FileId, &'static str, String), String> {
    format_order(quality)
        .iter()
        .chain(FALLBACK_FORMATS.iter())
        .find_map(|format| {
            files
                .get(format)
                .map(|file_id| (*file_id, extension_for(*format), label_for(*format)))
        })
        .ok_or_else(|| "Spotify has no supported file for this track".to_string())
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

    // Bounded, so a stalled server cannot hold the download at "tagging".
    let response = client
        .http
        .get(url)
        .timeout(COVER_TIMEOUT)
        .send()
        .await
        .ok()?;
    if !response.status().is_success() {
        return None;
    }
    if response
        .content_length()
        .is_some_and(|length| length > MAX_COVER_BYTES as u64)
    {
        return None;
    }

    // Read in chunks and stop at the cap: a body without a Content-Length
    // could otherwise be any size.
    let mut bytes = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.ok()?;
        if chunk.len() > MAX_COVER_BYTES.saturating_sub(bytes.len()) {
            return None;
        }
        bytes.extend_from_slice(&chunk);
    }
    (!bytes.is_empty()).then_some(bytes)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn head_with(at: usize, magic: &[u8]) -> Vec<u8> {
        let mut head = vec![0x11u8; SPOTIFY_HEADER_LEN + 4];
        head[at..at + magic.len()].copy_from_slice(magic);
        head
    }

    #[test]
    fn ogg_skips_the_spotify_header() {
        let mut head = head_with(0, b"OggS");
        head[SPOTIFY_HEADER_LEN..].copy_from_slice(b"OggS");
        assert_eq!(audio_start(&head, ".ogg"), SPOTIFY_HEADER_LEN);
    }

    #[test]
    fn ogg_without_the_header_starts_at_zero() {
        assert_eq!(audio_start(&head_with(0, b"OggS"), ".ogg"), 0);
    }

    #[test]
    fn unrecognised_ogg_falls_back_to_the_header_length() {
        assert_eq!(audio_start(&[0x11u8; SPOTIFY_HEADER_LEN + 4], ".ogg"), SPOTIFY_HEADER_LEN);
    }

    #[test]
    fn other_formats_prefer_zero_and_then_the_header() {
        assert_eq!(audio_start(&head_with(0, b"fLaC"), ".flac"), 0);
        assert_eq!(audio_start(&head_with(0, b"ID3\x04"), ".mp3"), 0);
        let mut head = vec![0x11u8; SPOTIFY_HEADER_LEN + 4];
        head[SPOTIFY_HEADER_LEN..].copy_from_slice(b"fLaC");
        assert_eq!(audio_start(&head, ".flac"), SPOTIFY_HEADER_LEN);
        assert_eq!(audio_start(&[0x11u8; SPOTIFY_HEADER_LEN + 4], ".m4a"), 0);
    }

    #[test]
    fn each_quality_setting_starts_at_its_own_bitrate() {
        use AudioFileFormat::*;
        assert!(matches!(format_order("normal")[0], OGG_VORBIS_96));
        assert!(matches!(format_order("high")[0], OGG_VORBIS_160));
        assert!(matches!(format_order("very-high")[0], OGG_VORBIS_320));
        assert!(matches!(format_order("lossless")[0], FLAC_FLAC_24BIT));
        // Unknown values behave like the default (very-high).
        assert!(matches!(format_order("anything")[0], OGG_VORBIS_320));
    }

    fn files(formats: &[AudioFileFormat]) -> AudioFiles {
        AudioFiles(
            formats
                .iter()
                .enumerate()
                .map(|(i, format)| (*format, FileId([i as u8; 20])))
                .collect(),
        )
    }

    #[test]
    fn select_file_falls_back_in_a_fixed_order() {
        use AudioFileFormat::*;
        let available = files(&[OTHER5, AAC_24, FLAC_FLAC, MP3_256, MP3_160_ENC]);
        // Run several times: a hash map's order would make this flaky.
        for _ in 0..20 {
            let (file_id, ext, label) = select_file(&available, "very-high").unwrap();
            assert_eq!(file_id, available[&MP3_256]);
            assert_eq!(ext, ".mp3");
            assert!(!label.is_empty());
        }

        let (_, ext, _) = select_file(&files(&[AAC_24, FLAC_FLAC]), "normal").unwrap();
        assert_eq!(ext, ".m4a");
        let (_, ext, _) = select_file(&files(&[MP3_96, FLAC_FLAC]), "lossless").unwrap();
        assert_eq!(ext, ".flac");
        let (_, ext, _) = select_file(&files(&[MP3_96, FLAC_FLAC]), "high").unwrap();
        assert_eq!(ext, ".mp3");
    }

    #[test]
    fn select_file_refuses_formats_it_cannot_handle() {
        use AudioFileFormat::*;
        assert!(select_file(&files(&[OTHER5, MP3_160_ENC]), "very-high").is_err());
        assert!(select_file(&files(&[]), "very-high").is_err());
    }

    #[test]
    fn decrypted_audio_must_match_the_extension() {
        assert!(looks_like(".ogg", b"OggS\x00\x02\x00\x00"));
        assert!(looks_like(".flac", b"fLaC\x00\x00\x00\x22"));
        assert!(looks_like(".mp3", b"ID3\x04\x00\x00\x00\x00"));
        assert!(looks_like(".mp3", &[0xff, 0xfb, 0x90, 0x00]));
        assert!(looks_like(".m4a", b"\x00\x00\x00\x20ftypM4A "));

        assert!(!looks_like(".ogg", b"fLaC\x00\x00\x00\x22"));
        assert!(!looks_like(".m4a", b"\x00\x00\x00\x20"));
        assert!(!looks_like(".flac", &[0x1a, 0xec, 0xe8, 0x3b]));
        assert!(!looks_like(".wav", b"RIFF\x00\x00\x00\x00"));

        // An undecryptable OGG falls back to the header offset, which then fails the check.
        let garbage = [0x11u8; SPOTIFY_HEADER_LEN + 8];
        let start = audio_start(&garbage, ".ogg");
        assert!(!looks_like(".ogg", &garbage[start..]));
    }

    #[test]
    fn recognises_mp4_and_mp3_frames() {
        assert!(is_audio_magic(b"\x00\x00\x00\x20ftypM4A "));
        assert!(is_audio_magic(&[0xff, 0xfb, 0x90, 0x00]));
        assert!(!is_audio_magic(&[0x1a, 0xec, 0xe8, 0x3b]));
    }
}
