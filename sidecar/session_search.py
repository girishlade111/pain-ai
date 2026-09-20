"""pain ai — Session Search & SQLite State DB Engine (state.db)
Source of truth: PRD.md §4.3 + hermes-map.md §6 + tools/session_search_tool.py
SSOT (Phase 1): This module owns the sidecar HTTP transport over
~/.pain-ai/state.db (messages_fts unicode61). Hermes session_search_tool +
hermes_state_* own upstream agent-loop recall; Rust memory_cron.session_search
holds no rows and delegates here by design.
"""

import os
import re
import sqlite3
import time
from pathlib import Path
from typing import Dict, Any, List, Optional, Tuple

_FTS5_SPECIAL_CHARS = '+{}():"^@/#&|~[]<>,;!?$=\\\''
_FTS5_SPECIAL_RE = re.compile(f"[{re.escape(_FTS5_SPECIAL_CHARS)}]")

SCHEMA_SQL = """
CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    title TEXT,
    source TEXT NOT NULL DEFAULT 'user',
    started_at REAL NOT NULL,
    ended_at REAL,
    message_count INTEGER DEFAULT 0
);

CREATE TABLE IF NOT EXISTS messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    role TEXT NOT NULL,
    content TEXT,
    tool_name TEXT,
    timestamp REAL NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_sessions_started ON sessions(started_at DESC);
CREATE INDEX IF NOT EXISTS idx_messages_session ON messages(session_id, timestamp);
"""

FTS_SCHEMA_SQL = """
CREATE VIRTUAL TABLE IF NOT EXISTS messages_fts USING fts5(
    content,
    tool_name,
    role,
    session_id UNINDEXED,
    tokenize='unicode61'
);

CREATE TRIGGER IF NOT EXISTS messages_fts_ai AFTER INSERT ON messages BEGIN
    INSERT INTO messages_fts(rowid, content, tool_name, role, session_id)
    VALUES (new.id, new.content, new.tool_name, new.role, new.session_id);
END;

-- NOTE: plain DELETE (not the FTS5 'delete'-command INSERT) maintains the
-- index. The command form raises "SQL logic error" on current runtimes
-- (verified SQLite 3.50.4), which made every message/session delete fail.
CREATE TRIGGER IF NOT EXISTS messages_fts_ad AFTER DELETE ON messages BEGIN
    DELETE FROM messages_fts WHERE rowid = old.id;
END;

CREATE TRIGGER IF NOT EXISTS messages_fts_au AFTER UPDATE ON messages BEGIN
    DELETE FROM messages_fts WHERE rowid = old.id;
    INSERT INTO messages_fts(rowid, content, tool_name, role, session_id)
    VALUES (new.id, new.content, new.tool_name, new.role, new.session_id);
END;
"""

# Migrates databases created with the broken 'delete'-command triggers:
# DROP + recreate runs on every StateDB construction (idempotent).
TRIGGER_MIGRATION_SQL = """
DROP TRIGGER IF EXISTS messages_fts_ad;
DROP TRIGGER IF EXISTS messages_fts_au;
DROP TRIGGER IF EXISTS messages_fts_ai;
"""


def get_pain_ai_home() -> Path:
    home_env = os.environ.get("PAIN_AI_HOME")
    if home_env:
        return Path(home_env)
    return Path.home() / ".pain-ai"


def get_state_db_path() -> Path:
    db_dir = get_pain_ai_home()
    db_dir.mkdir(parents=True, exist_ok=True)
    return db_dir / "state.db"


def escape_fts_query(query: str) -> str:
    """Escapes special FTS5 characters to avoid query syntax errors."""
    tokens = [t.strip() for t in query.split() if t.strip()]
    if not tokens:
        return '""'
    escaped_tokens = []
    for token in tokens:
        cleaned = _FTS5_SPECIAL_RE.sub(" ", token).strip()
        if cleaned:
            # Wrap in quotes for safe exact/prefix matching
            escaped_tokens.append(f'"{cleaned}"*')
    return " ".join(escaped_tokens) if escaped_tokens else '""'


