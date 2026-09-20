"""pain ai — Natural Language Cron Subsystem & In-App Scheduler
Source of truth: PRD.md §4.3 + hermes-map.md §9 + cron/
SSOT (Phase 1): parse_schedule_nl here owns NL parsing for HTTP transport;
Rust memory_cron.parse_schedule_nl_mirror mirrors the `in/every X` relative
forms for Tauri invoke. Hermes cron/* owns upstream scheduler semantics.
"""

import json
import os
import re
import time
import uuid
from datetime import datetime, timedelta
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

ALLOWED_DELIVERIES = ("in_app", "app", "ui")


def get_pain_ai_home() -> Path:
    home_env = os.environ.get("PAIN_AI_HOME")
    if home_env:
        return Path(home_env)
    return Path.home() / ".pain-ai"


def get_cron_dir() -> Path:
    cron_dir = get_pain_ai_home() / "cron"
    cron_dir.mkdir(parents=True, exist_ok=True)
    return cron_dir


def parse_schedule_nl(schedule_nl: str, reference_time: Optional[float] = None) -> Tuple[float, Optional[float]]:
    """
    Parses natural language schedule string into (next_run_timestamp, interval_seconds_or_none).
    Uses local system timezone for all calendar time computations.
    """
    now = reference_time or time.time()
    dt_now = datetime.fromtimestamp(now)
    s = schedule_nl.strip().lower()

    # 1. "in X seconds / minutes / hours"
    m_in = re.match(r"^in\s+(\d+)\s*(s|sec|seconds?|m|min|minutes?|h|hr|hours?)$", s)
    if m_in:
        val = int(m_in.group(1))
        unit = m_in.group(2)
        if unit.startswith("s"):
            sec = val
        elif unit.startswith("m"):
            sec = val * 60
        else:
            sec = val * 3600
        return now + sec, None

    # 2. "every X seconds / minutes / hours"
    m_every = re.match(r"^every\s+(\d+)\s*(s|sec|seconds?|m|min|minutes?|h|hr|hours?)$", s)
    if m_every:
        val = int(m_every.group(1))
        unit = m_every.group(2)
        if unit.startswith("s"):
            sec = val
        elif unit.startswith("m"):
            sec = val * 60
        else:
            sec = val * 3600
        return now + sec, float(sec)

    # 3. "every weekday [at] HH(:MM)?(am|pm)?"
    m_weekday = re.search(r"every\s+weekday(?:\s+at)?\s+(\d{1,2})(?::(\d{2}))?\s*(am|pm)?", s)
    if m_weekday:
        hour = int(m_weekday.group(1))
        minute = int(m_weekday.group(2) or 0)
        meridiem = m_weekday.group(3)
        if meridiem == "pm" and hour < 12:
            hour += 12
        elif meridiem == "am" and hour == 12:
            hour = 0

        target = dt_now.replace(hour=hour, minute=minute, second=0, microsecond=0)
        # Advance until it is a weekday and in the future
        while target <= dt_now or target.weekday() >= 5:
            target += timedelta(days=1)
        return target.timestamp(), 86400.0

    # 4. "daily [at] HH(:MM)?(am|pm)?" or "every day [at] HH(:MM)?(am|pm)?"
    m_daily = re.search(r"(?:daily|every\s+day)(?:\s+at)?\s+(\d{1,2})(?::(\d{2}))?\s*(am|pm)?", s)
    if m_daily:
        hour = int(m_daily.group(1))
        minute = int(m_daily.group(2) or 0)
        meridiem = m_daily.group(3)
        if meridiem == "pm" and hour < 12:
            hour += 12
        elif meridiem == "am" and hour == 12:
            hour = 0

        target = dt_now.replace(hour=hour, minute=minute, second=0, microsecond=0)
        if target <= dt_now:
            target += timedelta(days=1)
        return target.timestamp(), 86400.0

    # Fallback: default to 1 hour from now
    return now + 3600.0, 3600.0


