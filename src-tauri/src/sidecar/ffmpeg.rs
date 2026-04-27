use std::path::Path;

/// Check if FFmpeg is available on the system PATH.
pub fn is_ffmpeg_available() -> bool {
    std::process::Command::new("ffmpeg")
        .arg("-version")
        .output()
        .is_ok()
}

/// Get the list of supported media extensions.
pub fn supported_extensions() -> Vec<&'static str> {
    vec![
        "mp3", "wav", "flac", "m4a", "ogg", "wma", "aac", // Audio
        "mp4", "mkv", "avi", "webm", "mov", "wmv", "flv", // Video
    ]
}

/// Check if a file is a supported media format.
pub fn is_supported_media(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|ext| supported_extensions().contains(&ext.to_lowercase().as_str()))
}
