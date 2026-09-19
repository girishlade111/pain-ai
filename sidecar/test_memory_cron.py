"""pain ai — Pytest Suite for Memory, Session Search, Cron, and Delegation (P11b)
Tests:
- FTS5 BM25 ranked session search on seeded sessions
- 200-session search latency benchmark (<500ms)
- Memory character limits (2200 for MEMORY.md, 1375 for USER.md) & persistence across restart
- Cron natural language scheduling & strict in-app delivery assertion (platform delivery rejected)
- Cron scheduled vs fired delta (<=30s) and disabled job zero-fire guarantee
- Context compressor token reduction
- Subagent delegation parallel cap (1-3)
"""

import sys
import time
import pytest
from pathlib import Path

# Add project root and fixtures to sys.path
root_dir = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(root_dir / "sidecar"))
sys.path.insert(0, str(root_dir / "tests" / "fixtures"))

from memory_manager import MemoryManager, MEMORY_CHAR_LIMIT, USER_CHAR_LIMIT
from session_search import StateDB
from cron_manager import CronManager, parse_schedule_nl
from compressor import ContextCompressor, estimate_messages_tokens
from delegation import DelegationManager
from importlib import import_module

seed_module = import_module("seed-sessions")
seed_test_database = seed_module.seed_test_database


def test_session_search_ranked_results(tmp_path):
    """Step 1: 2 seeded sessions -> query returns ranked with scores."""
    db_file = tmp_path / "test_state.db"
    db = StateDB(db_file)

    now = time.time()
    # Session 1: Gate pattern security discussions
    db.insert_session("sess-1", "Permission Gate Security Analysis", "user", now - 100)
    db.insert_message("sess-1", "user", "How does the Tauri permission gate handle blocklist patterns?", timestamp=now - 99)
    db.insert_message("sess-1", "assistant", "The gate intercepts shell commands and flags rm -rf root deletions.", timestamp=now - 98)

    # Session 2: Web frontend styling
    db.insert_session("sess-2", "Tailwind Component Design", "user", now - 50)
    db.insert_message("sess-2", "user", "Can we update the button padding in index.css?", timestamp=now - 49)

    # Search for "permission gate"
    results = db.search_sessions("permission gate")
    assert len(results) >= 1
    top_hit = results[0]
    assert top_hit["session_id"] == "sess-1"
    assert "gate" in top_hit["snippet"].lower()
    assert top_hit["rank_score"] < 0  # SQLite BM25 scores are negative (lower = more relevant)


def test_200_session_search_latency_under_500ms(tmp_path):
    """Acceptance: Search latency <500ms on 200-session fixture."""
    db_file = tmp_path / "bench_state.db"
    db = StateDB(db_file)

    # Seed 200 sessions
    seed_test_database(db, 200)

    # Benchmark search across all 200 sessions (600 messages)
    t0 = time.perf_counter()
    results = db.search_sessions("accessibility tree automation", limit=20)
    elapsed_ms = (time.perf_counter() - t0) * 1000.0

    print(f"\n[BENCHMARK] 200-session search took {elapsed_ms:.2f}ms (target < 500ms)")
    assert elapsed_ms < 500.0, f"Search latency {elapsed_ms}ms exceeded 500ms cap"
    assert len(results) > 0


def test_memory_character_limits_and_restart_persistence(tmp_path):
    """Acceptance: Memory caps enforced and edits persist across restart."""
    mem_dir = tmp_path / "memories"
    mgr = MemoryManager(mem_dir)

    # 1. Test MEMORY.md character limit (2200 cap)
    valid_content = "Durable facts:\n" + ("- fact line\n" * 50)
    assert len(valid_content) <= MEMORY_CHAR_LIMIT
    res = mgr.update_memory("memory", valid_content)
    assert res["ok"] is True

    oversized_content = "X" * (MEMORY_CHAR_LIMIT + 50)
    res_bad = mgr.update_memory("memory", oversized_content)
    assert res_bad["ok"] is False
    assert "exceeds hard limit" in res_bad["error"]

    # 2. Test USER.md character limit (1375 cap)
    valid_user = "Operator preferences:\n- Concise output\n- Local model preferred"
    res_user = mgr.update_memory("user", valid_user)
    assert res_user["ok"] is True

    oversized_user = "U" * (USER_CHAR_LIMIT + 20)
    res_user_bad = mgr.update_memory("user", oversized_user)
    assert res_user_bad["ok"] is False
    assert "exceeds hard limit" in res_user_bad["error"]

    # 3. Simulate sidecar restart (new MemoryManager reading the same disk dir)
    mgr_restarted = MemoryManager(mem_dir)
    loaded_mem = mgr_restarted.get_memory("memory")
    assert loaded_mem["content"] == valid_content
    loaded_user = mgr_restarted.get_memory("user")
    assert loaded_user["content"] == valid_user


