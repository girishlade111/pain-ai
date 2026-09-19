"""
Generate pixel-perfect geometric chibi companion sprite sheet for pain ai (P11c).
Adheres strictly to the 4-color Trinity palette:
  INK   = #141413 (20, 20, 19)
  CORAL = #d97757 (217, 119, 87)
  CREAM = #faf9f5 (250, 249, 245)
  DARK  = #1f1e1d (31, 30, 29)
"""

from PIL import Image, ImageDraw
import os

INK = (20, 20, 19, 255)
CORAL = (217, 119, 87, 255)
CREAM = (250, 249, 245, 255)
DARK = (31, 30, 29, 255)
CLEAR = (0, 0, 0, 0)

ALLOWED_COLORS = {
    (20, 20, 19),
    (217, 119, 87),
    (250, 249, 245),
    (31, 30, 29),
}

SHEET_W = 512
SHEET_H = 512
CELL_W = 128
CELL_H = 128


def draw_pup_frame(draw, ox, oy, state, frame_idx):
    """
    Draw a geometric companion pup in a 128x128 cell starting at (ox, oy).
    """
    cx = ox + 64
    cy = oy + 64

    # Vertical offsets per animation state/frame
    y_offset = 0
    ear_mode = "floppy"
    eyes_mode = "open"
    mouth_mode = "closed"
    tail_mode = "normal"

    if state == "idle":
        if frame_idx == 0:
            pass
        elif frame_idx == 1:
            y_offset = -2
            ear_mode = "twitch"
        elif frame_idx == 2:
            eyes_mode = "blink"
        elif frame_idx == 3:
            tail_mode = "wag"
            eyes_mode = "warm"

    elif state == "talk":
        if frame_idx == 0:
            mouth_mode = "small"
        elif frame_idx == 1:
            y_offset = -1
            mouth_mode = "wide"
            tail_mode = "wag"
        elif frame_idx == 2:
            mouth_mode = "medium"
        elif frame_idx == 3:
            mouth_mode = "small"

    elif state == "react":
        if frame_idx == 0:
            y_offset = -8
            ear_mode = "perked"
            eyes_mode = "wide"
            mouth_mode = "smile"
            tail_mode = "high"
        elif frame_idx == 1:
            y_offset = -3
            ear_mode = "perked"
            eyes_mode = "warm"
            mouth_mode = "smile"
            tail_mode = "wag"

    base_y = cy + y_offset

    # 1. Tail (behind body)
    if tail_mode == "normal":
        draw.polygon([(cx + 24, base_y + 24), (cx + 42, base_y + 14), (cx + 38, base_y + 8), (cx + 20, base_y + 18)], fill=CORAL)
    elif tail_mode == "wag":
        draw.polygon([(cx + 24, base_y + 24), (cx + 44, base_y + 6), (cx + 38, base_y + 2), (cx + 18, base_y + 18)], fill=CORAL)
    elif tail_mode == "high":
        draw.polygon([(cx + 22, base_y + 22), (cx + 38, base_y - 4), (cx + 32, base_y - 6), (cx + 18, base_y + 16)], fill=CORAL)

    # 2. Torso / Body
    # Sitting rounded body
    draw.rounded_rectangle(
        [cx - 24, base_y + 8, cx + 24, base_y + 44],
        radius=14,
        fill=CORAL
    )
    # Cream chest bib
    draw.rounded_rectangle(
        [cx - 14, base_y + 14, cx + 14, base_y + 38],
        radius=8,
        fill=CREAM
    )

    # 3. Paws
    # Left and right front paws resting on ground
    draw.rounded_rectangle([cx - 22, base_y + 38, cx - 6, base_y + 46], radius=4, fill=CREAM)
    draw.rounded_rectangle([cx + 6, base_y + 38, cx + 22, base_y + 46], radius=4, fill=CREAM)
    # Paw pads lines
    draw.line([cx - 14, base_y + 41, cx - 14, base_y + 45], fill=INK, width=1)
    draw.line([cx + 14, base_y + 41, cx + 14, base_y + 45], fill=INK, width=1)

    # 4. Ears (drawn behind head or framing head)
    if ear_mode == "floppy":
        # Left floppy ear
        draw.rounded_rectangle([cx - 36, base_y - 20, cx - 22, base_y + 12], radius=6, fill=INK)
        # Right floppy ear
        draw.rounded_rectangle([cx + 22, base_y - 20, cx + 36, base_y + 12], radius=6, fill=INK)
    elif ear_mode == "twitch":
        # Left ear flared out
        draw.rounded_rectangle([cx - 39, base_y - 22, cx - 25, base_y + 10], radius=6, fill=INK)
        # Right ear normal
        draw.rounded_rectangle([cx + 22, base_y - 20, cx + 36, base_y + 12], radius=6, fill=INK)
    elif ear_mode == "perked":
        # Both ears perked upward!
        draw.polygon([(cx - 28, base_y - 12), (cx - 32, base_y - 36), (cx - 16, base_y - 18)], fill=INK)
        draw.polygon([(cx + 16, base_y - 18), (cx + 32, base_y - 36), (cx + 28, base_y - 12)], fill=INK)

    # 5. Head
    draw.rounded_rectangle(
        [cx - 26, base_y - 26, cx + 26, base_y + 14],
        radius=16,
        fill=CORAL
    )

    # Cream Snout / Muzzle
    draw.rounded_rectangle(
        [cx - 15, base_y - 6, cx + 15, base_y + 12],
        radius=9,
        fill=CREAM
    )

    # Ink Nose
    draw.polygon(
        [(cx - 5, base_y - 3), (cx + 5, base_y - 3), (cx, base_y + 3)],
        fill=INK
    )

    # 6. Eyes
    left_eye_x = cx - 12
    right_eye_x = cx + 12
    eye_y = base_y - 10

    if eyes_mode == "open":
        # Left eye
        draw.ellipse([left_eye_x - 4, eye_y - 4, left_eye_x + 4, eye_y + 4], fill=INK)
        draw.point([left_eye_x - 1, eye_y - 1], fill=CREAM)
        # Right eye
        draw.ellipse([right_eye_x - 4, eye_y - 4, right_eye_x + 4, eye_y + 4], fill=INK)
        draw.point([right_eye_x - 1, eye_y - 1], fill=CREAM)
    elif eyes_mode == "blink":
        draw.line([left_eye_x - 4, eye_y, left_eye_x + 4, eye_y], fill=INK, width=2)
        draw.line([right_eye_x - 4, eye_y, right_eye_x + 4, eye_y], fill=INK, width=2)
    elif eyes_mode == "warm":
        # Joyful curved eyes (^ ^)
        draw.arc([left_eye_x - 5, eye_y - 4, left_eye_x + 5, eye_y + 2], start=200, end=340, fill=INK, width=2)
        draw.arc([right_eye_x - 5, eye_y - 4, right_eye_x + 5, eye_y + 2], start=200, end=340, fill=INK, width=2)
    elif eyes_mode == "wide":
        # Alert sparkles
        draw.ellipse([left_eye_x - 6, eye_y - 6, left_eye_x + 6, eye_y + 6], fill=INK)
        draw.rectangle([left_eye_x - 2, eye_y - 3, left_eye_x + 1, eye_y], fill=CREAM)
        draw.ellipse([right_eye_x - 6, eye_y - 6, right_eye_x + 6, eye_y + 6], fill=INK)
        draw.rectangle([right_eye_x - 2, eye_y - 3, right_eye_x + 1, eye_y], fill=CREAM)

    # 7. Mouth
    mouth_y = base_y + 6
    if mouth_mode == "closed":
        draw.line([cx, base_y + 3, cx, mouth_y], fill=INK, width=1)
        draw.arc([cx - 6, mouth_y - 2, cx, mouth_y + 4], start=0, end=180, fill=INK, width=1)
        draw.arc([cx, mouth_y - 2, cx + 6, mouth_y + 4], start=0, end=180, fill=INK, width=1)
    elif mouth_mode == "small":
        draw.ellipse([cx - 3, mouth_y - 1, cx + 3, mouth_y + 4], fill=DARK)
        draw.ellipse([cx - 2, mouth_y + 1, cx + 2, mouth_y + 4], fill=CORAL)
    elif mouth_mode == "wide":
        draw.ellipse([cx - 5, mouth_y - 2, cx + 5, mouth_y + 6], fill=DARK)
        draw.ellipse([cx - 4, mouth_y + 1, cx + 4, mouth_y + 6], fill=CORAL)
    elif mouth_mode == "medium":
        draw.ellipse([cx - 4, mouth_y - 1, cx + 4, mouth_y + 5], fill=DARK)
        draw.ellipse([cx - 3, mouth_y + 1, cx + 3, mouth_y + 5], fill=CORAL)
    elif mouth_mode == "smile":
        draw.line([cx, base_y + 3, cx, mouth_y], fill=INK, width=1)
        draw.arc([cx - 7, mouth_y - 3, cx + 7, mouth_y + 5], start=0, end=180, fill=INK, width=2)
        draw.chord([cx - 4, mouth_y, cx + 4, mouth_y + 5], start=0, end=180, fill=CORAL)


