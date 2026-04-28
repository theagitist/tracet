mod commands;
mod models;
mod pipeline;
mod sidecar;

/// When launched from Finder/launchd, the app inherits a minimal PATH
/// (`/usr/bin:/bin:/usr/sbin:/sbin`) without Homebrew or user-local
/// directories. Tools like `ffmpeg`, `brew`, `python3` (when installed
/// via Homebrew), and `ollama` then look missing even though they work
/// fine from a terminal. Prepend the well-known macOS bin dirs so every
/// subprocess we spawn can find them.
fn augment_path_for_gui_launch() {
    let extras = [
        "/opt/homebrew/bin",
        "/opt/homebrew/sbin",
        "/usr/local/bin",
        "/usr/local/sbin",
    ];
    let home_local = std::env::var_os("HOME").map(|h| {
        let mut p = std::path::PathBuf::from(h);
        p.push(".local/bin");
        p
    });

    let current = std::env::var_os("PATH").unwrap_or_default();
    let mut parts: Vec<std::path::PathBuf> = std::env::split_paths(&current).collect();

    for extra in extras.iter().rev() {
        let p = std::path::PathBuf::from(extra);
        if !parts.contains(&p) {
            parts.insert(0, p);
        }
    }
    if let Some(p) = home_local {
        if !parts.contains(&p) {
            parts.push(p);
        }
    }

    if let Ok(joined) = std::env::join_paths(parts) {
        std::env::set_var("PATH", joined);
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    augment_path_for_gui_launch();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            commands::audio::read_audio_file,
            commands::convert::convert_media,
            commands::transcribe::transcribe_audio,
            commands::review::review_transcript,
            commands::export::export_transcript,
            commands::hardware::detect_hardware,
            commands::hardware::get_available_profiles,
            commands::setup::check_setup_status,
            commands::setup::run_setup,
            commands::setup::install_ollama,
            commands::project::save_project,
            commands::project::load_project,
            pipeline::orchestrator::run_pipeline,
            sidecar::python::setup_python_sidecar,
            sidecar::python::check_python_sidecar,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Tracet");
}
