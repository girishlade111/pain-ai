#!/usr/bin/env python3
"""pain ai — Session Search 200-Session Benchmark Seed Script
Creates 200 distinct sessions with realistic conversation transcripts
and indexes them in SQLite state.db for FTS5 latency benchmarking.
"""

import sys
import time
from pathlib import Path

# Add sidecar directory to python path
sidecar_dir = Path(__file__).resolve().parent.parent.parent / "sidecar"
if str(sidecar_dir) not in sys.path:
    sys.path.insert(0, str(sidecar_dir))

from session_search import StateDB

TOPICS = [
    ("Refactor Tauri Gate", "Can you review the blocklist patterns in gate.rs and ensure rm -rf is caught?"),
    ("Database Optimization", "We need an FTS5 index over messages with unicode61 tokenizer for fast search."),
    ("Invoice Q3 Triaging", "Please parse the attached invoice-2026-q3.pdf and extract total billing amount."),
    ("Python Sidecar Watchdog", "The FastAPI bridge needs an orphan-reaper process watching parent PID."),
    ("Windows UI Automation", "Traverse the UIA accessibility tree and locate AutomationID btnSave."),
    ("Linux AT-SPI Twin", "Verify org.a11y.Bus D-Bus connectivity under GNOME X11 session."),
    ("Screen Context Downsampling", "Ensure high-resolution screenshots are capped to 1568px bound using Lanczos3."),
    ("Speech-to-Text Pipeline", "Transcribe user voice memo using faster-whisper and Silero VAD trimming."),
    ("Text-to-Speech Captions", "Synchronize Piper TTS speech synthesis with simultaneous real-time caption bar."),
    ("Quarantine Security Audit", "Scan incoming skills tap scripts for API keys and destructive shell commands."),
]


def seed_test_database(db: StateDB, count: int = 200):
    """Seeds `count` sessions into the provided StateDB instance."""
    now = time.time()
    for i in range(count):
        topic_title, sample_prompt = TOPICS[i % len(TOPICS)]
        session_id = f"session-bench-{i:04d}"
        source = "cron" if (i % 7 == 0) else "user"
        title = f"{topic_title} #{i + 1}"
        db.insert_session(session_id=session_id, title=title, source=source, started_at=now - (i * 3600))

        # Insert 3 messages per session
        db.insert_message(session_id=session_id, role="user", content=sample_prompt, timestamp=now - (i * 3600))
        db.insert_message(
            session_id=session_id,
            role="assistant",
            content=f"Understood. Analyzing parameters for {topic_title.lower()} and checking constraints.",
            timestamp=now - (i * 3600) + 1,
        )
        db.insert_message(
            session_id=session_id,
            role="assistant",
            content=f"Operation complete. State persisted and verified for {topic_title.lower()}.",
            tool_name="terminal" if i % 2 == 0 else "read_file",
            timestamp=now - (i * 3600) + 2,
        )


if __name__ == "__main__":
    db = StateDB()
    print("Seeding 200 benchmark sessions into default state.db...")
    start = time.perf_counter()
    seed_test_database(db, 200)
    elapsed = time.perf_counter() - start
    print(f"Successfully seeded 200 sessions in {elapsed:.3f}s")
