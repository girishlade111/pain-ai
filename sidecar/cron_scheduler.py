#!/usr/bin/env python3
"""pain ai — Cron Scheduler Loop + Real Hermes Execution (cron_scheduler.py)

The sidecar tick loop (the lightweight alternative to adopting Hermes's
gateway-coupled scheduler — same jobs.json, no second scheduler state).
Hermes-free: the agent is injected as a factory, so every path is
unit-testable and this module never imports hermes-agent.

Every due job is executed by a REAL Hermes turn; the record reflects the
actual outcome (success with truncated output, or error with the exception
message). Nothing is fabricated: no history is written for work that did
not run.

Sessions: cron turns run under ``cron-<job_id>`` and are recorded into the
chat session store (source "cron", demoted in recall search by design).
"""

import logging
import os
import threading
import time
from pathlib import Path
from typing import Any, Callable, Dict, List, Optional

logger = logging.getLogger("cron_scheduler")

DEFAULT_INTERVAL_SEC = 30.0
MAX_OUTPUT_CHARS = 2000

_scheduler_thread: Optional[threading.Thread] = None
_scheduler_stop: Optional[threading.Event] = None
_scheduler_lock = threading.Lock()


def _managers():
    try:
        from cron_manager import get_cron_manager
        from session_search import get_state_db
        from turn_history import record_turn
    except ImportError:
        from sidecar.cron_manager import get_cron_manager
        from sidecar.session_search import get_state_db
        from sidecar.turn_history import record_turn
    return get_cron_manager(), get_state_db(), record_turn


def run_job_now(job_id: str,
                agent_factory: Callable[[], Any],
                cron_dir: Optional[Path] = None) -> Optional[Dict[str, Any]]:
    """Execute one job through a real Hermes turn and record the outcome.

    agent_factory() -> object with .run_conversation(user_message=..., ...)
    returning str | {"content"|"response": str}. Returns the history record,
    or None when the job does not exist. Exceptions from the agent become
    status="error" records carrying the message — never silent, never fake.
    """
    if cron_dir is not None:
        try:
            from cron_manager import CronManager
        except ImportError:
            from sidecar.cron_manager import CronManager
        manager = CronManager(cron_dir)
    else:
        manager, _, _ = _managers()
    jobs = {j["id"]: j for j in manager.list_jobs()}
    job = jobs.get(job_id)
    if job is None:
        return None

    session_id = f"cron-{job_id}"
    output = ""
    status = "success"
    try:
        agent = agent_factory()
        result = agent.run_conversation(user_message=job["prompt"])
        if isinstance(result, dict):
            output = result.get("content", "") or result.get("response", "")
        elif isinstance(result, str):
            output = result
        output = (output or "").strip()
        if not output:
            output = "(agent returned no text)"
    except Exception as exc:
        logger.error(f"cron job '{job_id}' execution failed: {exc}", exc_info=True)
        status = "error"
        output = f"Execution failed: {exc}"

    record = manager.record_run(job_id, status=status, output=output[:MAX_OUTPUT_CHARS])

    # Mirror the turn into chat history (best effort; never breaks the record).
    try:
        _, db, record_turn = _managers()
        record_turn(db, session_id, job["prompt"],
                    output if status == "success" else None, source="cron")
    except Exception as exc:
        logger.warning(f"cron history mirror failed: {exc}")
    return record


def tick_and_run(agent_factory: Callable[[], Any],
                 now: Optional[float] = None,
                 cron_dir: Optional[Path] = None) -> List[Dict[str, Any]]:
    """Fire all due jobs through real execution. Returns history records."""
    if cron_dir is not None:
        try:
            from cron_manager import CronManager
        except ImportError:
            from sidecar.cron_manager import CronManager
        due = CronManager(cron_dir).tick(now=now)
    else:
        manager, _, _ = _managers()
        due = manager.tick(now=now)
    records = []
    for item in due:
        record = run_job_now(item["job_id"], agent_factory, cron_dir=cron_dir)
        if record is not None:
            records.append(record)
    return records


def _loop(agent_factory: Callable[[], Any], interval: float,
          stop: threading.Event) -> None:
    logger.info(f"cron scheduler loop started (interval={interval}s)")
    while not stop.wait(interval):
        try:
            tick_and_run(agent_factory)
        except Exception as exc:
            logger.error(f"cron scheduler tick failed: {exc}", exc_info=True)


def ensure_scheduler(agent_factory: Callable[[], Any],
                     interval: Optional[float] = None) -> bool:
    """Start the daemon tick loop once. Honors LSC_CRON_DISABLE=1 and
    LSC_CRON_INTERVAL_SEC. Returns True when the loop is (now) running."""
    global _scheduler_thread, _scheduler_stop
    if os.environ.get("LSC_CRON_DISABLE", "").strip().lower() in ("1", "true", "yes"):
        logger.info("cron scheduler disabled via LSC_CRON_DISABLE")
        return False
    with _scheduler_lock:
        if _scheduler_thread is not None and _scheduler_thread.is_alive():
            return True
        try:
            seconds = float(interval if interval is not None
                            else os.environ.get("LSC_CRON_INTERVAL_SEC", DEFAULT_INTERVAL_SEC))
        except ValueError:
            seconds = DEFAULT_INTERVAL_SEC
        seconds = max(5.0, seconds)
        _scheduler_stop = threading.Event()
        _scheduler_thread = threading.Thread(
            target=_loop, args=(agent_factory, seconds, _scheduler_stop),
            name="lsc-cron-scheduler", daemon=True)
        _scheduler_thread.start()
        return True


def stop_scheduler() -> None:
    """Stop the daemon loop (tests / shutdown)."""
    global _scheduler_thread, _scheduler_stop
    with _scheduler_lock:
        if _scheduler_stop is not None:
            _scheduler_stop.set()
        thread, _scheduler_thread = _scheduler_thread, None
        _scheduler_stop = None
    if thread is not None:
        thread.join(timeout=5.0)
