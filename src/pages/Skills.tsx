import { useState, useEffect } from 'react';
import {
  skillsList,
  skillView,
  skillsTrust,
  skillsInstall,
  skillsRemove,
  learnDraftsList,
  learnDraftApprove,
  learnDraftReject,
  type SkillSummary,
  type SkillDetail,
  type QuarantineFinding,
  type LearnDraft,
} from '../lib/skills';
import { CodeCard } from '../components/CodeCard';

export function Skills() {
  const [skills, setSkills] = useState<SkillSummary[]>([]);
  const [selectedSkillName, setSelectedSkillName] = useState<string>('daily-brief');
  const [selectedSkill, setSelectedSkill] = useState<SkillDetail | null>(null);
  const [searchQuery, setSearchQuery] = useState('');
  const [filterSource, setFilterSource] = useState<'all' | 'bundled' | 'user' | 'project'>('all');
  const [loading, setLoading] = useState(false);
  const [quarantineFindings, setQuarantineFindings] = useState<QuarantineFinding[] | null>(null);
  const [quarantineError, setQuarantineError] = useState<string | null>(null);
  const [learnDrafts, setLearnDrafts] = useState<LearnDraft[]>([]);
  const [toastMsg, setToastMsg] = useState<string | null>(null);

  // Load skills list and drafts on mount
  useEffect(() => {
    loadSkills();
    loadDrafts();

    // Check URL parameters for quarantine demo view
    if (typeof window !== 'undefined') {
      const params = new URLSearchParams(window.location.search);
      if (params.get('view') === 'skills-quarantine') {
        triggerDirtyInstall();
      }
    }
  }, []);

  const showToast = (msg: string) => {
    setToastMsg(msg);
    setTimeout(() => setToastMsg(null), 3500);
  };

  const loadSkills = async () => {
    setLoading(true);
    try {
      const list = await skillsList();
      setSkills(list);
      if (list.length > 0 && !selectedSkillName) {
        setSelectedSkillName(list[0].name);
      }
    } catch (err) {
      console.error('Failed to load skills:', err);
      showToast(`Error: failed to load skills (${err})`);
    } finally {
      setLoading(false);
    }
  };

  const loadDrafts = async () => {
    try {
      const drafts = await learnDraftsList();
      setLearnDrafts(drafts);
    } catch (err) {
      console.error('Failed to load learn drafts:', err);
    }
  };

  // Load detail whenever selectedSkillName changes
  useEffect(() => {
    if (!selectedSkillName) return;
    skillView(selectedSkillName).then((detail) => {
      setSelectedSkill(detail);
    }).catch((err) => {
      console.error('Failed to load skill detail:', err);
      showToast(`Error: failed to load skill "${selectedSkillName}" (${err})`);
    });
  }, [selectedSkillName]);

  const handleTrustToggle = async () => {
    if (!selectedSkill) return;
    const nextState = !selectedSkill.trusted;
    try {
      const success = await skillsTrust('pain-ai', selectedSkill.name, nextState);
      if (success) {
        setSelectedSkill({ ...selectedSkill, trusted: nextState });
        setSkills((prev) =>
          prev.map((s) => (s.name === selectedSkill.name ? { ...s, trusted: nextState } : s))
        );
        showToast(
          nextState
            ? `Trust granted to project skill "${selectedSkill.name}"`
            : `Trust revoked from project skill "${selectedSkill.name}"`
        );
      }
    } catch (err) {
      showToast(`Error: trust update failed (${err})`);
    }
  };

  const handleRemoveSkill = async (name: string) => {
    try {
      const ok = await skillsRemove(name);
      if (ok) {
        showToast(`Removed skill ${name}`);
        loadSkills();
      }
    } catch (err) {
      showToast(`Error: failed to remove skill (${err})`);
    }
  };

  const triggerCleanInstall = async () => {
    setQuarantineFindings(null);
    setQuarantineError(null);
    const result = await skillsInstall('tests/fixtures/hub-tap/clean-tool', 'clean-tool');
    if (result.ok) {
      showToast(`Skill "${result.name}" passed quarantine scanner and installed cleanly (pinned ${result.version || '1.0.0'})`);
      loadSkills();
    } else {
      setQuarantineError(result.error || 'Installation failed');
      setQuarantineFindings(result.findings || null);
    }
  };

  const triggerDirtyInstall = async () => {
    setQuarantineError(null);
    const result = await skillsInstall('tests/fixtures/hub-tap/dirty-tool', 'dirty-tool');
    if (!result.ok) {
      setQuarantineError(result.error || 'Quarantine security violation detected');
      // Phase 2: render only scanner-returned findings; never fabricate entries.
      setQuarantineFindings(result.findings || []);
    }
  };

  const handleApproveDraft = async (id: string) => {
    try {
      await learnDraftApprove(id);
      showToast('Learned skill draft approved and activated into user skills');
    } catch (err) {
      showToast(`Error: draft approval unavailable (${err})`);
    }
    loadDrafts();
    loadSkills();
  };

  const handleRejectDraft = async (id: string) => {
    try {
      await learnDraftReject(id);
      showToast('Skill draft rejected');
    } catch (err) {
      showToast(`Error: draft rejection unavailable (${err})`);
    }
    loadDrafts();
  };

  const filteredSkills = skills.filter((s) => {
    const matchesSearch =
      s.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      s.description.toLowerCase().includes(searchQuery.toLowerCase());
    const matchesSource = filterSource === 'all' || s.source === filterSource;
    return matchesSearch && matchesSource;
  });

  return (
    <div className="flex-1 flex flex-col h-full bg-canvas overflow-hidden">
      {/* Toast Notification */}
      {toastMsg && (
        <div className="absolute top-4 right-6 z-50 bg-surface-dark text-on-dark px-4 py-2.5 rounded-lg shadow-lg border border-surface-dark-elevated text-sm flex items-center space-x-2 animate-in fade-in slide-in-from-top-2 duration-150">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="text-accent-teal">
            <polyline points="20 6 9 17 4 12" />
          </svg>
          <span>{toastMsg}</span>
        </div>
      )}

      {/* Main Header Strip */}
      <div className="px-6 py-4 border-b border-hairline bg-surface-soft/60 flex items-center justify-between">
        <div>
          <h1 className="font-display text-[22px] font-medium text-ink leading-tight">Skills System</h1>
          <p className="text-xs text-muted">
            Agent capabilities &middot; Progressive disclosure &middot; Hub quarantine scanner &middot; Project trust-gated
          </p>
        </div>
        <div className="flex items-center space-x-3">
          <button
            type="button"
            onClick={triggerCleanInstall}
            className="text-xs px-3 py-1.5 rounded-md bg-canvas border border-hairline hover:border-muted font-medium text-body cursor-pointer transition-colors shadow-2xs"
            title="Install verified clean skill from Hub"
          >
            + Test Clean Hub Install
          </button>
          <button
            type="button"
            onClick={triggerDirtyInstall}
            className="text-xs px-3 py-1.5 rounded-md bg-error/10 border border-error/30 hover:bg-error/20 font-medium text-error cursor-pointer transition-colors"
            title="Test quarantine scanner with secret and destructive command"
          >
            &times; Test Dirty Install (Quarantine)
          </button>
        </div>
      </div>

      {/* Two Column Layout */}
      <div className="flex-1 flex flex-col lg:flex-row min-h-0 overflow-hidden">
        {/* Left Column: List & Filters */}
        <div className="w-full lg:w-[380px] lg:min-w-[380px] border-r border-hairline flex flex-col h-full bg-surface-soft/30 overflow-hidden">
          {/* Search & Filter pills */}
          <div className="p-4 border-b border-hairline space-y-3">
            <div className="relative">
              <input
                type="text"
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                placeholder="Filter skills by name or keyword..."
                className="w-full h-9 pl-9 pr-3 rounded-lg bg-canvas border border-hairline text-xs text-ink placeholder:text-muted-soft focus:outline-none focus:border-primary transition-colors"
              />
              <svg
                width="14"
                height="14"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2"
                strokeLinecap="round"
                strokeLinejoin="round"
                className="absolute left-3 top-2.5 text-muted"
                aria-hidden="true"
              >
                <circle cx="11" cy="11" r="8" />
                <line x1="21" y1="21" x2="16.65" y2="16.65" />
              </svg>
            </div>

            <div className="flex items-center space-x-1.5 text-xs select-none">
              {(['all', 'bundled', 'user', 'project'] as const).map((tab) => (
                <button
                  key={tab}
                  type="button"
                  onClick={() => setFilterSource(tab)}
                  className={`px-2.5 py-1 rounded-md capitalize font-medium transition-colors cursor-pointer ${
                    filterSource === tab
                      ? 'bg-canvas text-ink border border-hairline shadow-2xs'
                      : 'text-muted hover:text-ink'
                  }`}
                >
                  {tab}
                </button>
              ))}
            </div>
          </div>

          {/* Skill Cards Scroll Area */}
          <div className="flex-1 overflow-y-auto p-4 space-y-2.5">
            {loading ? (
              <div className="text-center py-8 text-xs text-muted">Loading skills index...</div>
            ) : filteredSkills.length === 0 ? (
              <div className="text-center py-8 text-xs text-muted">No skills found.</div>
            ) : (
              filteredSkills.map((s) => {
                const isSelected = selectedSkillName === s.name;
                const isProject = s.source === 'project';
                return (
                  <div
                    key={s.name}
                    onClick={() => setSelectedSkillName(s.name)}
                    className={`p-3 rounded-lg border text-left cursor-pointer transition-all ${
                      isSelected
                        ? 'bg-canvas border-primary ring-1 ring-primary/20 shadow-xs'
                        : 'bg-canvas/70 border-hairline hover:border-hairline-strong hover:bg-canvas'
                    }`}
                  >
                    <div className="flex items-center justify-between mb-1">
                      <span className="font-mono text-[13px] font-semibold text-ink">{s.name}</span>
                      <div className="flex items-center space-x-1.5">
                        {s.version && (
                          <span className="text-[10px] font-mono text-muted px-1.5 py-0.5 rounded bg-surface-soft">
                            v{s.version}
                          </span>
                        )}
                        <span
                          className={`text-[10px] uppercase tracking-wider font-medium px-1.5 py-0.5 rounded ${
                            s.source === 'bundled'
                              ? 'bg-surface-cream-strong/70 text-body'
                              : s.source === 'user'
                              ? 'bg-accent-teal/15 text-accent-teal'
                              : 'bg-accent-amber/15 text-accent-amber'
                          }`}
                        >
                          {s.source}
                        </span>
                      </div>
                    </div>

                    <p className="text-xs text-muted line-clamp-2 leading-relaxed">{s.description}</p>

                    {isProject && (
                      <div className="mt-2 pt-2 border-t border-hairline-soft flex items-center justify-between text-[11px]">
                        <span className="text-muted">Trust status:</span>
                        {s.trusted ? (
                          <span className="text-emerald-700 font-medium flex items-center space-x-1">
                            <span className="w-1.5 h-1.5 rounded-full bg-emerald-500" />
                            <span>Trusted</span>
                          </span>
                        ) : (
                          <span className="text-amber-800 font-medium flex items-center space-x-1">
                            <span className="w-1.5 h-1.5 rounded-full bg-amber-500" />
                            <span>Untrusted (Gated)</span>
                          </span>
                        )}
                      </div>
                    )}
                  </div>
                );
              })
            )}

            {/* Agent Learned Drafts Section (/learn) */}
            {learnDrafts.length > 0 && (
              <div className="mt-6 pt-4 border-t border-hairline">
                <div className="flex items-center justify-between mb-2">
                  <span className="text-xs font-semibold text-ink uppercase tracking-wider">
                    Agent Drafts (/learn)
                  </span>
                  <span className="text-[10px] text-muted">Approval required</span>
                </div>
                {learnDrafts.map((d) => (
                  <div
                    key={d.draft_id}
                    className="p-3 mb-2 rounded-lg bg-surface-card border border-hairline text-xs"
                  >
                    <div className="font-mono font-medium text-ink mb-1">{d.name}</div>
                    <p className="text-muted mb-2">{d.description}</p>
                    <div className="flex items-center space-x-2">
                      <button
                        type="button"
                        onClick={() => handleApproveDraft(d.draft_id)}
                        className="px-2.5 py-1 rounded bg-primary text-white text-[11px] font-medium hover:bg-primary-active transition-colors cursor-pointer"
                      >
                        Approve
                      </button>
                      <button
                        type="button"
                        onClick={() => handleRejectDraft(d.draft_id)}
                        className="px-2.5 py-1 rounded bg-canvas border border-hairline text-body text-[11px] hover:bg-surface-cream-strong transition-colors cursor-pointer"
                      >
                        Reject
                      </button>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>

        {/* Right Column: Selected Skill Inspection & Quarantine Report */}
        <div className="flex-1 flex flex-col h-full overflow-y-auto p-6 bg-canvas">
          {/* Quarantine Findings Banner (Shown if audit fails) */}
          {quarantineError && (
            <div className="mb-6 p-5 rounded-xl bg-error/10 border-2 border-error/30 text-ink shadow-sm animate-in fade-in duration-200">
              <div className="flex items-start justify-between">
                <div className="flex items-center space-x-2.5">
                  <div className="w-7 h-7 rounded-full bg-error text-white flex items-center justify-center flex-shrink-0 font-bold text-sm">
                    !
                  </div>
                  <div>
                    <h3 className="font-semibold text-error text-[15px] leading-tight">
                      Quarantine Scanner Violation — Installation Blocked
                    </h3>
                    <p className="text-xs text-muted-soft mt-0.5">
                      {quarantineError}
                    </p>
                  </div>
                </div>
                <button
                  type="button"
                  onClick={() => {
                    setQuarantineError(null);
                    setQuarantineFindings(null);
                  }}
                  className="text-xs text-muted hover:text-ink cursor-pointer px-2 py-1 rounded border border-hairline"
                >
                  Dismiss
                </button>
              </div>

              {quarantineFindings && quarantineFindings.length > 0 && (
                <div className="mt-4 pt-4 border-t border-error/20">
                  <div className="text-xs font-semibold text-error mb-2 uppercase tracking-wide">
                    Flagged Invariants ({quarantineFindings.length} violations):
                  </div>
                  <div className="space-y-2">
                    {quarantineFindings.map((f, i) => (
                      <div
                        key={i}
                        className="p-2.5 rounded bg-surface-dark text-on-dark font-mono text-xs border border-surface-dark-elevated"
                      >
                        <div className="flex items-center justify-between text-[11px] text-on-dark-soft mb-1">
                          <span className="text-error font-semibold uppercase">{f.rule}</span>
                          <span>
                            {f.file}:{f.line}
                          </span>
                        </div>
                        <div className="text-accent-amber overflow-x-auto whitespace-pre-wrap">
                          {f.snippet}
                        </div>
                      </div>
                    ))}
                  </div>
                </div>
              )}
            </div>
          )}

          {selectedSkill ? (
            <div className="space-y-6 max-w-[840px] w-full mx-auto">
              {/* Top Details Card */}
              <div className="p-5 rounded-xl bg-surface-soft border border-hairline">
                <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-3">
                  <div>
                    <div className="flex items-center space-x-2.5">
                      <h2 className="font-mono text-lg font-bold text-ink">{selectedSkill.name}</h2>
                      {selectedSkill.version && (
                        <span className="text-xs font-mono text-muted px-2 py-0.5 rounded bg-canvas border border-hairline">
                          v{selectedSkill.version}
                        </span>
                      )}
                      <span
                        className={`text-xs uppercase font-medium px-2 py-0.5 rounded ${
                          selectedSkill.source === 'bundled'
                            ? 'bg-surface-cream-strong text-body'
                            : selectedSkill.source === 'user'
                            ? 'bg-accent-teal/15 text-accent-teal'
                            : 'bg-accent-amber/15 text-accent-amber'
                        }`}
                      >
                        {selectedSkill.source}
                      </span>
                    </div>
                    <p className="text-xs font-mono text-muted mt-1 break-all">
                      Location: {selectedSkill.source_dir}
                    </p>
                  </div>

                  <div className="flex items-center space-x-2">
                    {selectedSkill.source === 'project' && (
                      <button
                        type="button"
                        onClick={handleTrustToggle}
                        className={`px-3 py-1.5 rounded-lg text-xs font-medium cursor-pointer transition-colors border ${
                          selectedSkill.trusted
                            ? 'bg-emerald-50 text-emerald-800 border-emerald-300 hover:bg-emerald-100'
                            : 'bg-amber-50 text-amber-800 border-amber-300 hover:bg-amber-100'
                        }`}
                      >
                        {selectedSkill.trusted ? 'Revoke Trust' : 'Grant Trust (Allow Execution)'}
                      </button>
                    )}

                    {selectedSkill.source === 'user' && (
                      <button
                        type="button"
                        onClick={() => handleRemoveSkill(selectedSkill.name)}
                        className="px-3 py-1.5 rounded-lg text-xs font-medium text-error border border-error/30 hover:bg-error/10 cursor-pointer transition-colors"
                      >
                        Remove
                      </button>
                    )}
                  </div>
                </div>

                <p className="text-sm text-body mt-3 leading-relaxed">{selectedSkill.description}</p>

                {selectedSkill.source === 'project' && (
                  <div className="mt-3 p-2.5 rounded bg-amber-50/70 border border-amber-200 text-amber-900 text-xs flex items-center space-x-2">
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="text-amber-700 flex-shrink-0">
                      <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />
                    </svg>
                    <span>
                      <strong>Project Trust-Gate:</strong> Untrusted skills in repositories never execute without explicit operator approval.
                    </span>
                  </div>
                )}
              </div>

              {/* Requirements & Tools Grid */}
              <div className="grid grid-cols-1 md:grid-cols-3 gap-4 text-xs">
                <div className="p-3.5 rounded-lg bg-surface-soft/60 border border-hairline">
                  <span className="font-semibold text-ink block mb-2">Required Tools</span>
                  <div className="flex flex-wrap gap-1">
                    {selectedSkill.tools_required && selectedSkill.tools_required.length > 0 ? (
                      selectedSkill.tools_required.map((tool) => (
                        <span
                          key={tool}
                          className="font-mono text-[11px] px-1.5 py-0.5 rounded bg-canvas border border-hairline text-body"
                        >
                          {tool}
                        </span>
                      ))
                    ) : (
                      <span className="text-muted italic">None declared</span>
                    )}
                  </div>
                </div>

                <div className="p-3.5 rounded-lg bg-surface-soft/60 border border-hairline">
                  <span className="font-semibold text-ink block mb-2">Required Env Vars</span>
                  <div className="flex flex-wrap gap-1">
                    {selectedSkill.required_environment_variables &&
                    selectedSkill.required_environment_variables.length > 0 ? (
                      selectedSkill.required_environment_variables.map((env) => (
                        <span
                          key={env}
                          className="font-mono text-[11px] px-1.5 py-0.5 rounded bg-canvas border border-hairline text-body"
                        >
                          {env}
                        </span>
                      ))
                    ) : (
                      <span className="text-muted italic">None required</span>
                    )}
                  </div>
                </div>

                <div className="p-3.5 rounded-lg bg-surface-soft/60 border border-hairline">
                  <span className="font-semibold text-ink block mb-2">Platforms</span>
                  <div className="flex flex-wrap gap-1">
                    {selectedSkill.platforms && selectedSkill.platforms.length > 0 ? (
                      selectedSkill.platforms.map((p) => (
                        <span
                          key={p}
                          className="font-mono text-[11px] px-1.5 py-0.5 rounded bg-canvas border border-hairline text-body capitalize"
                        >
                          {p}
                        </span>
                      ))
                    ) : (
                      <span className="font-mono text-[11px] px-1.5 py-0.5 rounded bg-canvas border border-hairline text-body">
                        All OS
                      </span>
                    )}
                  </div>
                </div>
              </div>

              {/* Full SKILL.md Markdown Display */}
              <div>
                <div className="flex items-center justify-between mb-2">
                  <span className="text-xs font-semibold uppercase tracking-wider text-muted">
                    Skill Definition &amp; Instructions
                  </span>
                  <span className="text-xs text-muted font-mono">SKILL.md</span>
                </div>
                <CodeCard
                  filename="SKILL.md"
                  lang="markdown"
                  content={selectedSkill.content || `# ${selectedSkill.name}\n\n${selectedSkill.description}`}
                />
              </div>
            </div>
          ) : (
            <div className="flex-1 flex items-center justify-center text-muted text-sm">
              Select a skill to inspect its manifest and permissions
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
