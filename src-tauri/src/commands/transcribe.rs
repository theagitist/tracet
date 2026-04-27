use std::path::Path;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;

use crate::models::transcript::{
    PipelineProgress, SegmentType, Speaker, TokenConfidence, Transcript, TranscriptSegment,
};

/// Response from the whisperX Python sidecar.
#[derive(Debug, serde::Deserialize)]
struct SidecarResponse {
    segments: Option<Vec<SidecarSegment>>,
    speakers: Option<Vec<String>>,
    language: Option<String>,
    error: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct SidecarSegment {
    start: f64,
    end: f64,
    text: String,
    speaker: Option<String>,
    words: Option<Vec<SidecarWord>>,
}

#[derive(Debug, serde::Deserialize)]
struct SidecarWord {
    word: String,
    #[allow(dead_code)]
    start: f64,
    #[allow(dead_code)]
    end: f64,
    score: f64,
    speaker: Option<String>,
}

const SPEAKER_COLORS: &[&str] = &[
    "#3B82F6", "#EF4444", "#10B981", "#F59E0B", "#8B5CF6", "#EC4899", "#06B6D4", "#F97316",
];

/// Run whisperX transcription + alignment + diarization via Python sidecar.
/// Returns a complete Transcript with segments, speakers, and confidence data.
#[tauri::command]
pub async fn transcribe_audio(
    app: AppHandle,
    wav_path: String,
    source_file: String,
    duration_ms: u64,
    model_size: String,
    language: Option<String>,
    hf_token: Option<String>,
) -> Result<Transcript, String> {
    app.emit(
        "pipeline:progress",
        PipelineProgress {
            step: "transcribing".to_string(),
            percent: 0,
            message: "Starting whisperX transcription...".to_string(),
        },
    )
    .map_err(|e| e.to_string())?;

    let sidecar_dir = find_sidecar_dir()?;
    let python_bin = sidecar_dir.join(".venv").join("bin").join("python3");
    let script = sidecar_dir.join("diarize.py");

    if !python_bin.exists() {
        return Err(
            "Python environment not set up. Please run setup from Settings.".to_string(),
        );
    }

    // Spawn the whisperX sidecar
    let mut child = Command::new(python_bin.to_str().unwrap())
        .arg(script.to_str().unwrap())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to start transcription sidecar: {}", e))?;

    // Send request
    let stdin = child
        .stdin
        .as_mut()
        .ok_or("Failed to open stdin for sidecar")?;
    let request = serde_json::json!({
        "wav_path": &wav_path,
        "hf_token": hf_token,
        "model_size": model_size,
        "language": language,
    });
    stdin
        .write_all(format!("{}\n", request).as_bytes())
        .await
        .map_err(|e| format!("Failed to write to sidecar: {}", e))?;
    stdin
        .shutdown()
        .await
        .map_err(|e| format!("Failed to close sidecar stdin: {}", e))?;

    app.emit(
        "pipeline:progress",
        PipelineProgress {
            step: "transcribing".to_string(),
            percent: 30,
            message: "Starting whisperX...".to_string(),
        },
    )
    .map_err(|e| e.to_string())?;

    // Helper closure: forward an arbitrary library log line to the UI as a progress
    // message (with percent=0 meaning "keep current percent, just update text").
    let app_for_log = app.clone();
    let forward_log = move |raw: &str| {
        if let Some(msg) = extract_user_facing_message(raw) {
            let _ = app_for_log.emit(
                "pipeline:progress",
                PipelineProgress {
                    step: "transcribing".to_string(),
                    percent: 0,
                    message: msg,
                },
            );
        }
    };

    // Spawn a task to read stderr in real-time. Python emits structured progress
    // ("TRACET_PROGRESS: ...") on stderr; whisperX/pyannote/torch also emit log
    // lines and download bars there. We forward both kinds so the UI shows what
    // the underlying libraries are doing.
    let stderr = child
        .stderr
        .take()
        .ok_or("Failed to capture sidecar stderr")?;
    let app_for_stderr = app.clone();
    let stderr_buf = std::sync::Arc::new(tokio::sync::Mutex::new(String::new()));
    let stderr_buf_clone = stderr_buf.clone();
    let forward_log_stderr = forward_log.clone();

    let stderr_task = tokio::spawn(async move {
        let mut reader = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            stderr_buf_clone.lock().await.push_str(&line);
            stderr_buf_clone.lock().await.push('\n');

            if let Some(rest) = line.strip_prefix("TRACET_PROGRESS:") {
                let rest = rest.trim();
                let mut parts = rest.splitn(2, ' ');
                if let (Some(pct_str), Some(msg)) = (parts.next(), parts.next()) {
                    if let Ok(pct) = pct_str.parse::<u8>() {
                        let _ = app_for_stderr.emit(
                            "pipeline:progress",
                            PipelineProgress {
                                step: "transcribing".to_string(),
                                percent: pct,
                                message: msg.to_string(),
                            },
                        );
                    }
                }
            } else {
                forward_log_stderr(&line);
            }
            log::debug!("[sidecar stderr] {}", line);
        }
    });

