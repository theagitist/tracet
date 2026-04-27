use tauri::{AppHandle, Emitter};

use crate::models::transcript::{PipelineProgress, TranscriptSegment};

/// Review transcript segments for accuracy using Ollama.
///
/// Sends batches of segments to a local Ollama instance and asks the LLM
/// to identify likely transcription errors, incoherent phrases, or
/// misheard words.
#[tauri::command]
pub async fn review_transcript(
    app: AppHandle,
    mut segments: Vec<TranscriptSegment>,
    ollama_model: String,
    ollama_url: Option<String>,
) -> Result<Vec<TranscriptSegment>, String> {
    let base_url = ollama_url.unwrap_or_else(|| "http://localhost:11434".to_string());

    // Check if Ollama is running
    let client = reqwest::Client::new();
    if client.get(&base_url).send().await.is_err() {
        // Ollama not running: fall back to confidence-only flagging
        log::warn!("Ollama not reachable at {}, falling back to confidence-based review", base_url);
        return Ok(flag_by_confidence(segments));
    }

    app.emit(
        "pipeline:progress",
        PipelineProgress {
            step: "reviewing".to_string(),
            percent: 0,
            message: format!("Reviewing with {} via Ollama...", ollama_model),
        },
    )
    .map_err(|e| e.to_string())?;

    // Process in batches of ~20 segments to fit context windows
    let batch_size = 20;
    let total_batches = (segments.len() + batch_size - 1) / batch_size;

    for (batch_idx, chunk) in segments.chunks_mut(batch_size).enumerate() {
        let percent = ((batch_idx as f32 / total_batches as f32) * 100.0) as u8;
        let _ = app.emit(
            "pipeline:progress",
            PipelineProgress {
                step: "reviewing".to_string(),
                percent,
                message: format!("Reviewing batch {} of {}", batch_idx + 1, total_batches),
            },
        );

        // Build the prompt with segment text
        let mut prompt = String::from(
            "You are a transcription reviewer. Below are numbered transcript segments. \
             Identify any that appear to contain transcription errors: misheard words, \
             incoherent phrases, grammatically impossible sentences, or words that don't \
             make sense in context.\n\n\
             For each problematic segment, respond with ONLY a JSON array like:\n\
             [{\"index\": 0, \"reason\": \"brief explanation\"}]\n\n\
             If all segments look fine, respond with: []\n\n\
             Segments:\n",
        );

        for (i, seg) in chunk.iter().enumerate() {
            prompt.push_str(&format!("[{}] {}\n", i, seg.text));
        }

        // Call Ollama API
        let response = client
            .post(format!("{}/api/generate", base_url))
            .json(&serde_json::json!({
                "model": ollama_model,
                "prompt": prompt,
                "stream": false,
                "options": {
                    "temperature": 0.1,
                    "num_predict": 1024,
                }
            }))
            .send()
            .await
            .map_err(|e| format!("Ollama request failed: {}", e))?;

        if !response.status().is_success() {
            log::warn!("Ollama returned status {}, skipping batch", response.status());
            continue;
        }

        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse Ollama response: {}", e))?;

        let response_text = body["response"].as_str().unwrap_or("[]");

        // Parse the LLM's flagged segments
        if let Some(flags) = parse_review_flags(response_text) {
            for flag in flags {
                if let Some(seg) = chunk.get_mut(flag.index) {
                    seg.is_flagged = true;
                    seg.flag_reason = Some(flag.reason);
                }
            }
        }
    }

    // Also flag low-confidence segments that the LLM might have missed
    for seg in &mut segments {
        if !seg.is_flagged && seg.confidence < 0.5 {
            seg.is_flagged = true;
            seg.flag_reason =
                Some(format!("Low confidence: {:.0}%", seg.confidence * 100.0));
        }
    }

    app.emit(
        "pipeline:progress",
        PipelineProgress {
            step: "reviewing".to_string(),
            percent: 100,
            message: "Review complete".to_string(),
        },
    )
    .map_err(|e| e.to_string())?;

    Ok(segments)
}

#[derive(Debug)]
struct ReviewFlag {
    index: usize,
    reason: String,
}

/// Try to parse the LLM's response as a JSON array of flags.
fn parse_review_flags(text: &str) -> Option<Vec<ReviewFlag>> {
    // Find JSON array in the response (LLM might include extra text)
    let start = text.find('[')?;
    let end = text.rfind(']')? + 1;
    let json_str = &text[start..end];

    let arr: Vec<serde_json::Value> = serde_json::from_str(json_str).ok()?;

    Some(
        arr.iter()
            .filter_map(|v| {
                let index = v["index"].as_u64()? as usize;
                let reason = v["reason"].as_str()?.to_string();
                Some(ReviewFlag { index, reason })
            })
            .collect(),
    )
}

/// Fallback: flag segments based on confidence scores only.
fn flag_by_confidence(mut segments: Vec<TranscriptSegment>) -> Vec<TranscriptSegment> {
    for seg in &mut segments {
        if seg.confidence < 0.5 {
            seg.is_flagged = true;
            seg.flag_reason =
                Some(format!("Low confidence: {:.0}%", seg.confidence * 100.0));
        }

        let has_very_low_token = seg.token_confidences.iter().any(|t| t.confidence < 0.3);
        if has_very_low_token && !seg.is_flagged {
            seg.is_flagged = true;
            seg.flag_reason = Some("Contains words with very low confidence".to_string());
        }
    }
    segments
}
