#!/usr/bin/env python3
"""pain ai — File Parsers Verification & Fixtures (test_parsers.py)

Generates 5 test fixtures (PDF, DOCX, XLSX, PNG, MP4) and tests parse_file() on each.
"""

import os
import shutil
import subprocess
import sys
import tempfile
import unittest
import zipfile
from pathlib import Path

# Add project root
REPO_ROOT = Path(__file__).resolve().parent.parent.parent
if str(REPO_ROOT) not in sys.path:
    sys.path.insert(0, str(REPO_ROOT))

from sidecar.voice.parsers import parse_file


def _generate_synthetic_pdf(path: Path):
    """Generates a minimal valid PDF document."""
    pdf_content = (
        b"%PDF-1.4\n"
        b"1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj\n"
        b"2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj\n"
        b"3 0 obj<</Type/Page/MediaBox[0 0 612 792]/Parent 2 0 R/Resources<<>>/Contents 4 0 R>>endobj\n"
        b"4 0 obj<</Length 55>>stream\n"
        b"BT /F1 12 Tf 100 700 Td (pain ai PDF table extraction verified.) Tj ET\n"
        b"endstream\nendobj\n"
        b"xref\n0 5\n0000000000 65535 f \n0000000009 00000 n \n0000000052 00000 n \n0000000101 00000 n \n0000000199 00000 n \n"
        b"trailer<</Size 5/Root 1 0 R>>\nstartxref\n303\n%%EOF\n"
    )
    path.write_bytes(pdf_content)


def _generate_synthetic_docx(path: Path):
    """Generates a minimal valid DOCX file (ZIP archive with word/document.xml)."""
    document_xml = (
        '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
        '<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">'
        '<w:body><w:p><w:r><w:t>pain ai Word document text content verified.</w:t></w:r></w:p></w:body>'
        '</w:document>'
    )
    content_types_xml = (
        '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
        '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">'
        '<Default Extension="xml" ContentType="application/xml"/>'
        '<Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>'
        '</Types>'
    )
    with zipfile.ZipFile(path, "w") as zf:
        zf.writestr("[Content_Types].xml", content_types_xml)
        zf.writestr("word/document.xml", document_xml)


def _generate_synthetic_xlsx(path: Path):
    """Generates a genuine XLSX workbook via openpyxl (Phase 10: real files
    only — the previous hand-rolled ZIP lacked [Content_Types].xml and only
    parsed while openpyxl was absent)."""
    from openpyxl import Workbook

    wb = Workbook()
    ws = wb.active
    ws.title = "Metrics"
    ws.append(["Metric", "Value"])
    ws.append(["Accuracy", "99.9%"])
    wb.save(str(path))


def _generate_synthetic_png(path: Path):
    """Generates a minimal valid 1x1 PNG image."""
    png_bytes = bytes([
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A,
        0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
        0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
        0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4,
        0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41,
        0x54, 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00,
        0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00,
        0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE,
        0x42, 0x60, 0x82
    ])
    path.write_bytes(png_bytes)


def _generate_synthetic_mp4(path: Path):
    """Generates a 2-second test MP4 video using ffmpeg."""
    ffmpeg_bin = shutil.which("ffmpeg")
    if not ffmpeg_bin:
        path.write_bytes(b"ftypisom" + b"\x00" * 100)
        return

    cmd = [
        ffmpeg_bin, "-y",
        "-f", "lavfi", "-i", "testsrc=duration=2:size=320x240:rate=10",
        "-f", "lavfi", "-i", "sine=frequency=1000:duration=2",
        "-c:v", "libx264", "-pix_fmt", "yuv420p",
        "-c:a", "aac",
        str(path)
    ]
    subprocess.run(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)


class TestParsers(unittest.TestCase):

    def setUp(self):
        self.tmp_dir = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp_dir.name)

    def tearDown(self):
        self.tmp_dir.cleanup()

    def test_parse_pdf_fixture(self):
        pdf_path = self.root / "sample.pdf"
        _generate_synthetic_pdf(pdf_path)
        res = parse_file(str(pdf_path))
        self.assertTrue(res.get("ok"))
        self.assertEqual(res.get("kind"), "pdf")
        self.assertIn("pain ai PDF", res.get("text", ""))

    def test_parse_docx_fixture(self):
        docx_path = self.root / "sample.docx"
        _generate_synthetic_docx(docx_path)
        res = parse_file(str(docx_path))
        self.assertTrue(res.get("ok"))
        self.assertEqual(res.get("kind"), "docx")
        self.assertIn("Word document text content verified", res.get("text", ""))

    def test_parse_xlsx_fixture(self):
        xlsx_path = self.root / "sample.xlsx"
        _generate_synthetic_xlsx(xlsx_path)
        res = parse_file(str(xlsx_path))
        self.assertTrue(res.get("ok"))
        self.assertEqual(res.get("kind"), "xlsx")
        self.assertIn("Metric", res.get("text", ""))

    def test_parse_png_fixture(self):
        png_path = self.root / "sample.png"
        _generate_synthetic_png(png_path)
        res = parse_file(str(png_path))
        self.assertTrue(res.get("ok"))
        self.assertEqual(res.get("kind"), "image")

    def test_parse_mp4_fixture(self):
        mp4_path = self.root / "sample.mp4"
        _generate_synthetic_mp4(mp4_path)
        res = parse_file(str(mp4_path))
        self.assertTrue(res.get("ok"))
        self.assertEqual(res.get("kind"), "video")
        self.assertIn("Audio Transcript", res.get("text", ""))

    def test_file_size_cap(self):
        # Create a sparse file or simulated large file check
        fake_large = self.root / "large.txt"
        fake_large.write_bytes(b"header\n")
        # Test size limit threshold directly
        import sidecar.voice.parsers as p
        original_cap = p.MAX_FILE_SIZE_BYTES
        try:
            p.MAX_FILE_SIZE_BYTES = 5  # Small cap for test
            res = parse_file(str(fake_large))
            self.assertFalse(res.get("ok"))
            self.assertEqual(res.get("code"), "TOO_LARGE")
        finally:
            p.MAX_FILE_SIZE_BYTES = original_cap


if __name__ == "__main__":
    unittest.main()
