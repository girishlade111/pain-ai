"""pain ai — Pytest Suite for Chat Session Lifecycle (Phase 8)

Covers the required matrix: Chat A / Chat B isolation, restart persistence,
rename durability, delete cascade, list summaries, and title generation.
Isolated temp databases only.
"""

import sys
import time
from pathlib import Path

root_dir = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(root_dir / "sidecar"))

from session_search import StateDB, make_title


def _seed_two_chats(db: StateDB):
    db.insert_session("chat-A", "Alpha topic discussion", source="user")
    db.insert_message("chat-A", "user", "How do I parse JSON in Python?")
    db.insert_message("chat-A", "assistant", "Use the json module: json.loads(...).")
    db.insert_session("chat-B", "Beta build notes", source="user")
    db.insert_message("chat-B", "user", "Remind me to water the plants")
    db.insert_message("chat-B", "assistant", "Reminder set for tomorrow morning.")


def test_chat_a_b_isolation(tmp_path):
    db = StateDB(tmp_path / "s.db")
    _seed_two_chats(db)

    a_msgs = db.get_session_messages("chat-A")
    assert [m["role"] for m in a_msgs] == ["user", "assistant"]
    assert all(m["session_id"] == "chat-A" for m in a_msgs)
    assert "plants" not in " ".join(m["content"] for m in a_msgs)

    b_msgs = db.get_session_messages("chat-B")
    assert len(b_msgs) == 2
    assert "JSON" not in " ".join(m["content"] for m in b_msgs)

    # Scoped FTS: a query matching B returns nothing under A.
    assert db.search_sessions("plants", session_id="chat-A") == []
    assert len(db.search_sessions("plants", session_id="chat-B")) == 1


def test_restart_keeps_both_chats_accessible(tmp_path):
    path = tmp_path / "s.db"
    db = StateDB(path)
    _seed_two_chats(db)
    del db

    # Simulate app restart: brand-new handle on the same file.
    restarted = StateDB(path)
    sessions = restarted.list_sessions()
    assert {s["id"] for s in sessions} == {"chat-A", "chat-B"}
    assert len(restarted.get_session_messages("chat-A")) == 2
    assert len(restarted.get_session_messages("chat-B")) == 2


def test_list_summaries_carry_required_model(tmp_path):
    db = StateDB(tmp_path / "s.db")
    _seed_two_chats(db)
    by_id = {s["id"]: s for s in db.list_sessions()}
    for sid in ("chat-A", "chat-B"):
        s = by_id[sid]
        assert s["title"] and s["createdAt"] > 0
        assert s["updatedAt"] >= s["createdAt"]
        assert s["messageCount"] == 2
        assert s["lastMessage"] and s["metadata"] == {"source": "user"}
    # Most recently active first.
    assert [s["id"] for s in db.list_sessions()][0] == "chat-B"


def test_rename_persists_and_survives_reinsert(tmp_path):
    db = StateDB(tmp_path / "s.db")
    _seed_two_chats(db)
    assert db.rename_session("chat-A", "Renamed Alpha") is True
    assert db.get_session("chat-A")["title"] == "Renamed Alpha"
    # Continuing the session (re-insert) must not clobber the custom title.
    db.insert_session("chat-A", "Auto Title Attempt", source="user")
    assert db.get_session("chat-A")["title"] == "Renamed Alpha"
    assert db.rename_session("chat-missing", "x") is False
    import pytest
    with pytest.raises(ValueError):
        db.rename_session("chat-A", "   ")


def test_delete_cascades_messages_and_fts(tmp_path):
    db = StateDB(tmp_path / "s.db")
    _seed_two_chats(db)
    assert db.delete_session("chat-A") is True
    assert db.get_session("chat-A") is None
    assert db.get_session_messages("chat-A") == []
    assert db.search_sessions("parse JSON") == []
    # The other chat is untouched.
    assert len(db.get_session_messages("chat-B")) == 2
    assert db.delete_session("chat-A") is False


def test_make_title_cases():
    assert make_title("") == "New chat"
    assert make_title("   ") == "New chat"
    assert make_title("Short question?") == "Short question?"
    assert make_title("First line here\nSecond line here") == "First line here"
    long_text = "x" * 100
    titled = make_title(long_text)
    assert len(titled) == 60 and titled.endswith("...")
    assert make_title("  spaced   out   ") == "spaced out"


def test_legacy_broken_triggers_migrated_in_place(tmp_path):
    """Databases created with the old FTS5 'delete'-command triggers (every
    message/session delete failed) are repaired on open: deletes work and
    the index stays consistent for search."""
    import sqlite3

    path = tmp_path / "legacy.db"
    db = StateDB(path)
    db.insert_session("legacy", "Legacy chat", source="user")
    db.insert_message("legacy", "user", "unique legacy phrase for search")

    # Rewind the triggers to the broken legacy bodies.
    with db._get_connection() as conn:
        conn.executescript(
            """
            DROP TRIGGER IF EXISTS messages_fts_ad;
            DROP TRIGGER IF EXISTS messages_fts_au;
            CREATE TRIGGER messages_fts_ad AFTER DELETE ON messages BEGIN
                INSERT INTO messages_fts(messages_fts, rowid, content, tool_name, role, session_id)
                VALUES ('delete', old.id, old.content, old.tool_name, old.role, old.session_id);
            END;
            """
        )
        conn.commit()
    with db._get_connection() as conn:
        try:
            conn.execute("DELETE FROM messages")
            conn.commit()
            broken = False
        except sqlite3.OperationalError:
            broken = True
            conn.rollback()
    assert broken, "expected the legacy trigger to fail on this runtime"

    # Reopening migrates the triggers; delete + search stay consistent.
    migrated = StateDB(path)
    assert migrated.delete_session("legacy") is True
    assert migrated.get_session_messages("legacy") == []
    assert migrated.search_sessions("unique legacy phrase") == []
