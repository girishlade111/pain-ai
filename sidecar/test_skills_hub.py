#!/usr/bin/env python3
"""pain ai — Skills Hub, Trust & MCP Verification Test Suite (test_skills_hub.py)

Validates:
1. Hub installation of clean skill with lockfile version pinning.
2. Hub installation of dirty skill unconditionally blocked by Quarantine Scanner.
3. Untrusted project skill hidden from discovery until operator grants Trust.
4. /learn draft proposal, listing, and approval.
5. MCP tool include/exclude filtering logic (include beats exclude).
"""

import json
import tempfile
import unittest
from pathlib import Path

from sidecar.skills_manager import (
    skills_list, skill_view, hub_install, hub_audit, hub_remove,
    set_skill_trust, is_skill_trusted, get_lockfile,
    learn_draft_propose, learn_drafts_list, learn_draft_approve, learn_draft_reject
)
from sidecar.mcp_manager import filter_tools, prefix_tool_name


class TestSkillsHubAndTrust(unittest.TestCase):

    def setUp(self):
        self.base_dir = Path(__file__).resolve().parent.parent
        self.fixtures_dir = self.base_dir / "tests" / "fixtures" / "hub-tap"

    def test_1_hub_install_clean_skill_pins_lockfile(self):
        clean_src = self.fixtures_dir / "clean-tool"
        res = hub_install(str(clean_src), tap="test-tap")
        self.assertTrue(res["ok"])
        self.assertEqual(res["status"], "installed")
        self.assertEqual(res["skill"], "clean-tool")

        # Verify entry in lockfile
        lock = get_lockfile()
        self.assertIn("clean-tool", lock)
        self.assertEqual(lock["clean-tool"]["tap"], "test-tap")
        self.assertTrue(len(lock["clean-tool"]["sha256"]) > 0)

        # Cleanup
        hub_remove("clean-tool")

    def test_2_hub_install_dirty_skill_blocked_by_quarantine(self):
        dirty_src = self.fixtures_dir / "dirty-tool"
        res = hub_install(str(dirty_src), tap="untrusted-tap")
        self.assertFalse(res["ok"])
        self.assertEqual(res["status"], "quarantine_failed")
        self.assertIn("findings", res)
        self.assertGreater(len(res["findings"]), 0)

        # Verify NOT in lockfile
        lock = get_lockfile()
        self.assertNotIn("dirty-tool", lock)

    def test_3_project_skill_trust_gating(self):
        with tempfile.TemporaryDirectory() as ws_tmp:
            ws_path = Path(ws_tmp)
            proj_skills = ws_path / ".pain-ai" / "skills" / "my-proj-skill"
            proj_skills.mkdir(parents=True)
            (proj_skills / "SKILL.md").write_text(
                "---\nname: my-proj-skill\ndescription: A project specific automation.\n---\n# My Proj Skill\nContent",
                encoding="utf-8"
            )

            # Untrusted project skill should be absent from discovery
            self.assertFalse(is_skill_trusted(str(ws_path), "my-proj-skill"))
            discovered = [s["name"] for s in skills_list(workspace=str(ws_path))]
            self.assertNotIn("my-proj-skill", discovered)

            # Grant trust
            set_skill_trust(str(ws_path), "my-proj-skill", True)
            self.assertTrue(is_skill_trusted(str(ws_path), "my-proj-skill"))

            # Now discovered
            discovered_after = [s["name"] for s in skills_list(workspace=str(ws_path))]
            self.assertIn("my-proj-skill", discovered_after)

    def test_4_learn_draft_lifecycle(self):
        draft = learn_draft_propose(
            name="auto-formatter",
            description="Format json and yaml safely.",
            content="---\nname: auto-formatter\ndescription: Format json safely.\n---\n# Auto Formatter\nEcho",
        )
        self.assertTrue(draft["guard_agent_created"])
        self.assertTrue(draft["write_approval"])
        draft_id = draft["id"]

        drafts = learn_drafts_list()
        self.assertTrue(any(d["id"] == draft_id for d in drafts))

        # Approve draft
        appr_res = learn_draft_approve(draft_id)
        self.assertTrue(appr_res["ok"])
        self.assertTrue(appr_res["guard_agent_created"])

        # Cleanup installed skill
        hub_remove("auto-formatter")

    def test_5_mcp_include_exclude_precedence(self):
        sample_tools = [
            {"name": "read_file"},
            {"name": "write_file"},
            {"name": "delete_file"},
            {"name": "ping"},
        ]

        # Case A: Only include specified
        inc_only = filter_tools(sample_tools, include=["read_*", "ping"])
        self.assertEqual([t["name"] for t in inc_only], ["read_file", "ping"])

        # Case B: Only exclude specified
        exc_only = filter_tools(sample_tools, exclude=["delete_*"])
        self.assertEqual([t["name"] for t in exc_only], ["read_file", "write_file", "ping"])

        # Case C: Both specified -> include strictly beats exclude!
        both = filter_tools(sample_tools, include=["*_file"], exclude=["delete_file"])
        # include wins, so all *_file tools are returned
        self.assertEqual([t["name"] for t in both], ["read_file", "write_file", "delete_file"])

    def test_6_mcp_tool_collision_prefixing(self):
        names = set()
        p1 = prefix_tool_name("server1", "query", names)
        self.assertEqual(p1, "mcp_server1_query")

        p2 = prefix_tool_name("server1", "query", names)
        self.assertEqual(p2, "mcp_server1_query_2")

        p3 = prefix_tool_name("server1", "query", names)
        self.assertEqual(p3, "mcp_server1_query_3")


if __name__ == "__main__":
    unittest.main()
