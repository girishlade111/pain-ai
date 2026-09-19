"""pain ai — Context Window Compressor (/compress + auto-compaction)
Source of truth: PRD.md §4.3 + hermes-map.md §6 + agent/context_compressor.py
SSOT (Phase 1): This module owns sidecar HTTP compression (head/tail protect).
Hermes agent/context_compressor.py owns upstream loop semantics; Rust
memory_cron.context_compress is a desktop estimate only.
"""

from typing import List, Dict, Any, Tuple

THRESHOLD_PERCENT = 0.80
MICRO_COMPACT_DEFRAG_THRESHOLD_TOKENS = 2000
CHARS_PER_TOKEN = 4


def estimate_tokens(text: str) -> int:
    if not text:
        return 0
    return max(1, len(text) // CHARS_PER_TOKEN)


def estimate_messages_tokens(messages: List[Dict[str, Any]]) -> int:
    total = 0
    for m in messages:
        total += estimate_tokens(m.get("content", "") or "")
        if m.get("tool_name"):
            total += estimate_tokens(m.get("tool_name", ""))
    return total


class ContextCompressor:
    def __init__(self, threshold_percent: float = THRESHOLD_PERCENT):
        self.threshold_percent = threshold_percent

    def should_compress(self, current_tokens: int, context_limit: int = 128000) -> bool:
        return (current_tokens / float(context_limit)) >= self.threshold_percent

    def compress_messages(
        self,
        messages: List[Dict[str, Any]],
        context_limit: int = 128000,
        force: bool = False,
    ) -> Dict[str, Any]:
        """
        Compresses middle turns into a summary block while strictly protecting
        the initial user prompt (head) and recent interaction turns (tail).
        """
        orig_tokens = estimate_messages_tokens(messages)
        if len(messages) <= 4:
            return {
                "compressed": False,
                "reason": "Not enough turns to compress safely.",
                "original_tokens": orig_tokens,
                "compressed_tokens": orig_tokens,
                "messages": messages,
            }

        if not force and not self.should_compress(orig_tokens, context_limit):
            return {
                "compressed": False,
                "reason": f"Tokens ({orig_tokens}) below {int(self.threshold_percent * 100)}% threshold.",
                "original_tokens": orig_tokens,
                "compressed_tokens": orig_tokens,
                "messages": messages,
            }

        # Keep first turn (head) and last 2 turns (tail)
        head = messages[:1]
        tail = messages[-2:]
        middle = messages[1:-2]

        # Extract topics / intents from middle turns
        summaries = []
        for m in middle:
            role = m.get("role", "user")
            content = m.get("content", "")
            snippet = content[:80].replace("\n", " ")
            summaries.append(f"{role}: {snippet}...")

        summary_text = (
            "[CONTEXT COMPACTION SUMMARY]\n"
            f"The prior {len(middle)} turns were summarized to preserve context window:\n"
            + "\n".join(f"- {s}" for s in summaries[:5])
        )

        compacted_turn = {
            "role": "system",
            "content": summary_text,
            "_compressed_summary": 1,
        }

        compacted_messages = head + [compacted_turn] + tail
        new_tokens = estimate_messages_tokens(compacted_messages)

        return {
            "compressed": True,
            "original_tokens": orig_tokens,
            "compressed_tokens": new_tokens,
            "tokens_saved": max(0, orig_tokens - new_tokens),
            "messages": compacted_messages,
        }
