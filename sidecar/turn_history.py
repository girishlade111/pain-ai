#!/usr/bin/env python3
"""pain ai — Turn History Recorder (turn_history.py)

Single owner for persisting conversation turns into the chat session store
(sidecar state.db). Used by the chat endpoint AND the cron scheduler so both
paths record identically. Recording never raises: history must never break a
turn (Phase 8 contract).
"""

import logging
from typing import Optional

logger = logging.getLogger("turn_history")


def ensure_session(db, session_id: str, first_text: str, source: str = "user") -> None:
    """Create the session row once (title from first text). Re-inserts never
    overwrite (StateDB uses DO NOTHING on conflict), so renames survive."""
    try:
        from session_search import make_title
    except ImportError:
        from sidecar.session_search import make_title
    if db.get_session(session_id) is None:
        db.insert_session(session_id, make_title(first_text), source=source)


def record_message(db, session_id: str, role: str, content: str) -> None:
    """Append one message; skips blanks. Never raises."""
    try:
        if (content or "").strip():
            db.insert_message(session_id, role, content)
    except Exception as exc:
        logger.warning(f"session history record failed: {exc}")


def record_turn(db, session_id: str, user_text: str,
                assistant_text: Optional[str] = None,
                source: str = "user") -> None:
    """Record a full turn: ensure session, user message, assistant reply."""
    try:
        ensure_session(db, session_id, user_text, source=source)
        record_message(db, session_id, "user", user_text)
        if assistant_text is not None:
            record_message(db, session_id, "assistant", assistant_text)
    except Exception as exc:
        logger.warning(f"session history record failed: {exc}")
