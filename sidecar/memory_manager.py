"""pain ai — Memory Adapter over Hermes Native MemoryStore (memory_manager.py)

PHASE 7: Hermes native memory is the single source of truth. This module is a
THIN adapter — no memory engine of its own. Every read goes through a fresh
``tools.memory_tool.load_on_disk_store()`` (the Hermes-sanctioned accessor
for non-agent contexts: gateway, Desktop, /memory); every write goes through
``MemoryStore.apply_batch`` / ``remove`` with Hermes budgets, file locks,
atomic renames, drift protection, and strict-scope threat scanning intact.

Same files (MEMORY.md / USER.md under HERMES_HOME == ~/.pain-ai), same
external shapes ({name,target,path,content,char_count,char_limit,
within_limit} / {ok,...}) as the retired custom implementation, so the
bridge and UI are unchanged.

Whole-file UI edits map onto native entry ops: the new text is parsed with
Hermes ``_parse_entries`` and diffed against live entries into one atomic
batch (removes first, then adds). Clearing a non-empty store uses sequential
single ``remove()`` calls — the deliberate-wipe path Hermes blesses (batch
wipe of a non-empty store is refused by design, #103419). Native refusals
(over-budget, drift with .bak snapshot, threat hit, ambiguity) surface
verbatim as {ok: False} — never silently rewritten.

Normalization note: entries are stripped §-delimited segments, so a trailing
newline is not preserved across a native round-trip. First native save of a
legacy file normalizes it cosmetically; content is otherwise identical.

Operator UI writes call the store directly (as the legacy adapter wrote files
directly): the write-approval staging gate in memory_tool.py governs
AGENT-initiated tool writes, not explicit human edits.
"""

from pathlib import Path
from typing import Any, Dict, List, Optional

from tools.memory_tool import get_memory_dir, load_on_disk_store
from tools.memory_tool_store import ENTRY_DELIMITER, MemoryStore

# Caps mirror the Hermes MemoryStore constructor defaults; enforced BY that
# constructor at runtime (never by these literals). The parity test pins them.
_HERMES_DEFAULTS = MemoryStore()
MEMORY_CHAR_LIMIT = _HERMES_DEFAULTS.memory_char_limit  # 2200
USER_CHAR_LIMIT = _HERMES_DEFAULTS.user_char_limit  # 1375

DEFAULT_MEMORY_TEXT = (
    "# Long-Term Memory Notes\n\n"
    "- System: pain-ai desktop assistant initialized.\n"
    "- Mode: Local-first fail-closed security gate active.\n"
)
DEFAULT_USER_TEXT = (
    "# User Profile & Preferences\n\n"
    "- Role: Primary system operator\n"
    "- Tone: Concise, technical, and precise\n"
)


def _key_for(target: str) -> str:
    return "user" if "user" in (target or "").lower() else "memory"


def _name_for(key: str) -> str:
    return "USER.md" if key == "user" else "MEMORY.md"


def _limit_for(store: MemoryStore, key: str) -> int:
    return store.user_char_limit if key == "user" else store.memory_char_limit


def _path_for(key: str) -> Path:
    return get_memory_dir() / _name_for(key)


def _ensure_defaults(store: MemoryStore) -> None:
    """Create missing memory files via native add() (locked, atomic, scanned)."""
    for key, default in (("memory", DEFAULT_MEMORY_TEXT), ("user", DEFAULT_USER_TEXT)):
        if not _path_for(key).exists():
            try:
                store.add(key, default)
            except Exception:
                pass


def _content_of(store: MemoryStore, key: str) -> str:
    entries = store.user_entries if key == "user" else store.memory_entries
    return ENTRY_DELIMITER.join(entries)


def _parse_new(content: str) -> List[str]:
    """Parse UI text exactly the way Hermes parses files (deduped, ordered)."""
    seen: List[str] = []
    for entry in MemoryStore._parse_entries(content or ""):
        if entry not in seen:
            seen.append(entry)
    return seen


def _counts(key: str, name: str, content: str, limit: int) -> Dict[str, Any]:
    count = len(content)
    return {"name": name, "char_count": count, "char_limit": limit}


class MemoryManager:
    """Stateless facade: a fresh Hermes store per call (restart-safe)."""

    def get_memory(self, target: str = "memory") -> Dict[str, Any]:
        key = _key_for(target)
        name = _name_for(key)
        store = load_on_disk_store()
        _ensure_defaults(store)
        store.load_from_disk()
        limit = _limit_for(store, key)
        content = _content_of(store, key)
        count = len(content)
        return {
            "name": name,
            "target": target,
            "path": str(_path_for(key)),
            "content": content,
            "char_count": count,
            "char_limit": limit,
            "within_limit": count <= limit,
        }

    def update_memory(self, target: str, content: str) -> Dict[str, Any]:
        key = _key_for(target)
        name = _name_for(key)
        content = content or ""
        store = load_on_disk_store()
        _ensure_defaults(store)
        store.load_from_disk()
        limit = _limit_for(store, key)

        if not store.target_enabled(key):
            return {**_counts(key, name, content, limit),
                    "ok": False, "path": str(_path_for(key)), "target": target,
                    "error": f"Built-in {name} writes are disabled in memory config."}

        # Legacy fast-path wording preserved: hard-cap refusal before batching.
        if len(content) > limit:
            return {**_counts(key, name, content, limit),
                    "ok": False, "path": str(_path_for(key)), "target": target,
                    "error": (
                        f"Content length ({len(content)} chars) exceeds hard limit "
                        f"of {limit} characters for {name}."
                    )}

        old_entries = list(store.user_entries if key == "user" else store.memory_entries)
        new_entries = _parse_new(content)

        if not new_entries and not old_entries:
            return {**_counts(key, name, content, limit),
                    "ok": True, "path": str(_path_for(key)), "target": target}

        if not new_entries:
            # Whole-file clear of a non-empty store: sequential single
            # remove() calls (the deliberate-wipe path); batch wipe is
            # refused by Hermes design.
            for entry in old_entries:
                res = store.remove(key, entry)
                if not res.get("success"):
                    return {**_counts(key, name, content, limit),
                            "ok": False, "path": str(_path_for(key)), "target": target,
                            "error": res.get("error", "Failed to clear memory.")}
            return {**_counts(key, name, "", limit),
                    "ok": True, "path": str(_path_for(key)), "target": target}

        ops: List[Dict[str, Any]] = (
            [{"action": "remove", "old_text": e} for e in old_entries if e not in new_entries]
            + [{"action": "add", "content": e} for e in new_entries if e not in old_entries]
        )
        if not ops:
            return {**_counts(key, name, content, limit),
                    "ok": True, "path": str(_path_for(key)), "target": target}

        res = store.apply_batch(key, ops)
        if res.get("success"):
            # Re-read from disk (fresh store): the returned counts reflect
            # what Hermes actually persisted, not what the UI sent.
            saved_content = _content_of(load_on_disk_store(), key)
            count = len(saved_content)
            return {"name": name, "path": str(_path_for(key)),
                    "char_count": count, "char_limit": limit, "target": target,
                    "ok": True}
        return {**_counts(key, name, content, limit),
                "ok": False, "path": str(_path_for(key)), "target": target,
                "error": res.get("error", "Memory update refused.")}


# Global instance (stateless facade; kept for bridge API compatibility).
_default_memory_manager: Optional[MemoryManager] = None


def get_memory_manager() -> MemoryManager:
    global _default_memory_manager
    if _default_memory_manager is None:
        _default_memory_manager = MemoryManager()
    return _default_memory_manager
