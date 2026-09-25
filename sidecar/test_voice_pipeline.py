"""pain ai — Pytest Suite for the Real Voice Pipeline (Phase 11)

Proves real engines, never masquerade: Piper speech round-trips through
faster-whisper with words intact; silence transcribes to honest emptiness;
missing/corrupt inputs are explicit errors. Engine labels must match the
actually-used backend (cache carries provenance since Phase 11).
"""

import struct
import sys
import uuid
import wave
from pathlib import Path

root_dir = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(root_dir / "sidecar"))

from voice import tts as tts_mod
from voice import stt as stt_mod


def _unique(prefix: str) -> str:
    return f"{prefix} {uuid.uuid4().hex[:8]}"


def _expected_tts_engine() -> str:
    """Piper iff its voice model resolves in the effective models dir."""
    import os

    home = Path(os.environ.get("PAIN_AI_HOME", "").strip()
                or os.environ.get("HERMES_HOME", "").strip()
                or (Path.home() / ".pain-ai"))
    voice = tts_mod.DEFAULT_VOICE
    if (home / "models" / "piper" / f"{voice}.onnx").exists():
        return "piper"
    return "offline_synth"


def _wav_info(path: str):
    with wave.open(path, "rb") as wf:
        return {
            "channels": wf.getnchannels(),
            "width": wf.getsampwidth(),
            "rate": wf.getframerate(),
            "frames": wf.getnframes(),
            "raw": wf.readframes(wf.getnframes()),
        }


def _max_abs_sample(raw: bytes) -> int:
    peak = 0
    for (sample,) in struct.iter_unpack("<h", raw[: len(raw) // 2 * 2]):
        peak = max(peak, abs(sample))
    return peak


def test_tts_produces_valid_speech_wav():
    text = f"The quick brown fox jumps over the lazy dog {_unique('case')}."
    res = tts_mod.speak(text)
    assert res["ok"] is True, res
    assert res["engine"] == _expected_tts_engine(), res
    info = _wav_info(res["wav_path"])
    assert info["channels"] == 1 and info["width"] == 2 and info["rate"] == 22050
    assert info["frames"] > 22050, "speech must exceed one second for this text"
    assert _max_abs_sample(info["raw"]) > 1000, "audio must not be digital silence"


def test_tts_duration_scales_with_text():
    short = tts_mod.speak(f"Hi {_unique('s')}.")
    long_text = (
        "This is a deliberately longer sentence so the synthesized audio "
        f"lasts noticeably longer than the short greeting {_unique('l')}."
    )
    long = tts_mod.speak(long_text)
    assert long["ok"] and short["ok"]
    assert _wav_info(long["wav_path"])["frames"] > _wav_info(short["wav_path"])["frames"]


def test_stt_roundtrip_recovers_words():
    phrase = "Hello world, this is a voice pipeline test"
    spoken = tts_mod.speak(f"{phrase} {_unique('rt')}.")
    assert spoken["ok"] is True, spoken
    out = stt_mod.transcribe(spoken["wav_path"], model_size="tiny")
    assert out["ok"] is True, out
    assert out["engine"].startswith("faster-whisper"), out
    lowered = out["text"].lower()
    for word in ("hello", "world", "voice", "pipeline", "test"):
        assert word in lowered, f"missing {word!r} in {out['text']!r}"


def test_stt_long_input_keeps_keywords():
    phrase = (
        "The desktop assistant records audio from the microphone then "
        "transcribes speech to text and sends the result to Hermes"
    )
    spoken = tts_mod.speak(phrase)
    assert spoken["ok"] is True, spoken
    out = stt_mod.transcribe(spoken["wav_path"], model_size="tiny")
    assert out["ok"] is True, out
    lowered = out["text"].lower()
    hits = sum(1 for w in ("desktop", "microphone", "transcribes", "speech", "hermes")
               if w in lowered)
    assert hits >= 4, f"too few keywords in {out['text']!r}"


def test_stt_silence_yields_honest_empty(tmp_path):
    """Silence must transcribe to empty — never hallucinated words."""
    silent = tmp_path / "silence.wav"
    with wave.open(str(silent), "wb") as wf:
        wf.setnchannels(1)
        wf.setsampwidth(2)
        wf.setframerate(16000)
        wf.writeframes(b"\x00" * 16000 * 2)
    out = stt_mod.transcribe(str(silent), model_size="tiny")
    assert out["ok"] is True, out
    assert out["engine"] == "silence-gate", out
    assert out["text"].strip() == "", f"silence hallucinated: {out['text']!r}"


def test_voice_errors_are_explicit(tmp_path):
    missing = stt_mod.transcribe(str(tmp_path / "nope.wav"))
    assert missing["ok"] is False and missing["code"] == "FILE_NOT_FOUND"

    corrupt = tmp_path / "corrupt.wav"
    corrupt.write_bytes(b"\x00\x01\x02not a wav at all\xff\xfe" * 64)
    bad = stt_mod.transcribe(str(corrupt))
    assert bad["ok"] is False, bad

    empty = tts_mod.speak("   ")
    assert empty["ok"] is False and empty["code"] == "EMPTY_TEXT"
