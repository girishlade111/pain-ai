#!/usr/bin/env python3
"""pain ai — Skills Quarantine Security Scanner (quarantine.py)

Audits skill bundles, scripts, and manifests before installation.
Detects embedded API keys, tokens, private keys, and hardline destructive commands.
Fails closed: any finding unconditionally halts installation.
SSOT (Phase 1): Rust gate.rs owns desktop security policy; this scanner mirrors
its blocklists for sidecar Hub installs. Hermes tools/approval_detection.py is the
upstream agent-loop reference (vendored, untouched).
"""

import os
import re
from pathlib import Path
from typing import Any, Dict, List, Optional

# Secret detection patterns
SECRET_PATTERNS = [
    ("api_key_openai_anthropic", re.compile(r"sk-[A-Za-z0-9_-]{16,}")),
    ("slack_token", re.compile(r"xox[bap]-[A-Za-z0-9-]+")),
    ("github_token", re.compile(r"ghp_[A-Za-z0-9]{36}")),
    ("google_api_key", re.compile(r"AIza[0-9A-Za-z-_]{35}")),
    ("private_key", re.compile(r"-----BEGIN [A-Z ]*PRIVATE KEY-----")),
]

# Hardline destructive command patterns (reused from P04 gate blocklist)
BLOCKLIST_PATTERNS = [
    ("root_deletion", re.compile(r"\brm\s+-[rf]*\s+/(?:\s|$)")),
    ("root_glob_deletion", re.compile(r"\brm\s+-[rf]*\s+/\*")),
    ("windows_root_wipe", re.compile(r"\brmdir\s+/s\s+/q\s+[a-zA-Z]:\\(?:\s|$)", re.IGNORECASE)),
    ("disk_format_mkfs", re.compile(r"\bmkfs(?:\.[a-z0-9]+)?\s+/dev/")),
    ("raw_device_dd", re.compile(r"\bdd\s+.*of=/dev/(?:[sh]d[a-z]|nvme\d+n\d+|mmcblk\d+)")),
    ("windows_disk_format", re.compile(r"\bformat\s+[a-zA-Z]:\s*/[qf]", re.IGNORECASE)),
    ("disk_partition_fdisk", re.compile(r"\bfdisk\s+/dev/")),
    ("disk_partition_parted", re.compile(r"\bparted\s+/dev/")),
    ("fork_bomb", re.compile(r":\(\)\s*\{\s*:\s*\|\s*:\s*&\s*\}\s*;\s*:")),
    ("kernel_panic_trigger", re.compile(r"\becho\s+[cb]\s*>\s*/proc/sysrq-trigger")),
    ("registry_system_wipe", re.compile(r"\breg\s+delete\s+HKLM\\SYSTEM(?:\s|$)", re.IGNORECASE)),
]

SCANNABLE_EXTENSIONS = {
    ".py", ".sh", ".bash", ".js", ".ts", ".ps1", ".bat", ".cmd",
    ".md", ".yaml", ".yml", ".json", ".txt"
}


def scan_text(text: str, filename: str = "inline") -> List[Dict[str, Any]]:
    """Scans raw text content line-by-line for secrets and dangerous patterns."""
    findings: List[Dict[str, Any]] = []
    lines = text.splitlines()

    for idx, line in enumerate(lines, start=1):
        # 1. Check secrets
        for rule_name, pattern in SECRET_PATTERNS:
            m = pattern.search(line)
            if m:
                matched_snippet = m.group(0)
                # Redact matched secret in finding
                redacted = matched_snippet[:4] + "..." + matched_snippet[-4:] if len(matched_snippet) > 8 else "[REDACTED]"
                findings.append({
                    "file": filename,
                    "line": idx,
                    "rule": rule_name,
                    "severity": "CRITICAL",
                    "category": "SecretLeak",
                    "match": redacted,
                    "message": f"Found sensitive credential pattern ({rule_name})",
                })

        # 2. Check blocklist destructive commands
        for rule_name, pattern in BLOCKLIST_PATTERNS:
            m = pattern.search(line)
            if m:
                findings.append({
                    "file": filename,
                    "line": idx,
                    "rule": rule_name,
                    "severity": "CRITICAL",
                    "category": "DestructiveCommand",
                    "match": m.group(0).strip(),
                    "message": f"Found blocked destructive command pattern ({rule_name})",
                })

    return findings


def scan_skill_directory(dir_path: str | Path) -> Dict[str, Any]:
    """Recursively audits all scannable files inside a skill directory."""
    path = Path(dir_path)
    if not path.exists():
        return {
            "clean": False,
            "findings": [{
                "file": str(path),
                "line": 0,
                "rule": "directory_not_found",
                "severity": "CRITICAL",
                "category": "MissingPath",
                "match": str(path),
                "message": "Skill directory does not exist on disk",
            }],
        }

    all_findings: List[Dict[str, Any]] = []

    for root, _, files in os.walk(path):
        for file in files:
            file_path = Path(root) / file
            rel_path = file_path.relative_to(path).as_posix()

            # Only scan recognized source/text extensions
            if file_path.suffix.lower() in SCANNABLE_EXTENSIONS or file.lower() == "skill.md":
                try:
                    content = file_path.read_text(encoding="utf-8", errors="replace")
                    file_findings = scan_text(content, filename=rel_path)
                    all_findings.extend(file_findings)
                except Exception as exc:
                    all_findings.append({
                        "file": rel_path,
                        "line": 0,
                        "rule": "read_error",
                        "severity": "HIGH",
                        "category": "IOError",
                        "match": str(exc),
                        "message": f"Failed to read file for quarantine scan: {exc}",
                    })

    return {
        "clean": len(all_findings) == 0,
        "findings": all_findings,
    }


if __name__ == "__main__":
    import sys
    target = sys.argv[1] if len(sys.argv) > 1 else "."
    res = scan_skill_directory(target)
    print(res)
