//! pain ai — Audio Subsystem (audio.rs)
//!
//! SSOT (Phase 1): Desktop audio device boundary owner (rodio playback, cpal mic,
//! hound WAV, voice_state events). Speech engines live in sidecar/voice/
//! (stt.py faster-whisper, tts.py Piper, segment.py sentence split); caption
//! rendering lives in React (CaptionBar + lib/segment.ts mirror of segment.py).
//! Provides sequential sentence audio queue playback using `rodio`, immediate <500ms stop,
//! microphone capture via `cpal`/`hound`, and voice state synchronization events.

use hound::{WavSpec, WavWriter};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::BufReader;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

// Global playback cancellation flag
static PLAYBACK_STOP_FLAG: AtomicBool = AtomicBool::new(false);

// Active sink mutex for immediate <500ms stop
static AUDIO_ACTIVE_SINK: Mutex<Option<Arc<rodio::Sink>>> = Mutex::new(None);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceSentenceItem {
    pub i: usize,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceStatePayload {
    pub state: String, // "playing" | "stopped" | "done"
    pub active_i: Option<usize>,
    pub item_count: Option<usize>,
    pub latency_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordResult {
    pub wav_path: String,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SttResult {
    pub ok: bool,
    pub text: String,
    pub lang: Option<String>,
    pub ms: u64,
    pub engine: String,
}

fn get_app_data_audio_dir() -> PathBuf {
    // P13: PAIN_AI_HOME first so unit tests (IsolatedHome) never touch the
    // operator's live ~/.pain-ai/audio. Mirrors gate::get_appdata_dir.
    let base = if let Ok(home) = std::env::var("PAIN_AI_HOME") {
        PathBuf::from(home)
    } else if let Ok(hermes_home) = std::env::var("HERMES_HOME") {
        PathBuf::from(hermes_home)
    } else if let Ok(home) = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")) {
        PathBuf::from(home).join(".pain-ai")
    } else {
        PathBuf::from(".pain-ai")
    };
    let dir = base.join("audio");
    let _ = fs::create_dir_all(&dir);
    dir
}

/// Sidecar HTTP endpoint for voice (Phase 12 prod path). None when the
/// manager was never initialized (unit tests) — callers fall back to repo
/// scripts without spawning a child process.
fn sidecar_voice_base() -> Option<(String, String)> {
    let mgr = crate::sidecar::try_get_sidecar()?;
    let port = mgr.get_status().port;
    Some((format!("http://127.0.0.1:{}", port), mgr.get_token()))
}

/// Fetch synthesized speech over HTTP (works against the packaged engine;
/// repo scripts do not exist outside the checkout).
fn http_tts_wav(text: &str, voice: Option<&str>) -> Result<Vec<u8>, String> {
    use base64::Engine as _;
    let (base, token) = sidecar_voice_base().ok_or_else(|| "sidecar not initialized".to_string())?;
    let mut body = serde_json::json!({ "text": text });
    if let Some(v) = voice {
        body["voice"] = serde_json::Value::String(v.to_string());
    }
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(90))
        .build()
        .map_err(|e| format!("TTS HTTP client failed: {}", e))?;
    let resp = client
        .post(format!("{}/v1/voice/tts", base))
        .header("Authorization", format!("Bearer {}", token))
        .json(&body)
        .send()
        .map_err(|e| format!("TTS sidecar unreachable: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("TTS sidecar error (HTTP {})", resp.status().as_u16()));
    }
    let parsed: serde_json::Value = resp
        .json()
        .map_err(|e| format!("TTS bad response: {}", e))?;
    let b64 = parsed
        .get("wav_b64")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "TTS response missing audio".to_string())?;
    base64::engine::general_purpose::STANDARD
        .decode(b64)
        .map_err(|e| format!("TTS audio decode failed: {}", e))
}

/// Synthesizes speech for a sentence.
/// Phase 12: sidecar HTTP first (works with the packaged engine); repo
/// script fallback only when repo files exist (dev). Phase 2 invariant kept:
/// no Rust-side tone synthesis — failures are explicit errors.
pub fn generate_sentence_wav(text: &str, voice: Option<&str>) -> Result<PathBuf, String> {
    if let Ok(bytes) = http_tts_wav(text, voice) {
        // Cache by content hash so repeated sentences skip re-synthesis.
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        voice.hash(&mut hasher);
        let dir = get_app_data_audio_dir().join("tts_http");
        let _ = fs::create_dir_all(&dir);
        let path = dir.join(format!("speak_{:016x}.wav", hasher.finish()));
        fs::write(&path, &bytes).map_err(|e| format!("TTS cache write failed: {}", e))?;
        return Ok(path);
    }

    // cargo test runs with CWD=src-tauri; the desktop runs with CWD=repo root.
    let tts_script = ["sidecar/voice/tts.py", "../sidecar/voice/tts.py"]
        .iter()
        .map(PathBuf::from)
        .find(|p| p.exists());
    if let Some(script) = tts_script {
        let mut cmd = std::process::Command::new("python");
        cmd.arg(&script).arg("--text").arg(text);
        if let Some(v) = voice {
            cmd.arg("--voice").arg(v);
        }

        if let Ok(output) = cmd.output() {
            if output.status.success() {
                if let Ok(parsed) = serde_json::from_slice::<serde_json::Value>(&output.stdout) {
                    if let Some(wav_str) = parsed.get("wav_path").and_then(|p| p.as_str()) {
                        let path = PathBuf::from(wav_str);
                        if path.exists() {
                            return Ok(path);
                        }
                    }
                    if let Some(err) = parsed.get("error").and_then(|e| e.as_str()) {
                        return Err(format!("TTS engine failed: {}", err));
                    }
                }
                return Err("TTS engine returned unparseable output".to_string());
            }
            return Err(format!(
                "TTS engine exited with status {}",
                output.status.code().unwrap_or(-1)
            ));
        }
        return Err("Failed to spawn TTS engine (sidecar/voice/tts.py)".to_string());
    }

    Err("TTS unavailable: sidecar unreachable and no repo script fallback".to_string())
}

/// Plays sentence items sequentially on the audio queue while emitting live voice_state events
pub async fn voice_speak(
    items: Vec<VoiceSentenceItem>,
    app_handle: AppHandle,
) -> Result<(), String> {
    PLAYBACK_STOP_FLAG.store(false, Ordering::SeqCst);
    let total_count = items.len();

    // Spawning playback on a background thread so UI remains responsive
    tokio::task::spawn_blocking(move || {
        // Try opening default output stream
        let (_stream, stream_handle) = match rodio::OutputStream::try_default() {
            Ok(pair) => pair,
            Err(e) => {
                eprintln!("[AUDIO] Failed to initialize default audio output: {}", e);
                let _ = app_handle.emit("voice_state", VoiceStatePayload {
                    state: "done".into(),
                    active_i: None,
                    item_count: Some(total_count),
                    latency_ms: None,
                });
                return;
            }
        };

        let sink = match rodio::Sink::try_new(&stream_handle) {
            Ok(s) => Arc::new(s),
            Err(e) => {
                eprintln!("[AUDIO] Failed to create audio sink: {}", e);
                return;
            }
        };

        // Update active sink in mutex
        if let Ok(mut guard) = AUDIO_ACTIVE_SINK.lock() {
            if let Some(prev) = guard.take() {
                prev.stop();
            }
            *guard = Some(Arc::clone(&sink));
        }

        for item in items {
            // Check if playback was stopped
            if PLAYBACK_STOP_FLAG.load(Ordering::SeqCst) {
                sink.stop();
                break;
            }

            // Emit playing event for active sentence
            let _ = app_handle.emit("voice_state", VoiceStatePayload {
                state: "playing".into(),
                active_i: Some(item.i),
                item_count: Some(total_count),
                latency_ms: None,
            });

            // Generate or fetch WAV file
            if let Ok(wav_path) = generate_sentence_wav(&item.text, None) {
                if let Ok(file) = fs::File::open(&wav_path) {
                    let reader = BufReader::new(file);
                    if let Ok(source) = rodio::Decoder::new(reader) {
                        sink.append(source);
                        // Poll sink while checking stop flag
                        while !sink.empty() {
                            if PLAYBACK_STOP_FLAG.load(Ordering::SeqCst) {
                                sink.stop();
                                break;
                            }
                            std::thread::sleep(Duration::from_millis(50));
                        }
                    }
                }
            }

            if PLAYBACK_STOP_FLAG.load(Ordering::SeqCst) {
                break;
            }
        }

        // Final state emission
        if PLAYBACK_STOP_FLAG.load(Ordering::SeqCst) {
            let _ = app_handle.emit("voice_state", VoiceStatePayload {
                state: "stopped".into(),
                active_i: None,
                item_count: Some(total_count),
                latency_ms: None,
            });
        } else {
            let _ = app_handle.emit("voice_state", VoiceStatePayload {
                state: "done".into(),
                active_i: None,
                item_count: Some(total_count),
                latency_ms: None,
            });
        }
    });

    Ok(())
}

/// Immediately halts voice playback within <500ms
pub fn voice_stop(app_handle: Option<&AppHandle>) -> Result<u64, String> {
    let start = Instant::now();
    PLAYBACK_STOP_FLAG.store(true, Ordering::SeqCst);

    if let Ok(mut guard) = AUDIO_ACTIVE_SINK.lock() {
        if let Some(sink) = guard.take() {
            sink.stop();
        }
    }

    let elapsed_ms = start.elapsed().as_millis() as u64;

    if let Some(app) = app_handle {
        let _ = app.emit("voice_state", VoiceStatePayload {
            state: "stopped".into(),
            active_i: None,
            item_count: None,
            latency_ms: Some(elapsed_ms),
        });
    }

    Ok(elapsed_ms)
}

// Global recording state
static RECORDING_ACTIVE: AtomicBool = AtomicBool::new(false);
static RECORDING_SAMPLES: Mutex<Vec<i16>> = Mutex::new(Vec::new());
static RECORDING_START_TIME: Mutex<Option<Instant>> = Mutex::new(None);

/// Starts recording microphone input
pub fn voice_record_start() -> Result<(), String> {
    if RECORDING_ACTIVE.load(Ordering::SeqCst) {
        return Ok(());
    }

    RECORDING_ACTIVE.store(true, Ordering::SeqCst);
    if let Ok(mut samples) = RECORDING_SAMPLES.lock() {
        samples.clear();
    }
    if let Ok(mut start) = RECORDING_START_TIME.lock() {
        *start = Some(Instant::now());
    }

    // Spawn background capture thread
    std::thread::spawn(|| {
        use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
        let host = cpal::default_host();
        let device = match host.default_input_device() {
            Some(d) => d,
            None => {
                eprintln!("[MIC] No input device found");
                return;
            }
        };

        let config = match device.default_input_config() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[MIC] Failed to get default input config: {}", e);
                return;
            }
        };

        let stream = match config.sample_format() {
            cpal::SampleFormat::I16 => device.build_input_stream(
                &config.into(),
                move |data: &[i16], _: &_| {
                    if RECORDING_ACTIVE.load(Ordering::SeqCst) {
                        if let Ok(mut samples) = RECORDING_SAMPLES.lock() {
                            samples.extend_from_slice(data);
                        }
                    }
                },
                |err| eprintln!("[MIC] Error in input stream: {}", err),
                None,
            ),
            cpal::SampleFormat::F32 => device.build_input_stream(
                &config.into(),
                move |data: &[f32], _: &_| {
                    if RECORDING_ACTIVE.load(Ordering::SeqCst) {
                        if let Ok(mut samples) = RECORDING_SAMPLES.lock() {
                            for &s in data {
                                let sample_i16 = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
                                samples.push(sample_i16);
                            }
                        }
                    }
                },
                |err| eprintln!("[MIC] Error in input stream: {}", err),
                None,
            ),
            _ => return,
        };

        if let Ok(stream) = stream {
            let _ = stream.play();
            while RECORDING_ACTIVE.load(Ordering::SeqCst) {
                std::thread::sleep(Duration::from_millis(100));
            }
        }
    });

    Ok(())
}

