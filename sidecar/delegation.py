"""pain ai — Subagent Task Delegation Engine (delegate_task)
Source of truth: PRD.md §4.3 + hermes-map.md §6 + tools/delegate_tool.py
"""

from typing import Dict, Any, List, Optional
import concurrent.futures
import time

MAX_ALLOWED_PARALLEL = 3  # Strict v1 cap


class DelegationManager:
    def __init__(self, enabled: bool = True, max_parallel: int = 3):
        self.enabled = enabled
        self.max_parallel = min(MAX_ALLOWED_PARALLEL, max(1, max_parallel))

    def update_config(self, enabled: bool, max_parallel: int) -> Dict[str, Any]:
        self.enabled = enabled
        self.max_parallel = min(MAX_ALLOWED_PARALLEL, max(1, max_parallel))
        return {"enabled": self.enabled, "max_parallel": self.max_parallel}

    def get_config(self) -> Dict[str, Any]:
        return {"enabled": self.enabled, "max_parallel": self.max_parallel}

    def run_parallel_tasks(
        self,
        task_fns: List[Any],
    ) -> List[Any]:
        """
        Executes subagent tasks concurrently up to self.max_parallel limit.
        """
        if not self.enabled:
            raise RuntimeError("Subagent delegation is disabled in settings.")

        worker_count = min(self.max_parallel, len(task_fns))
        results = []

        with concurrent.futures.ThreadPoolExecutor(max_workers=worker_count) as executor:
            futures = [executor.submit(fn) for fn in task_fns]
            for f in concurrent.futures.as_completed(futures):
                results.append(f.result())

        return results


_default_delegation_manager: Optional[DelegationManager] = None

def get_delegation_manager() -> DelegationManager:
    global _default_delegation_manager
    if _default_delegation_manager is None:
        _default_delegation_manager = DelegationManager()
    return _default_delegation_manager
