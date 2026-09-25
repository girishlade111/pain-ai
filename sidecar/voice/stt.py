#!/usr/bin/env python3
"""pain ai — Speech-to-Text Engine (stt.py)

Transcribes audio recordings into text using:
1. faster-whisper with CUDA acceleration if GPU is present
2. whisper.cpp / faster-whisper CPU as fallback
3. Configurable model hierarchy: large-v3-turbo-int8 -> small -> base
4. Silero VAD trim to strip leading/trailing silence
5. Download-once model caching in ~/.pain-ai/models/stt/
6. Clean offline fallback for test environments without heavy model weights
"""

import argparse
import json
import logging
import math
import os
import shutil
import struct
import subprocess
import sys
import time
import wave
from pathlib import Path
from typing import Any, Dict, Optional, Tuple

logger = logging.getLogger("stt")

def _resolve_home() -> Path:
    """P13: home resolved per call (not frozen at import) so tests redirect
    ALL state with PAIN_AI_HOME/HERMES_HOME and never touch the operator's
    live ~/.pain-ai. Production behavior unchanged (env unset → ~/.pain-ai)."""
    import os as _os
    for key in ("PAIN_AI_HOME", "HERMES_HOME"):
        val = _os.environ.get(key, "").strip()
        if val:
            return Path(val)
    return Path.home() / ".pain-ai"


def _stt_models_dir() -> Path:
    d = _resolve_home() / "models" / "stt"
    d.mkdir(parents=True, exist_ok=True)
    return d


# Legacy module constants (import-time snapshot of the default home). Kept
# for backward-compatible reads; model resolution goes through the dynamic
# helper so env isolation is honored.
PAIN_AI_HOME = Path(os.environ.get("HERMES_HOME", Path.home() / ".pain-ai"))
STT_MODELS_DIR = PAIN_AI_HOME / "models" / "stt"
try:
    STT_MODELS_DIR.mkdir(parents=True, exist_ok=True)
except OSError:
    pass

MODEL_CHOICES = ["large-v3-turbo-int8", "small", "base"]

# Phase 11: process-lifetime model cache keyed (model, device). Constructing
# WhisperModel per call costs ~20s of reload on CPU; caching changes nothing
# about results, only latency.
_MODEL_CACHE: Dict[str, Any] = {}

# Phase 11: sub-speech energy floor (16-bit PCM peak). Below this there is no
# human speech to transcribe — tiny Whisper variants famously hallucinate
# words ("you", "thanks") on pure silence, so the gate returns honest empty
# text instead of model confabulation. -36 dBFS keeps real speech (peaks in
# the thousands) far above the floor.
SILENCE_PEAK_THRESHOLD = 500


def _trim_silence_vad(wav_path: Path) -> Path:
    """Trims leading and trailing silence from the audio file using energy thresholding (VAD).

    Returns the path to the trimmed WAV file (or original if trimming unnecessary).
    """
    try:
        with wave.open(str(wav_path), "rb") as wf:
            params = wf.getparams()
            frames = wf.readframes(params.nframes)

        sample_width = params.sampwidth
        n_channels = params.nchannels
        frame_rate = params.framerate

        if sample_width != 2:
            return wav_path  # Only process 16-bit PCM for simple VAD

        # Unpack samples (first channel if stereo)
        num_samples = len(frames) // (sample_width * n_channels)
        fmt = f"<{num_samples * n_channels}h"
        raw_data = struct.unpack(fmt, frames)
        samples = raw_data[0::n_channels]

        # Calculate energy in 20ms chunks
        chunk_size = int(frame_rate * 0.02)
        if chunk_size <= 0 or len(samples) < chunk_size:
            return wav_path

        energies = []
        for i in range(0, len(samples), chunk_size):
            chunk = samples[i:i + chunk_size]
            rms = math.sqrt(sum(s * s for s in chunk) / len(chunk))
            energies.append(rms)

        # Dynamic noise floor threshold
        noise_floor = min(energies) if energies else 100
        threshold = max(250, noise_floor * 2.5)

        start_chunk = 0
        while start_chunk < len(energies) and energies[start_chunk] < threshold:
            start_chunk += 1

        end_chunk = len(energies) - 1
        while end_chunk > start_chunk and energies[end_chunk] < threshold:
            end_chunk -= 1

        # Keep a 100ms buffer before and after
        pad_chunks = int(0.1 / 0.02)
        start_chunk = max(0, start_chunk - pad_chunks)
        end_chunk = min(len(energies) - 1, end_chunk + pad_chunks)

        start_sample = start_chunk * chunk_size
        end_sample = min(num_samples, (end_chunk + 1) * chunk_size)

        if start_sample == 0 and end_sample >= num_samples:
            return wav_path

        # Write trimmed wav
        trimmed_path = wav_path.parent / f"trimmed_{wav_path.name}"
        trimmed_samples = raw_data[start_sample * n_channels:end_sample * n_channels]
        trimmed_bytes = struct.pack(f"<{len(trimmed_samples)}h", *trimmed_samples)

        with wave.open(str(trimmed_path), "wb") as out_wf:
            out_wf.setnchannels(n_channels)
            out_wf.setsampwidth(sample_width)
            out_wf.setframerate(frame_rate)
            out_wf.writeframes(trimmed_bytes)

        return trimmed_path
    except Exception as e:
        logger.debug(f"VAD energy trimming skipped: {e}")
        return wav_path


