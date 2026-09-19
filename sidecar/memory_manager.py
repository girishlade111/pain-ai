"""pain ai — Long-Term Memory Manager (MEMORY.md + USER.md)
Source of truth: PRD.md §4.3 + hermes-map.md §6 + tools/memory_tool.py
"""

from pathlib import Path
from typing import Dict, Any, Optional, Tuple
import os

MEMORY_CHAR_LIMIT = 2200  # Hard cap ~550 tokens
USER_CHAR_LIMIT = 1375    # Hard cap ~350 tokens
DEFAULT_NUDGE_INTERVAL = 10


def get_pain_ai_home() -> Path:
    """Returns ~/.pain-ai directory."""
    home_env = os.environ.get("PAIN_AI_HOME")
    if home_env:
        return Path(home_env)
    return Path.home() / ".pain-ai"


def get_memories_dir() -> Path:
    mem_dir = get_pain_ai_home() / "memories"
    mem_dir.mkdir(parents=True, exist_ok=True)
    return mem_dir


class MemoryManager:
    def __init__(self, memories_dir: Optional[Path] = None):
        self.dir = memories_dir or get_memories_dir()
        self.dir.mkdir(parents=True, exist_ok=True)
        self.memory_file = self.dir / "MEMORY.md"
        self.user_file = self.dir / "USER.md"
        self._ensure_defaults()
        self.turn_counter = 0
        self.nudge_interval = DEFAULT_NUDGE_INTERVAL

    def _ensure_defaults(self):
        if not self.memory_file.exists():
            default_mem = (
                "# Long-Term Memory Notes\n\n"
                "- System: pain-ai desktop assistant initialized.\n"
                "- Mode: Local-first fail-closed security gate active.\n"
            )
            self.memory_file.write_text(default_mem, encoding="utf-8")

        if not self.user_file.exists():
            default_user = (
                "# User Profile & Preferences\n\n"
                "- Role: Primary system operator\n"
                "- Tone: Concise, technical, and precise\n"
            )
            self.user_file.write_text(default_user, encoding="utf-8")

    def get_memory(self, target: str = "memory") -> Dict[str, Any]:
        """Returns target memory content, character count, and cap."""
        target_lower = target.lower()
        if "user" in target_lower:
            path = self.user_file
            limit = USER_CHAR_LIMIT
            name = "USER.md"
        else:
            path = self.memory_file
            limit = MEMORY_CHAR_LIMIT
            name = "MEMORY.md"

        if not path.exists():
            content = ""
        else:
            content = path.read_text(encoding="utf-8")

        return {
            "name": name,
            "target": target,
            "path": str(path),
            "content": content,
            "char_count": len(content),
            "char_limit": limit,
            "within_limit": len(content) <= limit,
        }

    def update_memory(self, target: str, content: str) -> Dict[str, Any]:
        """Updates target memory content if within character limit."""
        target_lower = target.lower()
        if "user" in target_lower:
            path = self.user_file
            limit = USER_CHAR_LIMIT
            name = "USER.md"
        else:
            path = self.memory_file
            limit = MEMORY_CHAR_LIMIT
            name = "MEMORY.md"

        char_count = len(content)
        if char_count > limit:
            return {
                "ok": False,
                "error": f"Content length ({char_count} chars) exceeds hard limit of {limit} characters for {name}.",
                "char_count": char_count,
                "char_limit": limit,
                "target": target,
            }

        path.write_text(content, encoding="utf-8")
        return {
            "ok": True,
            "name": name,
            "path": str(path),
            "char_count": char_count,
            "char_limit": limit,
            "target": target,
        }

    def increment_turn(self) -> bool:
        """Increments turn counter; returns True if memory nudge is triggered."""
        self.turn_counter += 1
        return (self.turn_counter % self.nudge_interval) == 0

    def reset_nudge(self):
        self.turn_counter = 0


# Global instance
_default_memory_manager: Optional[MemoryManager] = None

def get_memory_manager() -> MemoryManager:
    global _default_memory_manager
    if _default_memory_manager is None:
        _default_memory_manager = MemoryManager()
    return _default_memory_manager
