//! Standalone harness for `identify_language_by_trial` (managers/transcription.rs
//! keeps that function private, so this is a hand-mirrored copy, not a shared
//! call — keep the trial logic in sync by hand if that function changes).
//!
//! Feeds real WAV recordings through the exact forced-language trial-decode
//! approach the app uses for Nemotron auto-detect with an allowlist, against
//! the real model, and reports expected vs. actual per test case.
//!
//! Usage:
//!   cargo run --example language_trial_harness --no-default-features -- \
//!     <cases.json> [--model <path/to.gguf>]
//!
//! cases.json:
//!   [
//!     {"wav": "samples/pl_short.wav", "expected": "pl-PL"},
//!     {"wav": "samples/en_short.wav", "expected": "en-US"}
//!   ]
//!
//! WAV files must be 16 kHz mono 16-bit PCM (same requirement as --transcribe-file;
//! convert with e.g. `afconvert -f WAVE -d LEI16@16000 -c 1 in.m4a out.wav`).

use serde::Deserialize;
use std::env;
use std::path::PathBuf;
use std::process::ExitCode;
use transcribe_cpp::{Model, ModelOptions, RunOptions, Session, Task, TimestampKind};

/// Must match `LANGUAGE_TRIAL_AUDIO_MS` in managers/transcription.rs.
const LANGUAGE_TRIAL_AUDIO_MS: usize = 1200;
const LANGUAGE_TRIAL_AUDIO_SAMPLES: usize = 16_000 * LANGUAGE_TRIAL_AUDIO_MS / 1000;

/// Candidates tried per case, mirroring `nemotron_trial_candidates()`'s output
/// for an `en-US` + `pl-PL` allowlist. Override per-case with "candidates" if
/// a manifest entry needs a different set.
const DEFAULT_CANDIDATES: &[&str] = &["en-US", "pl-PL"];

/// RMS-based leading-silence trim, approximating what the app's VAD-gated
/// recorder already does before audio ever reaches the trial buffer (see
/// `AudioRecordingManager::with_vad` in managers/audio.rs) — a raw voice-memo
/// recording has dead air the live pipeline would never hand the trial, so
/// trimming here keeps this harness honest about how much *speech* actually
/// lands in the trial window.
fn trim_leading_silence(pcm: &[f32]) -> &[f32] {
    const FRAME: usize = 480; // 30ms @ 16kHz, matches Silero's frame size
    const RMS_THRESHOLD: f32 = 0.01;

    for (i, frame) in pcm.chunks(FRAME).enumerate() {
        let rms = (frame.iter().map(|s| s * s).sum::<f32>() / frame.len() as f32).sqrt();
        if rms >= RMS_THRESHOLD {
            return &pcm[(i * FRAME)..];
        }
    }
    pcm
}

fn read_wav_samples(path: &std::path::Path) -> anyhow::Result<Vec<f32>> {
    let mut reader = hound::WavReader::open(path)?;
    let spec = reader.spec();
    if spec.sample_rate != 16_000 || spec.channels != 1 || spec.bits_per_sample != 16 {
        anyhow::bail!(
            "{}: expected 16kHz mono 16-bit PCM, got {}Hz/{}ch/{}-bit",
            path.display(),
            spec.sample_rate,
            spec.channels,
            spec.bits_per_sample
        );
    }
    Ok(reader
        .samples::<i16>()
        .map(|s| s.map(|v| v as f32 / i16::MAX as f32))
        .collect::<Result<Vec<f32>, _>>()?)
}

#[derive(Deserialize)]
struct Case {
    wav: PathBuf,
    expected: String,
    #[serde(default)]
    candidates: Option<Vec<String>>,
}

