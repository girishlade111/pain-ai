#!/usr/bin/env python3
"""pain ai — Sentence Segmentation Unit Tests (test_segment.py)

10 distinct unit test cases covering edge cases in sentence splitting,
abbreviation protection, decimal protection, and quote preservation.
"""

import sys
from pathlib import Path

# Add project root to sys.path for test discovery
REPO_ROOT = Path(__file__).resolve().parent.parent.parent
if str(REPO_ROOT) not in sys.path:
    sys.path.insert(0, str(REPO_ROOT))

import unittest
from sidecar.voice.segment import split_sentences


class TestSegment(unittest.TestCase):

    def test_01_simple_single_sentence(self):
        text = "Hello world from pain ai."
        res = split_sentences(text)
        self.assertEqual(len(res), 1)
        self.assertEqual(res[0]["i"], 0)
        self.assertEqual(res[0]["text"], "Hello world from pain ai.")

    def test_02_multiple_punctuation(self):
        text = "Can we start now? Yes, absolutely! Let's go."
        res = split_sentences(text)
        self.assertEqual(len(res), 3)
        self.assertEqual(res[0]["text"], "Can we start now?")
        self.assertEqual(res[1]["text"], "Yes, absolutely!")
        self.assertEqual(res[2]["text"], "Let's go.")

    def test_03_titles_and_honorifics(self):
        text = "Dr. Smith and Mr. Adams met at St. Jude Hospital. They began the trial."
        res = split_sentences(text)
        self.assertEqual(len(res), 2)
        self.assertEqual(res[0]["text"], "Dr. Smith and Mr. Adams met at St. Jude Hospital.")
        self.assertEqual(res[1]["text"], "They began the trial.")

    def test_04_latin_abbreviations(self):
        text = "Check the files, e.g. config.json and i.e. manifest.txt. All look good."
        res = split_sentences(text)
        self.assertEqual(len(res), 2)
        self.assertIn("e.g.", res[0]["text"])
        self.assertIn("i.e.", res[0]["text"])
        self.assertEqual(res[1]["text"], "All look good.")

    def test_05_decimal_numbers_and_percentages(self):
        text = "The latency dropped to 3.14ms with 99.9% uptime. System is stable."
        res = split_sentences(text)
        self.assertEqual(len(res), 2)
        self.assertEqual(res[0]["text"], "The latency dropped to 3.14ms with 99.9% uptime.")
        self.assertEqual(res[1]["text"], "System is stable.")

    def test_06_quotes_after_punctuation(self):
        text = '"Hello, pain ai," said the operator. "Is everything ready?"'
        res = split_sentences(text)
        self.assertEqual(len(res), 2)
        self.assertEqual(res[0]["text"], '"Hello, pain ai," said the operator.')
        self.assertEqual(res[1]["text"], '"Is everything ready?"')

    def test_07_ellipses_and_trailing_dots(self):
        text = "Loading models... Done! Let's proceed."
        res = split_sentences(text)
        self.assertEqual(len(res), 3)
        self.assertEqual(res[0]["text"], "Loading models...")
        self.assertEqual(res[1]["text"], "Done!")
        self.assertEqual(res[2]["text"], "Let's proceed.")

    def test_08_newlines_and_extra_whitespace(self):
        text = "First paragraph sentence.\n\nSecond paragraph sentence.   Third one."
        res = split_sentences(text)
        self.assertEqual(len(res), 3)
        self.assertEqual(res[0]["text"], "First paragraph sentence.")
        self.assertEqual(res[1]["text"], "Second paragraph sentence.")
        self.assertEqual(res[2]["text"], "Third one.")

    def test_09_empty_and_whitespace_only(self):
        self.assertEqual(split_sentences(""), [])
        self.assertEqual(split_sentences("   \n\t  "), [])

    def test_10_code_inline_and_symbols(self):
        text = "Execute `cargo check`. The command exits with code 0."
        res = split_sentences(text)
        self.assertEqual(len(res), 2)
        self.assertEqual(res[0]["text"], "Execute `cargo check`.")
        self.assertEqual(res[1]["text"], "The command exits with code 0.")


if __name__ == "__main__":
    unittest.main()
