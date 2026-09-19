#!/usr/bin/env python3
"""pain ai — Quarantine Security Scanner Unit Tests (test_quarantine.py)

Validates the 6 mandatory quarantine detection scenarios:
1. Clean skill passes audit.
2. OpenAI / Anthropic key triggers failure.
3. Slack token triggers failure.
4. GitHub token triggers failure.
5. Private key triggers failure.
6. Blocklisted destructive command triggers failure.
"""

import tempfile
import unittest
from pathlib import Path
from sidecar.quarantine import scan_skill_directory, scan_text


class TestQuarantineScanner(unittest.TestCase):

    def test_1_clean_skill_passes(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            skill_dir = Path(tmpdir)
            (skill_dir / "SKILL.md").write_text(
                "---\nname: clean-tool\ndescription: A safe utility.\n---\n# Clean Tool\nRuns safely.",
                encoding="utf-8"
            )
            scripts_dir = skill_dir / "scripts"
            scripts_dir.mkdir()
            (scripts_dir / "run.py").write_text(
                "print('Hello safe world')",
                encoding="utf-8"
            )

            result = scan_skill_directory(skill_dir)
            self.assertTrue(result["clean"])
            self.assertEqual(len(result["findings"]), 0)

    def test_2_openai_key_triggers_quarantine(self):
        content = 'export OPENAI_API_KEY="' + 'sk-' + 'proj-abc1234567890def1234567890"'
        findings = scan_text(content, "setup.sh")
        self.assertGreater(len(findings), 0)
        self.assertEqual(findings[0]["rule"], "api_key_openai_anthropic")
        self.assertEqual(findings[0]["category"], "SecretLeak")
        self.assertIn("sk-p...7890", findings[0]["match"])

    def test_3_slack_token_triggers_quarantine(self):
        content = 'SLACK_BOT_TOKEN = "' + 'xoxb-' + '123456789012-1234567890123-abcdefghijklmnop"'
        findings = scan_text(content, "bot.py")
        self.assertGreater(len(findings), 0)
        self.assertEqual(findings[0]["rule"], "slack_token")
        self.assertEqual(findings[0]["category"], "SecretLeak")

    def test_4_github_token_triggers_quarantine(self):
        content = 'git_token = "' + 'ghp_' + '1234567890abcdefghijklmnopqrstuvwxyz"'
        findings = scan_text(content, "sync.js")
        self.assertGreater(len(findings), 0)
        self.assertEqual(findings[0]["rule"], "github_token")
        self.assertEqual(findings[0]["category"], "SecretLeak")

    def test_5_private_key_triggers_quarantine(self):
        content = "-----BEGIN RSA PRIVATE KEY-----\nMIIEowIBAAKCAQEA0...\n-----END RSA PRIVATE KEY-----"
        findings = scan_text(content, "id_rsa")
        self.assertGreater(len(findings), 0)
        self.assertEqual(findings[0]["rule"], "private_key")
        self.assertEqual(findings[0]["category"], "SecretLeak")

    def test_6_destructive_blocklist_triggers_quarantine(self):
        content = "#!/bin/bash\nrm -rf / --no-preserve-root\necho done"
        findings = scan_text(content, "destroy.sh")
        self.assertGreater(len(findings), 0)
        self.assertEqual(findings[0]["rule"], "root_deletion")
        self.assertEqual(findings[0]["category"], "DestructiveCommand")

        # Test Windows format blocklist
        win_content = "format C: /q /y"
        win_findings = scan_text(win_content, "wipe.bat")
        self.assertGreater(len(win_findings), 0)
        self.assertEqual(win_findings[0]["rule"], "windows_disk_format")


if __name__ == "__main__":
    unittest.main()
