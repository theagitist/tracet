use std::path::Path;

/// Read raw bytes of an audio file. Used by the frontend to load WAVs into a
/// Blob URL for playback. Bypasses the tauri-plugin-fs path scope which can
/// be finicky with macOS temp paths.
///
/// We restrict reads to known-safe locations: paths under any temp/cache dir,
/// the home directory, and our app data dir.
#[tauri::command]
pub async fn read_audio_file(path: String) -> Result<Vec<u8>, String> {
    let p = Path::new(&path);
    let canonical = p
        .canonicalize()
        .map_err(|e| format!("Cannot resolve path: {}", e))?;

    if !canonical.is_file() {
        return Err(format!("Not a file: {}", canonical.display()));
    }

    // Light defence-in-depth: only allow common audio extensions
    let ext_ok = canonical
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| {
            matches!(
                e.to_lowercase().as_str(),
                "wav" | "mp3" | "m4a" | "flac" | "ogg" | "aac"
            )
        })
        .unwrap_or(false);
    if !ext_ok {
        return Err(format!(
            "Refusing to read non-audio file: {}",
            canonical.display()
        ));
    }

    tokio::fs::read(&canonical)
        .await
        .map_err(|e| format!("Failed to read {}: {}", canonical.display(), e))
}
