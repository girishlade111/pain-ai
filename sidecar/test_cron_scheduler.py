"""pain ai — Pytest Suite for Real Cron Execution (Phase 9)

The scheduler executes through injected agent factories (Hermes-free tests):
one-time, recurring, failure honesty, and restart persistence. History is
written ONLY by real execution — never by ticking.
"""

import sys
import time
from pathlib import Path

root_dir = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(root_dir / "sidecar"))

from cron_manager import CronManager
from cron_scheduler import run_job_now, tick_and_run


class StubAgent:
    """Stand-in Hermes turn: records prompts, returns canned output."""

    def __init__(self, output="stubbed turn output", fail=None):
        self.output = output
        self.fail = fail
        self.prompts = []

    def run_conversation(self, user_message="", **kwargs):
        self.prompts.append(user_message)
        if self.fail is not None:
            raise RuntimeError(self.fail)
        return {"content": self.output}


def _manager(tmp_path):
    return CronManager(tmp_path / "cron")


def test_one_time_job_executes_then_disables(tmp_path):
    mgr = _manager(tmp_path)
    job = mgr.create_job(name="Once", schedule_nl="in 1 minute",
                         prompt="Do the thing", delivery="in_app")
    assert job["interval_sec"] is None

    agent = StubAgent(output="did the thing")
    record = run_job_now(job["id"], lambda: agent, cron_dir=tmp_path / "cron")
    assert record is not None
    assert record["status"] == "success"
    assert record["output"] == "did the thing"
    assert agent.prompts == ["Do the thing"]

    jobs = {j["id"]: j for j in mgr.list_jobs()}
    assert jobs[job["id"]]["enabled"] is False
    assert len(jobs[job["id"]]["history"]) == 1
    # Second tick: one-shot never fires again.
    assert mgr.tick(now=time.time() + 3600.0) == []


def test_recurring_job_advances_and_records(tmp_path):
    mgr = _manager(tmp_path)
    job = mgr.create_job(name="Repeat", schedule_nl="every 1 hour",
                         prompt="Check thing", delivery="in_app")
    first_next = job["next_run"]

    agent = StubAgent(output="checked ok")
    records = tick_and_run(lambda: agent, now=first_next + 2.0,
                           cron_dir=tmp_path / "cron")
    assert len(records) == 1
    assert records[0]["status"] == "success"

    jobs = {j["id"]: j for j in mgr.list_jobs()}
    assert jobs[job["id"]]["enabled"] is True
    assert jobs[job["id"]]["next_run"] > first_next
    assert len(jobs[job["id"]]["history"]) == 1


def test_failure_records_error_honestly(tmp_path):
    mgr = _manager(tmp_path)
    job = mgr.create_job(name="Flaky", schedule_nl="in 1 minute",
                         prompt="Boom", delivery="in_app")
    agent = StubAgent(fail="provider exploded")

    record = run_job_now(job["id"], lambda: agent, cron_dir=tmp_path / "cron")
    assert record is not None
    assert record["status"] == "error"
    assert "provider exploded" in record["output"]

    jobs = {j["id"]: j for j in mgr.list_jobs()}
    assert jobs[job["id"]]["history"][0]["status"] == "error"


def test_restart_keeps_jobs_and_history(tmp_path):
    mgr = _manager(tmp_path)
    job = mgr.create_job(name="Persist", schedule_nl="every 1 hour",
                         prompt="Remember me", delivery="in_app")
    run_job_now(job["id"], lambda: StubAgent(output="done"),
                cron_dir=tmp_path / "cron")

    # Simulate app restart: fresh manager on the same directory.
    restarted = CronManager(tmp_path / "cron")
    jobs = {j["id"]: j for j in restarted.list_jobs()}
    assert job["id"] in jobs
    assert jobs[job["id"]]["history"][0]["output"] == "done"


def test_run_unknown_job_returns_none(tmp_path):
    assert run_job_now("job-missing", lambda: StubAgent(),
                       cron_dir=tmp_path / "cron") is None


def test_output_truncated_to_budget(tmp_path):
    mgr = _manager(tmp_path)
    job = mgr.create_job(name="Verbose", schedule_nl="in 1 minute",
                         prompt="Talk a lot", delivery="in_app")
    record = run_job_now(job["id"], lambda: StubAgent(output="x" * 5000),
                         cron_dir=tmp_path / "cron")
    assert len(record["output"]) == 2000
