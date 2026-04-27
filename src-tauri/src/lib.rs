mod commands;
mod models;
mod pipeline;
mod sidecar;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
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
