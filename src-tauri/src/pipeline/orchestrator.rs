use tauri::{AppHandle, Emitter};

use crate::commands::convert::{convert_media, get_duration_ms};
use crate::commands::transcribe::transcribe_audio;
use crate::models::transcript::{PipelineOptions, PipelineProgress, Transcript};

/// Run the full transcription pipeline:
/// 1. Convert media to WAV (PyAV)
/// 2. Transcribe + align + diarize (whisperX, single pass)
#[tauri::command]
pub async fn run_pipeline(
    app: AppHandle,
    input_path: String,
    options: PipelineOptions,
) -> Result<Transcript, String> {
    // Step 1: Convert media to 16kHz mono WAV
    let wav_path = convert_media(app.clone(), input_path.clone()).await?;

    let duration_ms = get_duration_ms(std::path::Path::new(&wav_path))
        .await
        .unwrap_or(0);

    let source_file = std::path::Path::new(&input_path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| input_path.clone());

    // Step 2: Transcribe + align + diarize in one pass via whisperX
    let transcript = transcribe_audio(
        app.clone(),
        wav_path,
        source_file,
        duration_ms,
        options.whisper_model,
        options.language,
        options.hf_token,
    )
    .await?;

    // Done
    app.emit(
        "pipeline:progress",
        PipelineProgress {
            step: "done".to_string(),
            percent: 100,
            message: "Transcription complete!".to_string(),
        },
    )
    .map_err(|e: tauri::Error| e.to_string())?;

    Ok(transcript)
}
