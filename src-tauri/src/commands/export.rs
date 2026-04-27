use crate::models::transcript::{
    ExportFormat, ExportOptions, SegmentType, Transcript, TranscriptSegment,
};

/// Export a transcript to a formatted string.
#[tauri::command]
pub fn export_transcript(transcript: Transcript, options: ExportOptions) -> Result<String, String> {
    match options.format {
        ExportFormat::Md => Ok(export_markdown(&transcript, &options)),
        ExportFormat::Txt => Ok(export_plaintext(&transcript, &options)),
        ExportFormat::AiMd => Ok(export_ai_markdown(&transcript)),
    }
}

fn format_timestamp(ms: u64) -> String {
    let total_secs = ms / 1000;
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;
    if hours > 0 {
        format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
    } else {
        format!("{:02}:{:02}", minutes, seconds)
    }
}

fn get_speaker_label(segment: &TranscriptSegment) -> String {
    segment
        .speaker_name
        .clone()
        .or_else(|| segment.speaker_id.clone())
        .unwrap_or_else(|| "Unknown".to_string())
}

fn export_markdown(transcript: &Transcript, options: &ExportOptions) -> String {
    let mut output = String::new();
    output.push_str(&format!("# Transcription: {}\n\n", transcript.source_file));

    if !transcript.speakers.is_empty() && options.include_speaker_labels {
        output.push_str("## Speakers\n\n");
        for speaker in &transcript.speakers {
            let name = speaker
                .name
                .as_deref()
                .unwrap_or(speaker.id.as_str());
            output.push_str(&format!("- **{}**\n", name));
        }
        output.push_str("\n---\n\n");
    }

    output.push_str("## Transcript\n\n");

    let mut last_speaker: Option<String> = None;

    for segment in &transcript.segments {
        // Handle non-speech annotations
        if segment.segment_type != SegmentType::Speech {
            if options.include_annotations {
                let annotation = match segment.segment_type {
                    SegmentType::Noise => "[noise]",
                    SegmentType::Music => "[music]",
                    SegmentType::Silence => "[silence]",
                    SegmentType::Unknown => "[unknown]",
                    SegmentType::Speech => unreachable!(),
                };
                output.push_str(&format!("*{}*\n\n", annotation));
            }
            continue;
        }

        // Speaker label (only when speaker changes)
        if options.include_speaker_labels {
            let current_speaker = Some(get_speaker_label(segment));
            if current_speaker != last_speaker {
                if let Some(ref speaker) = current_speaker {
                    output.push_str(&format!("**{}**", speaker));
                    if options.include_timestamps {
                        output.push_str(&format!(" [{}]", format_timestamp(segment.start_ms)));
                    }
                    output.push('\n');
                }
                last_speaker = current_speaker;
            } else if options.include_timestamps {
                // Same speaker, but show timestamp for new paragraph
                output.push_str(&format!("[{}] ", format_timestamp(segment.start_ms)));
            }
        } else if options.include_timestamps {
            output.push_str(&format!("[{}] ", format_timestamp(segment.start_ms)));
        }

        // Text with confidence highlighting
        if options.highlight_low_confidence && segment.is_flagged {
            output.push_str(&format!("> **[Low confidence]** {}\n", segment.text));
        } else if options.highlight_low_confidence {
            // Highlight individual low-confidence tokens
            let mut text = segment.text.clone();
            for tc in &segment.token_confidences {
                if tc.confidence < 0.7 {
                    text = text.replace(&tc.token, &format!("*{}*", tc.token));
                }
            }
            output.push_str(&format!("{}\n", text));
        } else {
            output.push_str(&format!("{}\n", segment.text));
        }

        output.push('\n');
    }

    output
}

