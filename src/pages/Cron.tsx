import { useState, useEffect } from 'react';
import {
  cronList,
  cronCreate,
  cronToggle,
  cronDelete,
  cronRunNow,
  type CronJob,
  type CronRunRecord,
} from '../lib/memory_cron';

export function Cron() {
  const [jobs, setJobs] = useState<CronJob[]>([]);
  const [nameInput, setNameInput] = useState('');
  const [nlInput, setNlInput] = useState('every weekday 8am');
  const [promptInput, setPromptInput] = useState('Summarize overnight git commits and active tasks');
  const [previewHint, setPreviewHint] = useState<string>('');
  const [toastMsg, setToastMsg] = useState<string | null>(null);
  const [nowTimestamp, setNowTimestamp] = useState<number>(Date.now());
  const [selectedJobHistory, setSelectedJobHistory] = useState<CronRunRecord[]>([]);

  useEffect(() => {
    loadJobs();
    // 30s countdown ticker update
    const interval = setInterval(() => {
      setNowTimestamp(Date.now());
    }, 30000);
    return () => clearInterval(interval);
  }, []);

  // Update preview hint whenever nlInput changes
  useEffect(() => {
    updatePreviewHint(nlInput);
  }, [nlInput]);

  const updatePreviewHint = (nl: string) => {
    const s = nl.toLowerCase().trim();
    if (s.includes('weekday')) {
      setPreviewHint('Schedule: Monday to Friday at specified hour (Local system timezone)');
    } else if (s.includes('daily') || s.includes('every day')) {
      setPreviewHint('Schedule: Every day (24-hour cadence) in local system timezone');
    } else if (s.startsWith('in ')) {
      setPreviewHint('Schedule: One-shot delayed run relative to current time');
    } else if (s.startsWith('every ')) {
      setPreviewHint('Schedule: Recurring periodic interval relative to reference time');
    } else {
      setPreviewHint('Schedule: Custom parsed interval (defaults to 1-hour interval)');
    }
  };

  const showToast = (msg: string) => {
    setToastMsg(msg);
    setTimeout(() => setToastMsg(null), 3500);
  };

  const loadJobs = async () => {
    try {
      const list = await cronList();
      setJobs(list);
      // Populate history from all jobs
      const allRuns: CronRunRecord[] = [];
      for (const j of list) {
        if (j.history) allRuns.push(...j.history);
      }
      allRuns.sort((a, b) => b.timestamp - a.timestamp);
      setSelectedJobHistory(allRuns.slice(0, 5));
    } catch (err) {
      console.error('Failed to load cron jobs:', err);
      showToast(`Error: failed to load cron jobs (${err})`);
    }
  };

  const handleCreate = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!nameInput.trim() || !promptInput.trim()) {
      showToast('Please provide a job name and prompt');
      return;
    }
    try {
      const created = await cronCreate(
        nameInput.trim(),
        nlInput.trim(),
        promptInput.trim(),
        'in_app'
      );
      showToast(`Created scheduled job "${created.name}"`);
      setNameInput('');
      loadJobs();
    } catch (err: any) {
      showToast(`Error: ${err?.toString() || 'Failed to create job'}`);
    }
  };

  const handleToggle = async (id: string, enabled: boolean) => {
    try {
      await cronToggle(id, enabled);
      setJobs((prev) =>
        prev.map((j) => (j.id === id ? { ...j, enabled } : j))
      );
      showToast(`${enabled ? 'Enabled' : 'Disabled'} job`);
    } catch (err) {
      showToast(`Error: failed to update job (${err})`);
    }
  };

  const handleDelete = async (id: string, name: string) => {
    try {
      const ok = await cronDelete(id);
      if (ok) {
        showToast(`Deleted job "${name}"`);
        loadJobs();
      }
    } catch (err) {
      showToast(`Error: failed to delete job (${err})`);
    }
  };

  const handleRunNow = async (id: string) => {
    try {
      const rec = await cronRunNow(id);
      showToast(`Job fired immediately (${rec.status}): ${rec.output}`);
      loadJobs();
    } catch (err) {
      console.error('Failed to run job now:', err);
      showToast(`Error: failed to run job (${err})`);
    }
  };

  const formatCountdown = (nextRunMs: number) => {
    const diff = nextRunMs - nowTimestamp;
    if (diff <= 0) return 'due now';
    const totalSecs = Math.floor(diff / 1000);
    const mins = Math.floor(totalSecs / 60);
    const secs = totalSecs % 60;
    if (mins >= 60) {
      const hrs = Math.floor(mins / 60);
      const remMins = mins % 60;
      return `in ${hrs}h ${remMins}m`;
    }
    return `in ${mins}m ${secs}s`;
  };

  return (
    <div className="flex-1 flex flex-col h-full bg-canvas overflow-y-auto">
      {/* Toast Notification */}
      {toastMsg && (
        <div className="fixed top-4 right-6 z-50 bg-surface-dark text-on-dark px-4 py-2.5 rounded-lg shadow-lg border border-surface-dark-elevated text-sm flex items-center space-x-2 animate-in fade-in slide-in-from-top-2 duration-150">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="text-accent-teal">
            <polyline points="20 6 9 17 4 12" />
          </svg>
          <span>{toastMsg}</span>
        </div>
      )}

      {/* Main Header Strip */}
      <div className="px-6 py-4 border-b border-hairline bg-surface-soft/60 flex items-center justify-between">
        <div>
          <h1 className="font-display text-[22px] font-medium text-ink leading-tight">Cron Jobs &amp; Scheduled Tasks</h1>
          <p className="text-xs text-muted">
            Natural language scheduler &middot; In-app delivery only &middot; Live 30s countdown &middot; Local timezone
          </p>
        </div>
        <div className="text-xs px-3 py-1.5 rounded-md bg-canvas border border-hairline font-mono text-muted">
          Active jobs: {jobs.filter((j) => j.enabled).length} / {jobs.length}
        </div>
      </div>

      <div className="max-w-[1000px] w-full mx-auto p-6 space-y-8">
        {/* Create Scheduled Task Box */}
        <div className="bg-surface-card border border-hairline-soft rounded-xl p-5 shadow-2xs">
          <h2 className="text-xs font-semibold uppercase tracking-wider text-muted mb-3">
            Schedule New Task (Natural Language)
          </h2>

          <form onSubmit={handleCreate} className="space-y-3 text-xs">
            <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
              <div>
                <label className="font-medium text-ink block mb-1">Job Name</label>
                <input
                  type="text"
                  value={nameInput}
                  onChange={(e) => setNameInput(e.target.value)}
                  placeholder="e.g. Daily Morning Brief, Check Stale Temp Files"
                  className="w-full h-8 px-3 rounded-lg bg-canvas border border-hairline text-xs text-ink focus:outline-none focus:border-primary"
                />
              </div>

              <div>
                <label className="font-medium text-ink block mb-1">Cadence / Schedule (Natural Language)</label>
                <input
                  type="text"
                  value={nlInput}
                  onChange={(e) => setNlInput(e.target.value)}
                  placeholder="every weekday 8am, every 2 hours, in 10 minutes..."
                  className="w-full h-8 px-3 rounded-lg bg-canvas border border-hairline font-mono text-xs text-ink focus:outline-none focus:border-primary"
                />
              </div>
            </div>

            <div>
              <label className="font-medium text-ink block mb-1">Task Prompt / Instructions</label>
              <input
                type="text"
                value={promptInput}
                onChange={(e) => setPromptInput(e.target.value)}
                placeholder="Instructions dispatched to agent upon schedule trigger..."
                className="w-full h-8 px-3 rounded-lg bg-canvas border border-hairline text-xs text-ink focus:outline-none focus:border-primary"
              />
            </div>

            {/* Validate-Hint Box */}
            <div className="p-2.5 rounded bg-surface-soft border border-hairline flex items-center justify-between text-xs">
              <div className="flex items-center space-x-2 text-muted">
                <span className="font-semibold text-ink">Parsed preview:</span>
                <span>{previewHint}</span>
              </div>
              <span className="font-mono text-[10px] uppercase tracking-wider text-accent-teal font-medium px-2 py-0.5 rounded bg-canvas border border-hairline">
                In-App Only
              </span>
            </div>

            <div className="flex items-center justify-between pt-1">
              <span className="text-[11px] text-muted">
                Platform delivery (Telegram/Discord) is refused per safety policy.
              </span>
              <button
                type="submit"
                className="px-4 py-2 rounded-lg bg-primary text-white text-xs font-medium hover:bg-primary-active transition-colors cursor-pointer"
              >
                + Create Scheduled Job
              </button>
            </div>
          </form>
        </div>

        {/* Scheduled Jobs Table */}
        <div>
          <div className="flex items-center justify-between mb-3">
            <h2 className="text-xs font-semibold uppercase tracking-wider text-muted">
              Configured Scheduled Jobs ({jobs.length})
            </h2>
            <span className="text-[11px] text-muted font-mono">
              Live ticker: 30s interval
            </span>
          </div>

          <div className="space-y-3">
            {jobs.length === 0 ? (
              <div className="text-center py-8 text-xs text-muted bg-surface-soft/40 rounded-xl border border-hairline">
                No scheduled jobs configured.
              </div>
            ) : (
              jobs.map((job) => (
                <div
                  key={job.id}
                  className="p-4 rounded-xl bg-surface-soft border border-hairline hover:border-hairline-strong transition-all flex flex-col md:flex-row md:items-center md:justify-between gap-3 text-xs"
                >
                  <div className="space-y-1">
                    <div className="flex items-center space-x-2">
                      <span className="font-bold text-ink text-sm">{job.name}</span>
                      <span className="text-[10px] font-mono uppercase tracking-wider px-2 py-0.5 rounded bg-surface-cream-strong text-body font-medium">
                        {job.schedule_nl}
                      </span>
                      <span
                        className={`text-[10px] font-mono px-2 py-0.5 rounded ${
                          job.enabled
                            ? 'bg-accent-teal/15 text-accent-teal font-semibold'
                            : 'bg-hairline text-muted'
                        }`}
                      >
                        {job.enabled ? 'ACTIVE' : 'PAUSED'}
                      </span>
                    </div>
                    <p className="text-body leading-relaxed">{job.prompt}</p>
                    <div className="flex items-center space-x-3 text-[11px] font-mono text-muted pt-0.5">
                      <span>Next run: {formatCountdown(job.next_run)}</span>
                      <span>&middot;</span>
                      <span>Delivery: {job.delivery}</span>
                    </div>
                  </div>

                  <div className="flex items-center space-x-3 flex-shrink-0 self-start md:self-center">
                    <button
                      type="button"
                      onClick={() => handleRunNow(job.id)}
                      className="px-2.5 py-1 rounded bg-canvas border border-hairline hover:bg-surface-cream-strong text-body text-[11px] font-medium transition-colors cursor-pointer"
                    >
                      Run Now
                    </button>

                    {/* Enable / Disable Switch */}
                    <button
                      type="button"
                      onClick={() => handleToggle(job.id, !job.enabled)}
                      className={`w-10 h-5 flex items-center rounded-full p-0.5 cursor-pointer transition-colors ${
                        job.enabled ? 'bg-primary' : 'bg-hairline'
                      }`}
                      aria-label="Toggle job"
                    >
                      <div
                        className={`bg-white w-4 h-4 rounded-full shadow-md transform transition-transform ${
                          job.enabled ? 'translate-x-5' : 'translate-x-0'
                        }`}
                      />
                    </button>

                    {/* Delete text link */}
                    <button
                      type="button"
                      onClick={() => handleDelete(job.id, job.name)}
                      className="text-error hover:underline text-[11px] font-medium cursor-pointer ml-1"
                    >
                      Delete
                    </button>
                  </div>
                </div>
              ))
            )}
          </div>
        </div>

        {/* Fired Runs History (Last 5) */}
        <div>
          <h2 className="text-xs font-semibold uppercase tracking-wider text-muted mb-3">
            Execution History (Last 5 Fired Runs)
          </h2>

          <div className="space-y-2">
            {selectedJobHistory.length === 0 ? (
              <div className="text-center py-6 text-xs text-muted bg-surface-soft/40 rounded-xl border border-hairline">
                No recent run history recorded.
              </div>
            ) : (
              selectedJobHistory.map((run) => (
                <div
                  key={run.run_id}
                  className="p-3 rounded-lg bg-canvas border border-hairline flex flex-col sm:flex-row sm:items-center sm:justify-between gap-2 text-xs"
                >
                  <div className="space-y-0.5">
                    <div className="flex items-center space-x-2">
                      <span
                        className={`text-[10px] font-mono uppercase px-1.5 py-0.2 rounded font-semibold ${
                          run.status === 'success'
                            ? 'bg-accent-teal/15 text-accent-teal'
                            : 'bg-error/15 text-error'
                        }`}
                      >
                        {run.status}
                      </span>
                      <span className="font-mono text-muted text-[11px]">
                        {new Date(run.timestamp).toLocaleTimeString()}
                      </span>
                      <span className="font-mono text-muted text-[11px]">
                        (delta: {run.delta_sec}s &le; 30s)
                      </span>
                    </div>
                    <div className="text-body font-mono text-[11px]">{run.output}</div>
                  </div>
                  <span className="text-[10px] font-mono text-muted self-start sm:self-center">
                    {run.run_id}
                  </span>
                </div>
              ))
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
