use crate::models::transcript::{
    ExportFormat, ExportOptions, SegmentType, Transcript, TranscriptSegment,
};

/// Export a transcript to a formatted string (txt or md).
#[tauri::command]
pub fn export_transcript(transcript: Transcript, options: ExportOptions) -> Result<String, String> {
    match options.format {
        ExportFormat::Md => Ok(export_markdown(&transcript, &options)),
        ExportFormat::Txt => Ok(export_plaintext(&transcript, &options)),
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