def test_cron_in_app_delivery_and_platform_refusal(tmp_path):
    """Acceptance: Platform delivery attempt -> clean refused error."""
    cron_mgr = CronManager(tmp_path / "cron")

    # In-app delivery succeeds
    job = cron_mgr.create_job(
        name="Morning Brief",
        schedule_nl="every weekday 8am",
        prompt="Summarize overnight git commits and active tasks",
        delivery="in_app",
    )
    assert job["id"].startswith("job-")
    assert job["delivery"] == "in_app"

    # Platform delivery (e.g. Telegram / Discord) strictly refused
    with pytest.raises(ValueError) as exc:
        cron_mgr.create_job(
            name="External Alert",
            schedule_nl="in 10 minutes",
            prompt="Send telegram notification",
            delivery="telegram",
        )
    assert "strictly disabled in pain ai desktop v1" in str(exc.value)


def test_cron_fire_tolerance_and_disable_guarantee(tmp_path):
    """Acceptance: Scheduled vs fired delta <= 30s; disabled job fires zero times."""
    cron_mgr = CronManager(tmp_path / "cron")
    now = time.time()

    # Create job scheduled in 2 minutes
    job = cron_mgr.create_job(
        name="Ping Test",
        schedule_nl="in 2 minutes",
        prompt="Say PING-TEST",
        delivery="in_app",
    )
    scheduled_next_run = job["next_run"]
    assert abs((scheduled_next_run - now) - 120.0) < 1.0

    # 1. Tick before scheduled time -> 0 fires
    fired = cron_mgr.tick(now=scheduled_next_run - 10.0)
    assert len(fired) == 0

    # 2. Tick at scheduled time + 5s -> fires with delta <= 30s
    fired = cron_mgr.tick(now=scheduled_next_run + 5.0)
    assert len(fired) == 1
    assert fired[0]["job_name"] == "Ping Test"
    assert fired[0]["delta_sec"] <= 30.0  # 5.0s delta <= 30s tolerance

    # 3. Create recurring job, then disable it -> zero fires
    recurring = cron_mgr.create_job(
        name="Recurring Task",
        schedule_nl="every 5 minutes",
        prompt="Check disk storage",
        delivery="in_app",
    )
    cron_mgr.toggle_job(recurring["id"], False)

    # Tick past its scheduled run
    future = recurring["next_run"] + 60.0
    fired_disabled = cron_mgr.tick(now=future)
    assert len(fired_disabled) == 0, "Disabled job must never fire"


def test_subagent_delegation_max_parallel_clamping():
    """Step 5: Delegation toggle and max_parallel strictly capped to 1-3."""
    delegation = DelegationManager(enabled=True, max_parallel=3)
    assert delegation.max_parallel == 3

    # Attempt to exceed v1 cap of 3
    delegation.update_config(enabled=True, max_parallel=10)
    assert delegation.max_parallel == 3

    # Clamp below 1
    delegation.update_config(enabled=True, max_parallel=0)
    assert delegation.max_parallel == 1

    # Run 3 parallel mock tasks
    def mock_task(query):
        return f"Found {query}"

    tasks = [lambda q=q: mock_task(q) for q in ["query-A", "query-B", "query-C"]]
    delegation.update_config(enabled=True, max_parallel=3)
    results = delegation.run_parallel_tasks(tasks)
    assert len(results) == 3
    assert set(results) == {"Found query-A", "Found query-B", "Found query-C"}


def test_context_compressor_reduces_tokens():
    """Step 6: Compressor (/compress) summarizes middle turns and reduces tokens."""
    compressor = ContextCompressor(threshold_percent=0.80)

    # Build a long transcript with 10 turns
    messages = [
        {"role": "user", "content": "Initial project goal: build a local-first desktop agent."},
    ]
    for i in range(1, 9):
        messages.append({
            "role": "assistant" if i % 2 == 0 else "user",
            "content": f"Turn {i}: Discussing technical details of module {i} with detailed explanation and logs." * 10,
        })
    messages.append({"role": "user", "content": "What was the initial project goal?"})

    orig_tokens = estimate_messages_tokens(messages)

    # Trigger compaction
    result = compressor.compress_messages(messages, context_limit=1000, force=True)
    assert result["compressed"] is True
    assert result["compressed_tokens"] < orig_tokens
    assert result["tokens_saved"] > 0

    # Ensure head and tail are preserved
    new_messages = result["messages"]
    assert new_messages[0]["content"] == messages[0]["content"]  # Head intact
    assert new_messages[-1]["content"] == messages[-1]["content"]  # Tail intact
    assert any("[CONTEXT COMPACTION SUMMARY]" in m.get("content", "") for m in new_messages)
