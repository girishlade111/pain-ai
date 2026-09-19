#!/usr/bin/env python3
"""pain ai — Sentence Segmentation Engine (segment.py)

Splits text into discrete, spoken sentences for dual-render TTS audio generation
and real-time caption synchronization. Guards against false splits on common
abbreviations, decimal numbers, and closing quotation marks.
"""

import re
from typing import Any, Dict, List

# Common abbreviations that should not trigger a sentence break
ABBREVIATIONS = {
    "mr", "mrs", "ms", "dr", "prof", "sr", "jr", "st", "no", "vs",
    "eg", "ie", "etc", "am", "pm", "fig", "al", "dept", "gen", "gov",
    "jan", "feb", "mar", "apr", "jun", "jul", "aug", "sep", "sept", "oct", "nov", "dec"
}

# Regex for sentence terminators followed by whitespace or end of string
# Matches combinations of ., !, ?, …, optionally followed by closing quotes or brackets
PUNCT_SPLIT_REGEX = re.compile(r'([.!?…]+["\']?)(?:\s+|$)')


def _is_abbreviation(prefix: str) -> bool:
    """Checks if the word preceding a period is a known abbreviation or single initial."""
    cleaned = prefix.strip().lower().rstrip(".").replace(".", "")
    if not cleaned:
        return False
    if cleaned in ABBREVIATIONS:
        return True
    # Single uppercase initial (e.g. "J." in "J. Doe")
    if len(cleaned) == 1 and cleaned.isalpha():
        return True
    return False


def split_sentences(text: str) -> List[Dict[str, Any]]:
    """Splits text into a list of sentence items with 0-based indices.

    Returns:
        List of dicts: [{"i": 0, "text": "First sentence."}, ...]
    """
    if not text or not text.strip():
        return []

    raw_text = text.strip()

    # Step 1: Protect known abbreviations by replacing their period with a placeholder
    placeholder = "\u2060"  # Word joiner non-printing character

    def protect_match(m: re.Match) -> str:
        word = m.group(1)
        if _is_abbreviation(word):
            return word + placeholder
        return m.group(0)

    # Protect abbreviations like e.g. and i.e.
    protected = re.sub(r'\b([a-zA-Z]{1,5})\.', protect_match, raw_text)

    # Protect decimals: digit.digit
    protected = re.sub(r'(\d+)\.(\d+)', r'\1' + placeholder + r'\2', protected)

    # Step 2: Split on sentence boundaries: [.!?…]+ followed by space or newline
    parts = []
    last_idx = 0

    for m in PUNCT_SPLIT_REGEX.finditer(protected):
        end_pos = m.end()
        candidate = protected[last_idx:end_pos].strip()
        if candidate:
            parts.append(candidate)
        last_idx = end_pos

    # Capture any remainder text
    if last_idx < len(protected):
        remainder = protected[last_idx:].strip()
        if remainder:
            parts.append(remainder)

    # Step 3: Restore placeholders and assemble sentence list
    sentences: List[Dict[str, Any]] = []
    item_idx = 0

    for chunk in parts:
        restored = chunk.replace(placeholder, ".").strip()
        if restored:
            sentences.append({
                "i": item_idx,
                "text": restored
            })
            item_idx += 1

    # Fallback if no punctuation splitting occurred but text exists
    if not sentences and raw_text:
        sentences.append({"i": 0, "text": raw_text})

    return sentences


if __name__ == "__main__":
    import json
    import sys

    sample = sys.argv[1] if len(sys.argv) > 1 else "Hello world! Dr. Smith arrived at 3.14 p.m. Can we begin? Yes."
    result = split_sentences(sample)
    print(json.dumps(result, indent=2))