/// Mirrors `identify_language_by_trial()` in managers/transcription.rs.
fn identify_language_by_trial(
    session: &mut Session,
    pcm: &[f32],
    candidates: &[String],
) -> (Option<String>, Vec<(String, f32, String)>) {
    let mut scored = Vec::new();
    let mut best: Option<(String, f32)> = None;
    for candidate in candidates {
        let options = RunOptions {
            task: Task::Transcribe,
            timestamps: TimestampKind::Token,
            language: Some(candidate.clone()),
            ..Default::default()
        };
        let transcript = match session.run(pcm, &options) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("  trial '{candidate}' errored: {e}");
                continue;
            }
        };
        let scores: Vec<f32> = transcript
            .tokens
            .iter()
            .map(|t| t.p)
            .filter(|p| p.is_finite())
            .collect();
        if scores.is_empty() {
            scored.push((candidate.clone(), f32::NAN, transcript.text));
            continue;
        }
        let avg_p = scores.iter().sum::<f32>() / scores.len() as f32;
        scored.push((candidate.clone(), avg_p, transcript.text));
        if best.as_ref().is_none_or(|(_, best_p)| avg_p > *best_p) {
            best = Some((candidate.clone(), avg_p));
        }
    }
    (best.map(|(lang, _)| lang), scored)
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let mut manifest_path: Option<PathBuf> = None;
    let mut model_path: Option<PathBuf> = None;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--model" => {
                i += 1;
                model_path = args.get(i).map(PathBuf::from);
            }
            other => manifest_path = Some(PathBuf::from(other)),
        }
        i += 1;
    }

    let Some(manifest_path) = manifest_path else {
        eprintln!("usage: language_trial_harness <cases.json> [--model <path.gguf>]");
        return ExitCode::FAILURE;
    };
    let model_path = model_path.unwrap_or_else(|| {
        let home = env::var("HOME").unwrap_or_default();
        PathBuf::from(format!(
            "{home}/.cache/huggingface/hub/models--handy-computer--nemotron-3.5-asr-streaming-0.6b-gguf/snapshots/6d44e540bc31b0de1dbe174a3cea87f53a7f22fb/nemotron-3.5-asr-streaming-0.6b-Q8_0.gguf"
        ))
    });

    let manifest = match std::fs::read_to_string(&manifest_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("cannot read {}: {e}", manifest_path.display());
            return ExitCode::FAILURE;
        }
    };
    let mut cases: Vec<Case> = match serde_json::from_str(&manifest) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("cannot parse {}: {e}", manifest_path.display());
            return ExitCode::FAILURE;
        }
    };
    // Resolve each case's `wav` relative to the manifest file, not the
    // process's CWD, so `cases.json` can be run from anywhere.
    if let Some(manifest_dir) = manifest_path.parent() {
        for case in &mut cases {
            if case.wav.is_relative() {
                case.wav = manifest_dir.join(&case.wav);
            }
        }
    }

    eprintln!("loading model: {}", model_path.display());
    let model = match Model::load_with(&model_path, &ModelOptions::default()) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("failed to load model: {e}");
            return ExitCode::FAILURE;
        }
    };
    let mut session = match model.session() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("failed to create session: {e}");
            return ExitCode::FAILURE;
        }
    };

    let mut pass = 0;
    let mut fail = 0;
    for case in &cases {
        let candidates: Vec<String> = case
            .candidates
            .clone()
            .unwrap_or_else(|| DEFAULT_CANDIDATES.iter().map(|s| s.to_string()).collect());

        println!(
            "\n=== {} (expected {}) ===",
            case.wav.display(),
            case.expected
        );

        let raw = match read_wav_samples(&case.wav) {
            Ok(s) => s,
            Err(e) => {
                println!("  SKIP: {e}");
                continue;
            }
        };
        let trimmed = trim_leading_silence(&raw);
        let leading_silence_ms = (raw.len() - trimmed.len()) * 1000 / 16_000;
        let window_end = trimmed.len().min(LANGUAGE_TRIAL_AUDIO_SAMPLES);
        let trial_pcm = &trimmed[..window_end];
        println!(
            "  audio: {}ms total, {}ms leading silence trimmed, {}ms fed to trial",
            raw.len() * 1000 / 16_000,
            leading_silence_ms,
            trial_pcm.len() * 1000 / 16_000
        );

        let (actual, scored) = identify_language_by_trial(&mut session, trial_pcm, &candidates);
        for (lang, avg_p, text) in &scored {
            println!("  {lang}: avg_p={avg_p:.4} text={text:?}");
        }

        match &actual {
            Some(lang) if *lang == case.expected => {
                println!("  PASS (picked {lang})");
                pass += 1;
            }
            Some(lang) => {
                println!("  FAIL (picked {lang}, expected {})", case.expected);
                fail += 1;
            }
            None => {
                println!("  FAIL (undecided — too little audio for any candidate to score)");
                fail += 1;
            }
        }
    }

    println!("\n{pass} passed, {fail} failed, {} total", cases.len());
    if fail > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