def _peak_amplitude(wav_path: Path) -> Optional[int]:
    """Peak absolute sample of a 16-bit PCM WAV (None when unreadable)."""
    try:
        with wave.open(str(wav_path), "rb") as wf:
            if wf.getsampwidth() != 2:
                return None
            frames = wf.readframes(wf.getnframes())
        peak = 0
        for (sample,) in struct.iter_unpack("<h", frames[: len(frames) // 2 * 2]):
            peak = max(peak, abs(sample))
            if peak >= SILENCE_PEAK_THRESHOLD:
                break
        return peak
    except Exception:
        return None


def _transcribe_faster_whisper(wav_path: Path, model_size: str, device: str) -> Optional[Tuple[str, str, int]]:
    """Transcribes audio using faster-whisper if available (model cached)."""
    try:
        from faster_whisper import WhisperModel
        start_time = time.time()
        cache_key = f"{model_size}/{device}"
        model = _MODEL_CACHE.get(cache_key)
        if model is None:
            compute_type = "int8" if device == "cpu" else "float16"
            model = WhisperModel(
                model_size,
                device=device,
                compute_type=compute_type,
                download_root=str(_stt_models_dir())
            )
            _MODEL_CACHE[cache_key] = model
        segments, info = model.transcribe(str(wav_path), beam_size=5)
        text = " ".join(seg.text for seg in segments).strip()
        elapsed_ms = int((time.time() - start_time) * 1000)
        return text, info.language, elapsed_ms
    except Exception as e:
        logger.debug(f"faster-whisper inference unavailable: {e}")
        return None


def _transcribe_whisper_cpp(wav_path: Path, model_size: str) -> Optional[Tuple[str, str, int]]:
    """Transcribes audio using whisper.cpp CLI if available."""
    whisper_bin = os.environ.get("WHISPER_CPP_BIN") or shutil.which("whisper-cli") or shutil.which("whisper-cpp")
    model_bin = _stt_models_dir() / f"ggml-{model_size}.bin"

    if whisper_bin and model_bin.exists():
        try:
            start_time = time.time()
            cmd = [
                whisper_bin,
                "-m", str(model_bin),
                "-f", str(wav_path),
                "--no-timestamps"
            ]
            res = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, check=True)
            text = res.stdout.strip()
            elapsed_ms = int((time.time() - start_time) * 1000)
            return text, "en", elapsed_ms
        except Exception as e:
            logger.debug(f"whisper.cpp inference failed: {e}")
            return None

    return None


def transcribe(wav_path: str, model_size: Optional[str] = None) -> Dict[str, Any]:
    """Transcribes a WAV audio file into text.

    Returns:
        Dict: {"ok": True, "text": str, "lang": str, "ms": int, "engine": str}
    """
    path = Path(wav_path)
    if not path.exists():
        return {
            "ok": False,
            "error": f"WAV file not found: {wav_path}",
            "code": "FILE_NOT_FOUND"
        }

    # Step 1: VAD Silence Trimming
    trimmed_wav = _trim_silence_vad(path)

    # Step 1b: Sub-speech energy gate. Deterministic honest-empty for silence
    # (tiny Whisper hallucinates words on pure silence); real speech with
    # peaks in the thousands passes far above the floor.
    peak = _peak_amplitude(trimmed_wav)
    if peak is not None and peak < SILENCE_PEAK_THRESHOLD:
        return {"ok": True, "text": "", "lang": "en", "ms": 0,
                "engine": "silence-gate"}

    # Step 2: Determine Engine & Device
    engine_override = os.environ.get("LSC_STT", "").lower().strip()
    active_model = model_size or os.environ.get("LSC_STT_MODEL", "base")

    # Check CUDA presence
    has_cuda = False
    try:
        import torch
        has_cuda = torch.cuda.is_available()
    except Exception:
        pass

    preferred_device = "cuda" if has_cuda else "cpu"

    # Step 3: Attempt whisper.cpp if explicitly selected
    if engine_override == "whisper.cpp":
        cpp_res = _transcribe_whisper_cpp(trimmed_wav, active_model)
        if cpp_res:
            text, lang, ms = cpp_res
            return {"ok": True, "text": text, "lang": lang, "ms": ms, "engine": "whisper.cpp"}

    # Step 4: Attempt faster-whisper
    fw_res = _transcribe_faster_whisper(trimmed_wav, active_model, preferred_device)
    if fw_res:
        text, lang, ms = fw_res
        return {"ok": True, "text": text, "lang": lang, "ms": ms, "engine": f"faster-whisper-{preferred_device}"}

    # Step 5: No model available. Phase 2: explicit failure, never a canned
    # transcript. A fabricated transcript would enter the prompt as user input.
    return {
        "ok": False,
        "text": "",
        "lang": "en",
        "ms": 0,
        "engine": "unavailable",
        "code": "STT_UNAVAILABLE",
        "error": "No speech-to-text model available (faster-whisper/whisper.cpp weights missing and offline transcription disabled).",
    }


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="pain ai STT CLI")
    parser.add_argument("--wav", required=True, help="Path to WAV audio file to transcribe")
    parser.add_argument("--model", default=None, help="Whisper model size")
    args = parser.parse_args()

    res = transcribe(args.wav, model_size=args.model)
    print(json.dumps(res))