fn export_plaintext(transcript: &Transcript, options: &ExportOptions) -> String {
    let mut output = String::new();
    output.push_str(&format!("Transcription: {}\n", transcript.source_file));
    output.push_str(&format!(
        "{}\n\n",
        "=".repeat(transcript.source_file.len() + 16)
    ));

    let mut last_speaker: Option<String> = None;

    for segment in &transcript.segments {
        if segment.segment_type != SegmentType::Speech {
            if options.include_annotations {
                let annotation = match segment.segment_type {
                    SegmentType::Noise => "[noise]",
                    SegmentType::Music => "[music]",
                    SegmentType::Silence => "[silence]",
                    SegmentType::Unknown => "[unknown]",
                    SegmentType::Speech => unreachable!(),
                };
                output.push_str(&format!("{}\n\n", annotation));
            }
            continue;
        }

        let mut line = String::new();

        if options.include_timestamps {
            line.push_str(&format!("[{}] ", format_timestamp(segment.start_ms)));
        }

        if options.include_speaker_labels {
            let current_speaker = Some(get_speaker_label(segment));
            if current_speaker != last_speaker {
                if let Some(ref speaker) = current_speaker {
                    line.push_str(&format!("[{}]: ", speaker));
                }
                last_speaker = current_speaker;
            }
        }

        line.push_str(&segment.text);

        if options.highlight_low_confidence && segment.is_flagged {
            line.push_str(" [?]");
        }

        output.push_str(&format!("{}\n", line));
    }

    output
}

// =============================================================================
// AI-friendly markdown export
// =============================================================================

/// Show segment confidence inline only when below this threshold (as a percent).
/// Above this, "good enough" is implied and we save tokens by omitting it.
const AI_CONFIDENCE_DISPLAY_BELOW: u8 = 85;

/// Mark individual words as `~~struck-through~~` when their alignment score
/// is below this threshold. Matches the UI's red-dashed-underline tier.
const AI_WORD_STRIKETHROUGH_BELOW: f32 = 0.30;

/// Format a timestamp as `MM:SS.mmm` or `HH:MM:SS.mmm`.
fn format_timestamp_ms(ms: u64) -> String {
    let total_secs = ms / 1000;
    let millis = ms % 1000;
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;
    if hours > 0 {
        format!("{:02}:{:02}:{:02}.{:03}", hours, minutes, seconds, millis)
    } else {
        format!("{:02}:{:02}.{:03}", minutes, seconds, millis)
    }
}

fn format_duration_human(ms: u64) -> String {
    let total_secs = ms / 1000;
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;
    if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, seconds)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds)
    } else {
        format!("{}s", seconds)
    }
}

/// YAML scalars that contain `:` `#` quotes etc. need quoting. We do it
/// conservatively (always quote when in doubt) since these strings come
/// from arbitrary filenames and user-typed speaker names.
fn yaml_string(s: &str) -> String {
    let needs_quotes = s.is_empty()
        || s.contains([':', '#', '"', '\'', '\n', '\r', '\t', '`'])
        || s.starts_with(['-', '?', '!', '|', '>', '%', '@', '&', '*', '['])
        || s.starts_with(' ')
        || s.ends_with(' ');
    if !needs_quotes {
        return s.to_string();
    }
    let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{}\"", escaped)
}

/// Render the segment text with `~~strikethrough~~` around very-low-confidence words.
fn render_text_with_strikethrough(segment: &TranscriptSegment) -> String {
    if segment.token_confidences.is_empty() {
        return segment.text.clone();
    }

    let mut result = String::with_capacity(segment.text.len() + 32);
    let mut remaining: &str = &segment.text;

    for tc in &segment.token_confidences {
        if tc.token.is_empty() {
            continue;
        }
        if let Some(idx) = remaining.find(tc.token.as_str()) {
            result.push_str(&remaining[..idx]);
            if tc.confidence < AI_WORD_STRIKETHROUGH_BELOW {
                result.push_str("~~");
                result.push_str(&tc.token);
                result.push_str("~~");
            } else {
                result.push_str(&tc.token);
            }
            remaining = &remaining[idx + tc.token.len()..];
        }
    }
    result.push_str(remaining);
    result
}