    // Read stdout: most lines are noise (libraries that misbehavedly write to
    // stdout). The actual JSON result is the line prefixed with RESULT_MARKER.
    const RESULT_MARKER: &str = "###TRACET_RESULT###";
    let stdout = child
        .stdout
        .take()
        .ok_or("Failed to capture sidecar stdout")?;
    let mut reader = BufReader::new(stdout).lines();

    let mut result_line = String::new();
    while let Some(line) = reader
        .next_line()
        .await
        .map_err(|e| format!("Failed to read sidecar output: {}", e))?
    {
        if let Some(rest) = line.strip_prefix(RESULT_MARKER) {
            result_line = rest.to_string();
            break;
        }
        // Non-result stdout line: still potentially useful library logging.
        forward_log(&line);
        log::debug!("[sidecar stdout] {}", line);
    }

    let status = child
        .wait()
        .await
        .map_err(|e| format!("Sidecar process error: {}", e))?;

    // Wait for stderr task to finish so we have full error context
    let _ = stderr_task.await;
    let stderr_output = stderr_buf.lock().await.clone();

    if !status.success() {
        let tail: String = stderr_output
            .lines()
            .rev()
            .take(20)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join("\n");
        return Err(format!(
            "Transcription sidecar failed (exit code: {:?}).\n\nLast stderr lines:\n{}",
            status.code(),
            tail
        ));
    }

    if result_line.is_empty() {
        let tail: String = stderr_output
            .lines()
            .rev()
            .take(20)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join("\n");
        return Err(format!(
            "Transcription sidecar produced no output. stderr:\n{}",
            tail
        ));
    }

    let response: SidecarResponse = serde_json::from_str(&result_line)
        .map_err(|e| format!("Failed to parse sidecar response: {}. Raw: {}", e, result_line))?;

    if let Some(error) = response.error {
        return Err(format!("whisperX error: {}", error));
    }

    let raw_segments = response.segments.unwrap_or_default();
    let raw_speakers = response.speakers.unwrap_or_default();

    app.emit(
        "pipeline:progress",
        PipelineProgress {
            step: "transcribing".to_string(),
            percent: 90,
            message: format!(
                "Processed {} segments, {} speakers",
                raw_segments.len(),
                raw_speakers.len()
            ),
        },
    )
    .map_err(|e| e.to_string())?;

    // Convert to our Transcript format
    let segments: Vec<TranscriptSegment> = raw_segments
        .into_iter()
        .map(|seg| {
            let words = seg.words.unwrap_or_default();

            let token_confidences: Vec<TokenConfidence> = words
                .iter()
                .map(|w| TokenConfidence {
                    token: w.word.clone(),
                    confidence: w.score as f32,
                })
                .collect();

            let avg_confidence = if token_confidences.is_empty() {
                0.5
            } else {
                token_confidences.iter().map(|t| t.confidence).sum::<f32>()
                    / token_confidences.len() as f32
            };

            // Determine segment type based on text content
            let segment_type = classify_segment(&seg.text);

            // Use word-level speaker if available, fall back to segment-level
            let speaker_id = words
                .first()
                .and_then(|w| w.speaker.clone())
                .or(seg.speaker.clone());

            // Auto-flag segments that look likely to be incorrect, even when
            // LLM review is disabled. We use heuristics over the word-level
            // confidence scores from whisperX.
            let very_low_count = token_confidences
                .iter()
                .filter(|t| t.confidence < 0.3)
                .count();
            let zero_score_count = token_confidences
                .iter()
                .filter(|t| t.confidence < 0.05)
                .count();

            let (is_flagged, flag_reason) = if avg_confidence < 0.55 {
                (
                    true,
                    Some(format!(
                        "Low average confidence ({:.0}%)",
                        avg_confidence * 100.0
                    )),
                )
            } else if zero_score_count >= 2 {
                (
                    true,
                    Some(format!(
                        "{} words with near-zero alignment score",
                        zero_score_count
                    )),
                )
            } else if very_low_count > 0 && token_confidences.len() < 12 {
                // Short segments with even one very-low-confidence word are suspicious.
                (
                    true,
                    Some(format!(
                        "{} word(s) with very low confidence",
                        very_low_count
                    )),
                )
            } else if very_low_count >= 3 {
                (
                    true,
                    Some(format!(
                        "{} words with very low confidence",
                        very_low_count
                    )),
                )
            } else {
                (false, None)
            };

            TranscriptSegment {
                id: uuid::Uuid::new_v4().to_string(),
                start_ms: (seg.start * 1000.0) as u64,
                end_ms: (seg.end * 1000.0) as u64,
                text: seg.text,
                speaker_id,
                speaker_name: None,
                confidence: avg_confidence,
                token_confidences,
                is_flagged,
                flag_reason,
                segment_type,
            }
        })
        .collect();