class StateDB:
    def __init__(self, db_path: Optional[Path] = None):
        self.db_path = db_path or get_state_db_path()
        self._init_db()

    def _get_connection(self) -> sqlite3.Connection:
        conn = sqlite3.connect(str(self.db_path), timeout=10.0)
        conn.row_factory = sqlite3.Row
        conn.execute("PRAGMA foreign_keys = ON")
        return conn

    def _init_db(self):
        with self._get_connection() as conn:
            conn.executescript(SCHEMA_SQL)
            # Replace any legacy broken delete triggers before creating.
            conn.executescript(TRIGGER_MIGRATION_SQL)
            conn.executescript(FTS_SCHEMA_SQL)
            conn.commit()

    def insert_session(
        self,
        session_id: str,
        title: str,
        source: str = "user",
        started_at: Optional[float] = None,
    ):
        ts = started_at or time.time()
        with self._get_connection() as conn:
            # DO NOTHING on conflict: a re-insert (e.g. continuing a renamed
            # session) must never clobber the stored title.
            conn.execute(
                """
                INSERT INTO sessions (id, title, source, started_at, message_count)
                VALUES (?, ?, ?, ?, 0)
                ON CONFLICT(id) DO NOTHING
                """,
                (session_id, title, source, ts),
            )
            conn.commit()

    def get_session(self, session_id: str) -> Optional[Dict[str, Any]]:
        with self._get_connection() as conn:
            cursor = conn.execute(
                "SELECT id, title, source, started_at, message_count FROM sessions WHERE id = ?",
                (session_id,),
            )
            row = cursor.fetchone()
            return dict(row) if row else None

    def rename_session(self, session_id: str, title: str) -> bool:
        title = (title or "").strip()
        if not title:
            raise ValueError("Session title must not be empty")
        title = title[:200]
        with self._get_connection() as conn:
            cursor = conn.execute(
                "UPDATE sessions SET title = ? WHERE id = ?",
                (title, session_id),
            )
            conn.commit()
            return cursor.rowcount > 0

    def delete_session(self, session_id: str) -> bool:
        with self._get_connection() as conn:
            cursor = conn.execute("DELETE FROM sessions WHERE id = ?", (session_id,))
            conn.commit()
            return cursor.rowcount > 0

    def insert_message(
        self,
        session_id: str,
        role: str,
        content: str,
        tool_name: Optional[str] = None,
        timestamp: Optional[float] = None,
    ) -> int:
        ts = timestamp or time.time()
        with self._get_connection() as conn:
            cursor = conn.execute(
                """
                INSERT INTO messages (session_id, role, content, tool_name, timestamp)
                VALUES (?, ?, ?, ?, ?)
                """,
                (session_id, role, content, tool_name, ts),
            )
            msg_id = cursor.lastrowid
            conn.execute(
                """
                UPDATE sessions SET message_count = message_count + 1 WHERE id = ?
                """,
                (session_id,),
            )
            conn.commit()
            return msg_id

    def search_sessions(
        self,
        query: str,
        session_id: Optional[str] = None,
        limit: int = 20,
    ) -> List[Dict[str, Any]]:
        """3-Step Session Search: Discovery (FTS5 + BM25 ranking), Scroll, and Read."""
        if not query or not query.strip():
            return []

        fts_query = escape_fts_query(query)
        start_time = time.perf_counter()

        sql = """
            SELECT
                m.id AS message_id,
                m.session_id,
                m.role,
                m.content,
                m.tool_name,
                m.timestamp,
                s.title AS session_title,
                s.source AS session_source,
                s.started_at AS session_started,
                snippet(messages_fts, 0, '<mark>', '</mark>', '...', 24) AS snippet,
                bm25(messages_fts) AS rank_score
            FROM messages_fts
            JOIN messages m ON m.id = messages_fts.rowid
            JOIN sessions s ON s.id = m.session_id
            WHERE messages_fts MATCH ?
        """
        params: List[Any] = [fts_query]

        if session_id:
            sql += " AND m.session_id = ?"
            params.append(session_id)

        # Demote cron/background sources below interactive sessions
        sql += """
            ORDER BY
                CASE WHEN s.source = 'cron' THEN 1 ELSE 0 END ASC,
                rank_score ASC,
                m.timestamp DESC
            LIMIT ?
        """
        params.append(limit)

        results = []
        with self._get_connection() as conn:
            cursor = conn.execute(sql, params)
            for row in cursor.fetchall():
                results.append({
                    "message_id": row["message_id"],
                    "session_id": row["session_id"],
                    "session_title": row["session_title"] or f"Session {row['session_id'][:8]}",
                    "session_source": row["session_source"],
                    "session_started": row["session_started"],
                    "role": row["role"],
                    "tool_name": row["tool_name"],
                    "content": row["content"],
                    "snippet": row["snippet"],
                    "rank_score": float(row["rank_score"]),
                    "timestamp": row["timestamp"],
                })

        duration_ms = (time.perf_counter() - start_time) * 1000.0
        # Telemetry: verify <500ms latency requirement
        return results

    def get_session_messages(self, session_id: str, limit: int = 100) -> List[Dict[str, Any]]:
        """Step 3 (Read): Reads session messages in chronological order."""
        with self._get_connection() as conn:
            cursor = conn.execute(
                """
                SELECT id, session_id, role, content, tool_name, timestamp
                FROM messages
                WHERE session_id = ?
                ORDER BY timestamp ASC, id ASC
                LIMIT ?
                """,
                (session_id, limit),
            )
            return [dict(r) for r in cursor.fetchall()]

    def list_recent_sessions(self, limit: int = 20) -> List[Dict[str, Any]]:
        with self._get_connection() as conn:
            cursor = conn.execute(
                """
                SELECT id, title, source, started_at, message_count
                FROM sessions
                ORDER BY started_at DESC
                LIMIT ?
                """,
                (limit,),
            )
            return [dict(r) for r in cursor.fetchall()]

    def list_sessions(self, limit: int = 50) -> List[Dict[str, Any]]:
        """Chat session summaries: id/title/createdAt/updatedAt/messageCount/
        lastMessage + metadata. updatedAt derives from the latest message
        (no schema migration); sessions without messages report started_at."""
        with self._get_connection() as conn:
            cursor = conn.execute(
                """
                SELECT
                    s.id AS id,
                    s.title AS title,
                    s.source AS source,
                    s.started_at AS createdAt,
                    COALESCE(
                        (SELECT MAX(m.timestamp) FROM messages m WHERE m.session_id = s.id),
                        s.started_at
                    ) AS updatedAt,
                    (SELECT COUNT(*) FROM messages m WHERE m.session_id = s.id) AS messageCount,
                    (SELECT m.role FROM messages m WHERE m.session_id = s.id
                     ORDER BY m.timestamp DESC, m.id DESC LIMIT 1) AS lastRole,
                    (SELECT substr(m.content, 1, 200) FROM messages m WHERE m.session_id = s.id
                     ORDER BY m.timestamp DESC, m.id DESC LIMIT 1) AS lastMessage
                FROM sessions s
                ORDER BY updatedAt DESC, s.started_at DESC
                LIMIT ?
                """,
                (limit,),
            )
            out = []
            for row in cursor.fetchall():
                item = dict(row)
                item["metadata"] = {"source": item.pop("source")}
                out.append(item)
            return out


def make_title(text: str, fallback: str = "New chat") -> str:
    """Deterministic session title from the first user message (no LLM call)."""
    line = (text or "").strip().splitlines()[0] if (text or "").strip() else ""
    line = " ".join(line.split())
    if not line:
        return fallback
    return line if len(line) <= 60 else line[:57].rstrip() + "..."


# Global state DB instance
_default_state_db: Optional[StateDB] = None

def get_state_db() -> StateDB:
    global _default_state_db
    if _default_state_db is None:
        _default_state_db = StateDB()
    return _default_state_db
