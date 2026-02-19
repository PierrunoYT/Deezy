use std::io::Write;
use std::path::{Path, PathBuf};

use futures::StreamExt;
use id3::TagLike;
use serde_json::Value;
use tauri::Emitter;

use super::models::DownloadProgress;
use super::{crypto, get_quality_ext, DeezerClient};
use crate::settings::FolderStructure;

pub async fn download_track(
    client: &DeezerClient,
    track_id: &str,
    output_dir: &str,
    quality: &str,
    folder_structure: &FolderStructure,
    app: &tauri::AppHandle,
) -> Result<String, String> {
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

    // Build the directory path based on folder structure
    let base_dir = PathBuf::from(output_dir);
    let download_dir = match folder_structure {
        FolderStructure::Flat => base_dir,
        FolderStructure::ArtistTrack => {
            base_dir.join(sanitize_path_component(&artist))
        }
        FolderStructure::ArtistAlbumTrack => {
            base_dir
                .join(sanitize_path_component(&artist))
                .join(sanitize_path_component(&album_title))
        }
        FolderStructure::AlbumTrack => {
            base_dir.join(sanitize_path_component(&album_title))
        }
    };

    std::fs::create_dir_all(&download_dir).map_err(|e| e.to_string())?;

    let filename = clean_filename(&format!("{} - {}{}", artist, full_title, ext));

    // Check if file exists and create unique filename if needed
    let mut download_path = download_dir.join(&filename);
    let mut counter = 1;
    while download_path.exists() {
        let base_name = clean_filename(&format!("{} - {}", artist, full_title));
        let new_filename = format!("{} ({}){}", base_name, counter, ext);
        download_path = download_dir.join(&new_filename);
        counter += 1;

        // Prevent infinite loop
        if counter > 1000 {
            return Err("Too many files with the same name".to_string());
        }
    }

    emit_progress(app, track_id, &full_title, 5.0, "downloading");

    let response = client
        .http
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Download failed: {}", e))?;

    let total_size = response.content_length().unwrap_or(0);
    let mut stream = response.bytes_stream();
    let mut file =
        std::fs::File::create(&download_path).map_err(|e| format!("Cannot create file: {}", e))?;

    let mut buffer: Vec<u8> = Vec::new();
    let mut chunk_index = 0u64;
    let mut downloaded = 0u64;

    while let Some(item) = stream.next().await {
        let bytes = item.map_err(|e| format!("Stream error: {}", e))?;
        buffer.extend_from_slice(&bytes);

        while buffer.len() >= 2048 {
            let chunk: Vec<u8> = buffer.drain(..2048).collect();

            if chunk_index % 3 == 0 {
                let decrypted = crypto::decrypt_blowfish_chunk(&chunk, &bf_key);
                file.write_all(&decrypted).map_err(|e| e.to_string())?;
            } else {
                file.write_all(&chunk).map_err(|e| e.to_string())?;
            }

            chunk_index += 1;
            downloaded += 2048;

            if total_size > 0 {
                let percent = 5.0 + (downloaded as f64 / total_size as f64 * 85.0).min(85.0);
                emit_progress(app, track_id, &full_title, percent, "downloading");
            }
        }
    }

    if !buffer.is_empty() {
        file.write_all(&buffer).map_err(|e| e.to_string())?;
    }
    drop(file);

    emit_progress(app, track_id, &full_title, 92.0, "tagging");

    if ext == ".mp3" {
        if let Err(e) =
            write_mp3_tags(&download_path, &full_title, &artist, &album_title, track_data, client, &album_id).await
        {
            eprintln!("Warning: failed to write tags: {}", e);
            let _ = app.emit("tag-writing-error", serde_json::json!({
                "track_id": track_id,
                "title": full_title,
                "error": e.to_string()
            }));
        }
    } else if ext == ".flac" {
        if let Err(e) =
            write_flac_tags(&download_path, &full_title, &artist, &album_title, track_data, client, &album_id).await
        {
            eprintln!("Warning: failed to write FLAC tags: {}", e);
            let _ = app.emit("tag-writing-error", serde_json::json!({
                "track_id": track_id,
                "title": full_title,
                "error": e.to_string()
            }));
        }
    }

    emit_progress(app, track_id, &full_title, 100.0, "complete");

    Ok(download_path.to_string_lossy().to_string())
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

    tag.write_to_path(path, id3::Version::Id3v24)
        .map_err(|e| format!("Tag write error: {}", e))?;

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
    let mut tag =
        metaflac::Tag::read_from_path(path).map_err(|e| format!("FLAC read error: {}", e))?;

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

    tag.write_to_path(path)
        .map_err(|e| format!("FLAC tag write error: {}", e))?;

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

fn clean_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            _ => c,
        })
        .collect::<String>()
        .trim()
        .to_string()
}

fn sanitize_path_component(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            _ => c,
        })
        .collect::<String>()
        .trim()
        .trim_matches('.')
        .to_string()
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
        .or_else(|| val.as_u64().map(|n| n as u32))
}