def generate_sheet():
    img = Image.new("RGBA", (SHEET_W, SHEET_H), CLEAR)
    draw = ImageDraw.Draw(img)

    # Row 0: Idle (4 frames)
    for i in range(4):
        draw_pup_frame(draw, i * CELL_W, 0 * CELL_H, "idle", i)

    # Row 1: Talk (4 frames)
    for i in range(4):
        draw_pup_frame(draw, i * CELL_W, 1 * CELL_H, "talk", i)

    # Row 2: React (2 frames)
    for i in range(2):
        draw_pup_frame(draw, i * CELL_W, 2 * CELL_H, "react", i)

    out_path = os.path.join("src", "assets", "chibi.png")
    os.makedirs(os.path.dirname(out_path), exist_ok=True)
    img.save(out_path, format="PNG")
    print(f"Saved chibi sprite sheet to {out_path}")

    # Verify palette conformance
    audit_palette(img)


def audit_palette(img):
    pixels = img.load()
    w, h = img.size
    violations = 0
    color_counts = {}

    for y in range(h):
        for x in range(w):
            r, g, b, a = pixels[x, y]
            if a == 0:
                continue
            # For anti-aliasing or direct drawing, clamp or check
            rgb = (r, g, b)
            color_counts[rgb] = color_counts.get(rgb, 0) + 1
            if rgb not in ALLOWED_COLORS:
                violations += 1

    print(f"Audited {w}x{h} sheet: {violations} palette violations found.")
    for c, cnt in color_counts.items():
        print(f"  Color {c}: {cnt} px")

    if violations > 0:
        print("Note: Palette has intermediate colors (possibly from PIL line/arc antialiasing). Remapping to nearest Trinity color...")
        remap_to_trinity(img)


def remap_to_trinity(img):
    pixels = img.load()
    w, h = img.size
    palette_list = list(ALLOWED_COLORS)

    def dist(c1, c2):
        return (c1[0]-c2[0])**2 + (c1[1]-c2[1])**2 + (c1[2]-c2[2])**2

    for y in range(h):
        for x in range(w):
            r, g, b, a = pixels[x, y]
            if a < 128:
                pixels[x, y] = (0, 0, 0, 0)
            else:
                nearest = min(palette_list, key=lambda p: dist((r, g, b), p))
                pixels[x, y] = (nearest[0], nearest[1], nearest[2], 255)

    out_path = os.path.join("src", "assets", "chibi.png")
    img.save(out_path, format="PNG")
    print(f"Remapped and saved strictly conforming 4-color palette to {out_path}")

    # Re-verify
    violations = 0
    for y in range(h):
        for x in range(w):
            r, g, b, a = pixels[x, y]
            if a > 0 and (r, g, b) not in ALLOWED_COLORS:
                violations += 1
    assert violations == 0, f"Expected 0 violations after remap, got {violations}"
    print("Verification PASSED: 100% of non-transparent pixels match Trinity palette!")


if __name__ == "__main__":
    generate_sheet()