/// Stops recording microphone input, flushes samples to WAV, and returns metadata
pub fn voice_record_stop() -> Result<RecordResult, String> {
    RECORDING_ACTIVE.store(false, Ordering::SeqCst);
    let duration_ms = if let Ok(mut start) = RECORDING_START_TIME.lock() {
        start.take().map(|s| s.elapsed().as_millis() as u64).unwrap_or(0)
    } else {
        0
    };

    let samples = if let Ok(mut s) = RECORDING_SAMPLES.lock() {
        std::mem::take(&mut *s)
    } else {
        Vec::new()
    };

    let audio_dir = get_app_data_audio_dir().join("recordings");
    let _ = fs::create_dir_all(&audio_dir);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let wav_path = audio_dir.join(format!("rec_{}.wav", now));

    let spec = WavSpec {
        channels: 1,
        sample_rate: 16000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = WavWriter::create(&wav_path, spec).map_err(|e| e.to_string())?;
    // If no real samples were collected (e.g. headless box without mic), insert minimal valid audio buffer
    if samples.is_empty() {
        for _ in 0..1600 {
            let _ = writer.write_sample(0i16);
        }
    } else {
        for sample in samples {
            let _ = writer.write_sample(sample);
        }
    }
    writer.finalize().map_err(|e| e.to_string())?;

    Ok(RecordResult {
        wav_path: wav_path.to_string_lossy().to_string(),
        duration_ms: duration_ms.max(100),
    })
}

/// Transcribes a WAV file.
/// Phase 12: sidecar HTTP first (packaged engine); repo script fallback for
/// dev. Phase 2: explicit errors only — a canned transcript would enter the
/// prompt as fabricated user input.
pub fn stt_transcribe(wav_path: &str) -> Result<SttResult, String> {
    if let Some((base, token)) = sidecar_voice_base() {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(180))
            .build()
            .map_err(|e| format!("STT HTTP client failed: {}", e))?;
        match client
            .post(format!("{}/v1/voice/transcribe", base))
            .header("Authorization", format!("Bearer {}", token))
            .json(&serde_json::json!({ "wav_path": wav_path }))
            .send()
        {
            Ok(resp) => {
                if resp.status().is_success() {
                    if let Ok(res) = resp.json::<SttResult>() {
                        if !res.ok {
                            return Err("STT engine unavailable (STT_UNAVAILABLE)".to_string());
                        }
                        if res.text.trim().is_empty() {
                            return Err("STT engine returned empty transcript".to_string());
                        }
                        return Ok(res);
                    }
                    return Err("STT bad response".to_string());
                }
            }
            Err(_) => {}
        }
        // Fall through to the repo script path below on any HTTP failure.
    }
    // cargo test runs with CWD=src-tauri; the desktop runs with CWD=repo root.
    let stt_script = ["sidecar/voice/stt.py", "../sidecar/voice/stt.py"]
        .iter()
        .map(PathBuf::from)
        .find(|p| p.exists());
    if let Some(script) = stt_script {
        let mut cmd = std::process::Command::new("python");
        cmd.arg(&script).arg("--wav").arg(wav_path);

        if let Ok(output) = cmd.output() {
            if output.status.success() {
                if let Ok(res) = serde_json::from_slice::<SttResult>(&output.stdout) {
                    if !res.ok {
                        return Err("STT engine unavailable (STT_UNAVAILABLE)".to_string());
                    }
                    if res.text.trim().is_empty() {
                        return Err("STT engine returned empty transcript".to_string());
                    }
                    return Ok(res);
                }
                return Err("STT engine returned unparseable output".to_string());
            }
            return Err(format!(
                "STT engine exited with status {}",
                output.status.code().unwrap_or(-1)
            ));
        }
        return Err("Failed to spawn STT engine (sidecar/voice/stt.py)".to_string());
    }

    Err("STT engine script not found (sidecar/voice/stt.py)".to_string())
}

#[cfg(test)]
#[path = "audio_tests.rs"]
mod audio_tests;
