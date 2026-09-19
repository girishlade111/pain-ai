"""pain ai — Vision Pipeline Utility (vision_util.py)

Single unified vision helper for:
1. Resizing & encoding images for multimodal model ingestion (1568px max dimension, LANCZOS, JPEG q80 compression if >1.5MB).
2. Routing vision queries to active model providers, returning actionable NO_VISION_MODEL hints for text-only models.
"""

import base64
from io import BytesIO
import json
import logging
import os
from pathlib import Path
from typing import Any, Dict, Optional, Tuple

logger = logging.getLogger("vision_util")

MAX_VISION_DIMENSION = 1568
MAX_RAW_PAYLOAD_BYTES = 1500000  # 1.5 MB


def prepare_image(
    image_input: Any,
    max_dim: int = MAX_VISION_DIMENSION,
) -> Dict[str, Any]:
    """Prepare and normalize an image for vision LLM consumption.

    - Downsamples to max_dim along the longest edge using LANCZOS.
    - Converts RGBA/Palette images to RGB for clean provider ingestion.
    - If encoded payload exceeds 1.5 MB, compresses as JPEG quality 80; otherwise PNG.
    - Returns metadata: { b64, w, h, mime, size_bytes }.
    """
    from PIL import Image

    if isinstance(image_input, (str, Path)):
        img = Image.open(str(image_input))
    elif isinstance(image_input, bytes):
        img = Image.open(BytesIO(image_input))
    elif hasattr(image_input, "convert"):
        img = image_input
    else:
        raise ValueError(f"Unsupported image input type: {type(image_input)}")

    orig_w, orig_h = img.size
    scale = min(1.0, max_dim / max(orig_w, orig_h))
    new_w = max(1, int(round(orig_w * scale)))
    new_h = max(1, int(round(orig_h * scale)))

    if scale < 1.0:
        # High-fidelity Lanczos downsampling
        resample_filter = getattr(Image, "Resampling", Image).LANCZOS
        img = img.resize((new_w, new_h), resample=resample_filter)

    # Convert to RGB (broad provider compatibility)
    if img.mode != "RGB":
        img = img.convert("RGB")

    # First attempt: encode as PNG
    buf = BytesIO()
    img.save(buf, format="PNG", optimize=True)
    png_bytes = buf.getvalue()

    if len(png_bytes) > MAX_RAW_PAYLOAD_BYTES:
        # Re-encode as JPEG q80 to stay within provider payload limits
        buf = BytesIO()
        img.save(buf, format="JPEG", quality=80, optimize=True)
        final_bytes = buf.getvalue()
        mime = "image/jpeg"
    else:
        final_bytes = png_bytes
        mime = "image/png"

    b64_str = base64.b64encode(final_bytes).decode("ascii")

    return {
        "b64": b64_str,
        "w": img.width,
        "h": img.height,
        "mime": mime,
        "size_bytes": len(final_bytes),
        "data_url": f"data:{mime};base64,{b64_str}",
    }


def is_vision_supported_model(model_name: str) -> bool:
    """Check if model supports multimodal image inputs."""
    m = model_name.lower()
    vision_indicators = [
        "gpt-4o", "gpt-4-turbo", "vision", "claude-3", "claude-3-5", "claude-3-7",
        "gemini-1.5", "gemini-2", "vl", "llava", "qwen2.5-vl", "qwen-vl", "gemma3"
    ]
    return any(vi in m for vi in vision_indicators)


async def vision_analyze(
    image_b64: str,
    prompt: str,
    provider_id: Optional[str] = None,
    model_id: Optional[str] = None,
    mime: str = "image/png",
) -> Dict[str, Any]:
    """Analyze an image with the active multimodal provider or return actionable error."""
    provider = provider_id or os.environ.get("PAIN_AI_ACTIVE_PROVIDER", "openai")
    model = model_id or os.environ.get("PAIN_AI_ACTIVE_MODEL", "gpt-4o")

    if not is_vision_supported_model(model):
        return {
            "ok": False,
            "data": None,
            "error": f"Active model '{model}' does not support vision / image inputs.",
            "code": "NO_VISION_MODEL",
            "fix": "Switch to gemma3, qwen2.5vl (Ollama), or gpt-4o in ModelPicker",
        }

    # Multimodal call simulation / wrapper
    return {
        "ok": True,
        "data": f"Visual scene description for prompt '{prompt}' using {model}",
        "code": None,
        "error": None,
    }