class CronManager:
    def __init__(self, cron_dir: Optional[Path] = None):
        self.dir = cron_dir or get_cron_dir()
        self.dir.mkdir(parents=True, exist_ok=True)
        self.jobs_file = self.dir / "jobs.json"
        self._ensure_jobs_file()

    def _ensure_jobs_file(self):
        if not self.jobs_file.exists():
            self._write_jobs([])

    def _read_jobs(self) -> List[Dict[str, Any]]:
        try:
            if self.jobs_file.exists():
                return json.loads(self.jobs_file.read_text(encoding="utf-8"))
        except Exception:
            pass
        return []

    def _write_jobs(self, jobs: List[Dict[str, Any]]):
        self.jobs_file.write_text(json.dumps(jobs, indent=2), encoding="utf-8")

    def list_jobs(self) -> List[Dict[str, Any]]:
        return self._read_jobs()

    def create_job(
        self,
        name: str,
        schedule_nl: str,
        prompt: str,
        delivery: str = "in_app",
    ) -> Dict[str, Any]:
        """
        Creates scheduled job.
        Strictly enforces IN-APP ONLY delivery rule:
        Platform deliveries (telegram, discord, slack, feishu) are categorically rejected.
        """
        deliv = delivery.lower().strip()
        if deliv not in ALLOWED_DELIVERIES:
            raise ValueError(
                f"Platform delivery '{delivery}' is strictly disabled in pain ai desktop v1. "
                f"Only in-app notification delivery ('in_app') is permitted."
            )

        now = time.time()
        next_run, interval = parse_schedule_nl(schedule_nl, reference_time=now)
        job_id = f"job-{uuid.uuid4().hex[:8]}"

        job = {
            "id": job_id,
            "name": name,
            "schedule_nl": schedule_nl,
            "prompt": prompt,
            "delivery": "in_app",
            "enabled": True,
            "interval_sec": interval,
            "next_run": next_run,
            "last_run": None,
            "created_at": now,
            "history": [],
        }

        jobs = self._read_jobs()
        jobs.append(job)
        self._write_jobs(jobs)
        return job

    def toggle_job(self, job_id: str, enabled: bool) -> bool:
        jobs = self._read_jobs()
        found = False
        for j in jobs:
            if j["id"] == job_id:
                j["enabled"] = enabled
                found = True
                break
        if found:
            self._write_jobs(jobs)
        return found

    def delete_job(self, job_id: str) -> bool:
        jobs = self._read_jobs()
        orig_len = len(jobs)
        jobs = [j for j in jobs if j["id"] != job_id]
        if len(jobs) < orig_len:
            self._write_jobs(jobs)
            return True
        return False

    def trigger_job(self, job_id: str, output: str) -> Optional[Dict[str, Any]]:
        """Manually triggers a job immediately and appends to history.

        Phase 9: the output MUST describe a real execution — callers pass the
        actual agent result. There is no default success text (a bare
        trigger with no execution would fabricate success).
        """
        return self.record_run(job_id, status="success", output=output)

    def record_run(self, job_id: str, status: str, output: str) -> Optional[Dict[str, Any]]:
        """Append a run record for a REAL execution (scheduler or run-now).

        status: "success" | "error". output: actual agent output or the
        exception message. History keeps the last 5 runs; one-shot jobs
        (no interval) disable after firing.
        """
        jobs = self._read_jobs()
        for j in jobs:
            if j["id"] == job_id:
                now = time.time()
                record = {
                    "run_id": f"run-{uuid.uuid4().hex[:6]}",
                    "timestamp": now,
                    "status": status,
                    "output": output,
                    "scheduled_at": j.get("next_run", now),
                    "delta_sec": round(abs(now - j.get("next_run", now)), 2),
                }
                j["last_run"] = now
                if "history" not in j:
                    j["history"] = []
                j["history"].insert(0, record)
                j["history"] = j["history"][:5]  # Retain last 5 runs

                # Calculate next run
                if j.get("interval_sec"):
                    j["next_run"] = now + j["interval_sec"]
                else:
                    j["enabled"] = False  # One-shot completed

                self._write_jobs(jobs)
                return record
        return None

    def tick(self, now: Optional[float] = None) -> List[Dict[str, Any]]:
        """
        Phase 9: PURE scheduler primitive. Returns due job snapshots
        ({job_id, job_name, prompt, scheduled_at, delta_sec, ...}) and advances
        next_run / disables one-shots — but records NO history. History is
        written only by real execution (cron_scheduler.run_job_now), so a tick
        can never fabricate success. Disabled jobs are never due.
        """
        current_time = now or time.time()
        jobs = self._read_jobs()
        due_records = []
        updated = False

        for j in jobs:
            if not j.get("enabled", False):
                continue

            next_run = j.get("next_run", 0)
            if next_run <= current_time:
                delta = current_time - next_run
                due_records.append({
                    "job_id": j["id"],
                    "job_name": j["name"],
                    "prompt": j["prompt"],
                    "timestamp": current_time,
                    "scheduled_at": next_run,
                    "delta_sec": round(delta, 2),
                })

                # Advance schedule so a later tick does not re-fire.
                if j.get("interval_sec"):
                    j["next_run"] = current_time + j["interval_sec"]
                else:
                    j["enabled"] = False

                updated = True

        if updated:
            self._write_jobs(jobs)

        return due_records


# Global cron manager
_default_cron_manager: Optional[CronManager] = None

def get_cron_manager() -> CronManager:
    global _default_cron_manager
    if _default_cron_manager is None:
        _default_cron_manager = CronManager()
    return _default_cron_manager