fn export_ai_markdown(transcript: &Transcript) -> String {
    let speech_segments: Vec<&TranscriptSegment> = transcript
        .segments
        .iter()
        .filter(|s| s.segment_type == SegmentType::Speech)
        .collect();
    let flagged_count = speech_segments.iter().filter(|s| s.is_flagged).count();

    let mut output = String::new();

    // ---------- YAML frontmatter ----------
    output.push_str("---\n");
    output.push_str("format: tracet-transcript\n");
    output.push_str("version: 1\n");
    output.push_str(&format!(
        "source: {}\n",
        yaml_string(&transcript.source_file)
    ));
    output.push_str(&format!("duration_ms: {}\n", transcript.duration_ms));
    output.push_str(&format!(
        "duration_human: {}\n",
        yaml_string(&format_duration_human(transcript.duration_ms))
    ));
    output.push_str(&format!(
        "exported_at: {}\n",
        yaml_string(&chrono::Utc::now().to_rfc3339())
    ));

    if !transcript.speakers.is_empty() {
        output.push_str("speakers:\n");
        for sp in &transcript.speakers {
            let name = sp.name.as_deref().unwrap_or(sp.id.as_str());
            output.push_str(&format!(
                "  - {{ id: {}, name: {} }}\n",
                yaml_string(&sp.id),
                yaml_string(name)
            ));
        }
    }

    output.push_str(&format!("segment_count: {}\n", speech_segments.len()));
    output.push_str(&format!("flagged_count: {}\n", flagged_count));

    output.push_str("notation:\n");
    output.push_str(
        "  numbered_segments: each segment has a leading number for precise reference\n",
    );
    output.push_str(
        "  strikethrough: word had very low alignment confidence (<30%)\n",
    );
    output.push_str(
        "  warning_emoji: segment was heuristically flagged as likely incorrect\n",
    );
    output.push_str(&format!(
        "  confidence_shown_below: {}\n",
        AI_CONFIDENCE_DISPLAY_BELOW
    ));
    output.push_str("---\n\n");

    // ---------- Body ----------
    output.push_str(&format!("# Transcript: {}\n\n", transcript.source_file));

    let mut segment_number: u32 = 0;
    for segment in &transcript.segments {
        // Non-speech annotations: kept compact and clearly marked.
        if segment.segment_type != SegmentType::Speech {
            let label = match segment.segment_type {
                SegmentType::Noise => "[noise]",
                SegmentType::Music => "[music]",
                SegmentType::Silence => "[silence]",
                SegmentType::Unknown => "[unknown]",
                SegmentType::Speech => unreachable!(),
            };
            output.push_str(&format!(
                "_{} `[{} → {}]`_\n\n",
                label,
                format_timestamp_ms(segment.start_ms),
                format_timestamp_ms(segment.end_ms)
            ));
            continue;
        }

        segment_number += 1;
        let speaker = segment
            .speaker_name
            .clone()
            .or_else(|| segment.speaker_id.clone())
            .unwrap_or_else(|| "Unknown".to_string());

        // Header line
        let confidence_pct = (segment.confidence * 100.0).round() as u8;
        let show_confidence = confidence_pct < AI_CONFIDENCE_DISPLAY_BELOW;

        output.push_str(&format!(
            "**{}** {} `[{} → {}]`",
            segment_number,
            speaker,
            format_timestamp_ms(segment.start_ms),
            format_timestamp_ms(segment.end_ms)
        ));
        if show_confidence {
            output.push_str(&format!(" ({}%)", confidence_pct));
        }
        if segment.is_flagged {
            output.push_str(" \u{26A0}"); // ⚠
        }
        output.push_str(":\n");

        // Text body with optional inline strikethrough
        output.push_str(&render_text_with_strikethrough(segment));
        output.push('\n');

        // Flag reason as blockquote
        if let Some(reason) = &segment.flag_reason {
            output.push_str(&format!("> Flagged: {}\n", reason));
        }
        output.push('\n');
    }

    output
}
