use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use zip::write::SimpleFileOptions;
use zip::CompressionMethod;

use crate::models::transcript::Transcript;

const PROJECT_VERSION: u32 = 1;
const TRANSCRIPT_FILENAME: &str = "transcript.json";
const AUDIO_FILENAME: &str = "audio.wav";
const METADATA_FILENAME: &str = "metadata.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProjectMetadata {
    /// Bumped if/when the on-disk format changes incompatibly.
    version: u32,
    app_name: String,
    app_version: String,
    /// ISO-8601 UTC timestamp when the project was saved.
    saved_at: String,
}

/// Save a project (.tracet bundle): transcript + converted audio + metadata,
/// packaged as a zip archive.
#[tauri::command]
pub async fn save_project(transcript: Transcript, output_path: String) -> Result<(), String> {
    let audio_path = PathBuf::from(&transcript.audio_path);
    if !audio_path.exists() {
        return Err(format!(
            "Audio file no longer exists at {}: re-run transcription before saving",
            audio_path.display()
        ));
    }

    // Run the (synchronous) zip work in a blocking task so we don't tie up
    // the async runtime when the audio file is large.
    let transcript_for_save = transcript.clone();
    let output = output_path.clone();
    tokio::task::spawn_blocking(move || write_project(&transcript_for_save, &output))
        .await
        .map_err(|e| format!("Background task failed: {}", e))?
}

fn write_project(transcript: &Transcript, output_path: &str) -> Result<(), String> {
    let file = File::create(output_path)
        .map_err(|e| format!("Failed to create project file: {}", e))?;
    let mut zip = zip::ZipWriter::new(file);

    // Use Stored (no compression) for the audio: WAV barely compresses and
    // skipping deflate makes save/load much faster on large files.
    let stored = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    let deflated = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

    // We rewrite audio_path inside the bundle to a relative path so the
    // saved JSON is portable across machines.
    let mut bundled_transcript = transcript.clone();
    bundled_transcript.audio_path = AUDIO_FILENAME.to_string();
    let transcript_json = serde_json::to_string_pretty(&bundled_transcript)
        .map_err(|e| format!("Failed to serialise transcript: {}", e))?;

    zip.start_file(TRANSCRIPT_FILENAME, deflated)
        .map_err(|e| format!("Failed to add transcript: {}", e))?;
    zip.write_all(transcript_json.as_bytes())
        .map_err(|e| format!("Failed to write transcript: {}", e))?;

    let metadata = ProjectMetadata {
        version: PROJECT_VERSION,
        app_name: "Tracet".to_string(),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        saved_at: chrono::Utc::now().to_rfc3339(),
    };
    let metadata_json = serde_json::to_string_pretty(&metadata)
        .map_err(|e| format!("Failed to serialise metadata: {}", e))?;
    zip.start_file(METADATA_FILENAME, deflated)
        .map_err(|e| format!("Failed to add metadata: {}", e))?;
    zip.write_all(metadata_json.as_bytes())
        .map_err(|e| format!("Failed to write metadata: {}", e))?;

    let mut audio_file = File::open(&transcript.audio_path)
        .map_err(|e| format!("Failed to open audio: {}", e))?;
    zip.start_file(AUDIO_FILENAME, stored)
        .map_err(|e| format!("Failed to add audio: {}", e))?;
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = audio_file
            .read(&mut buf)
            .map_err(|e| format!("Failed to read audio: {}", e))?;
        if n == 0 {
            break;
        }
        zip.write_all(&buf[..n])
            .map_err(|e| format!("Failed to write audio: {}", e))?;
    }

    zip.finish().map_err(|e| format!("Failed to finalise zip: {}", e))?;
    Ok(())
}

/// Load a .tracet bundle: extracts the audio to a stable temp dir and returns
/// the transcript with audio_path pointing at the extracted file.
#[tauri::command]
pub async fn load_project(input_path: String) -> Result<Transcript, String> {
    tokio::task::spawn_blocking(move || read_project(&input_path))
        .await
        .map_err(|e| format!("Background task failed: {}", e))?
}

fn read_project(input_path: &str) -> Result<Transcript, String> {
    let file = File::open(input_path)
        .map_err(|e| format!("Failed to open project file: {}", e))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| format!("Not a valid .tracet file: {}", e))?;

    // Validate metadata version
    if let Ok(mut entry) = archive.by_name(METADATA_FILENAME) {
        let mut buf = String::new();
        entry
            .read_to_string(&mut buf)
            .map_err(|e| format!("Failed to read metadata: {}", e))?;
        let meta: ProjectMetadata = serde_json::from_str(&buf)
            .map_err(|e| format!("Invalid metadata: {}", e))?;
        if meta.version > PROJECT_VERSION {
            return Err(format!(
                "Project file is from a newer version (v{}). Update Tracet to open it.",
                meta.version
            ));
        }
    }

    // Read transcript JSON
    let mut transcript: Transcript = {
        let mut entry = archive
            .by_name(TRANSCRIPT_FILENAME)
            .map_err(|e| format!("Project missing transcript.json: {}", e))?;
        let mut buf = String::new();
        entry
            .read_to_string(&mut buf)
            .map_err(|e| format!("Failed to read transcript: {}", e))?;
        serde_json::from_str(&buf).map_err(|e| format!("Invalid transcript JSON: {}", e))?
    };

    // Extract audio to a stable per-project temp dir keyed by transcript.id so
    // multiple projects don't trample each other.
    let audio_dir = dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("com.tracet.app")
        .join("projects")
        .join(&transcript.id);
    std::fs::create_dir_all(&audio_dir)
        .map_err(|e| format!("Failed to create cache dir: {}", e))?;
    let audio_out = audio_dir.join(AUDIO_FILENAME);

    {
        let mut entry = archive
            .by_name(AUDIO_FILENAME)
            .map_err(|e| format!("Project missing audio.wav: {}", e))?;
        let mut out = File::create(&audio_out)
            .map_err(|e| format!("Failed to create audio file: {}", e))?;
        std::io::copy(&mut entry, &mut out)
            .map_err(|e| format!("Failed to extract audio: {}", e))?;
    }

    transcript.audio_path = audio_out.to_string_lossy().to_string();
    Ok(transcript)
}

// Allow access to PathBuf in extract path even when used only in path-builder
#[allow(dead_code)]
fn _path_marker(_p: &Path) {}