    let speakers: Vec<Speaker> = raw_speakers
        .into_iter()
        .enumerate()
        .map(|(i, id)| Speaker {
            id,
            name: None,
            color: SPEAKER_COLORS[i % SPEAKER_COLORS.len()].to_string(),
        })
        .collect();

    app.emit(
        "pipeline:progress",
        PipelineProgress {
            step: "transcribing".to_string(),
            percent: 100,
            message: "Transcription complete".to_string(),
        },
    )
    .map_err(|e| e.to_string())?;

    Ok(Transcript {
        id: uuid::Uuid::new_v4().to_string(),
        source_file,
        audio_path: wav_path,
        segments,
        speakers,
        created_at: chrono::Utc::now().to_rfc3339(),
        duration_ms,
    })
}

/// Classify a segment as speech or non-speech based on text patterns.
fn classify_segment(text: &str) -> SegmentType {
    let lower = text.trim().to_lowercase();
    if lower.is_empty() {
        return SegmentType::Silence;
    }
    // whisperX may produce these markers
    if lower.contains("[music]") || lower.contains("♪") {
        return SegmentType::Music;
    }
    if lower.contains("[noise]") || lower.contains("[background")  {
        return SegmentType::Noise;
    }
    if lower.contains("[silence]") || lower.contains("[blank_audio]") {
        return SegmentType::Silence;
    }
    SegmentType::Speech
}

/// Extract a user-facing progress message from an arbitrary stdout/stderr line.
/// Returns None if the line is just noise (tracebacks, tqdm bars, etc.).
fn extract_user_facing_message(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }

    // Skip tqdm/torch download progress bars: they're already noisy and we
    // surface the "Downloading" event separately.
    if trimmed.contains("\r") || trimmed.starts_with('|') || trimmed.contains("MB/s") {
        return None;
    }
    // Skip Python tracebacks
    if trimmed.starts_with("Traceback")
        || trimmed.starts_with("  File \"")
        || trimmed.starts_with("    ")
    {
        return None;
    }

    // Standard Python logging format:
    //   "2026-04-26 13:55:12 - whisperx.asr - INFO - <message>"
    //   "2026-04-26 13:55:12 - pyannote.audio - WARNING - <message>"
    if let Some(idx) = trimmed.find(" - INFO - ") {
        return Some(simplify_log_message(&trimmed[idx + " - INFO - ".len()..]));
    }
    if let Some(idx) = trimmed.find(" - WARNING - ") {
        let msg = &trimmed[idx + " - WARNING - ".len()..];
        // Filter out the noisy Lightning checkpoint upgrade nag
        if msg.contains("Lightning automatically upgraded") {
            return None;
        }
        return Some(simplify_log_message(msg));
    }

    if trimmed.contains("Downloading") && trimmed.contains("http") {
        return Some("Downloading model (first run only)".to_string());
    }
    None
}

/// Convert a verbose library log message into something concise for the UI.
fn simplify_log_message(msg: &str) -> String {
    if msg.contains("voice activity detection") {
        return "Running voice activity detection".to_string();
    }
    if msg.contains("language will be detected") {
        return "Auto-detecting language".to_string();
    }
    // Truncate very long messages
    let s = msg.trim();
    if s.len() > 100 {
        format!("{}…", &s[..100])
    } else {
        s.to_string()
    }
}

fn find_sidecar_dir() -> Result<std::path::PathBuf, String> {
    let dev_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("sidecar");
    if dev_path.exists() {
        return Ok(dev_path);
    }
    Err("Could not locate sidecar directory".to_string())
}
