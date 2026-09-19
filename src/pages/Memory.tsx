import { useState, useEffect } from 'react';
import {
  memoryGet,
  memoryEdit,
  sessionSearch,
  type MemoryDoc,
  type SessionSearchHit,
} from '../lib/memory_cron';
import {
  learnDraftsList,
  learnDraftApprove,
  learnDraftReject,
  type LearnDraft,
} from '../lib/skills';
import { useAppStore } from '../store';
import { DiffView, type DiffData } from '../components/DiffView';

export function Memory() {
  const { setActiveTab, setDraft } = useAppStore();
  const [memoryDoc, setMemoryDoc] = useState<MemoryDoc | null>(null);
  const [userDoc, setUserDoc] = useState<MemoryDoc | null>(null);
  const [searchQuery, setSearchQuery] = useState('');
  const [searchResults, setSearchResults] = useState<SessionSearchHit[]>([]);
  const [learnDrafts, setLearnDrafts] = useState<LearnDraft[]>([]);
  const [editingTarget, setEditingTarget] = useState<'memory' | 'user' | null>(null);
  const [editContent, setEditContent] = useState('');
  const [activeDiff, setActiveDiff] = useState<DiffData | null>(null);
  const [toastMsg, setToastMsg] = useState<string | null>(null);
  const [isSearching, setIsSearching] = useState(false);

  useEffect(() => {
    loadMemoryDocs();
    loadDrafts();
    runSearch('gate');
  }, []);

  const showToast = (msg: string) => {
    setToastMsg(msg);
    setTimeout(() => setToastMsg(null), 3500);
  };

  const loadMemoryDocs = async () => {
    try {
      const m = await memoryGet('memory');
      setMemoryDoc(m);
      const u = await memoryGet('user');
      setUserDoc(u);
    } catch (err) {
      console.error('Failed to load memory documents:', err);
      showToast(`Error: memory unavailable (${err})`);
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

  const runSearch = async (query: string) => {
    setIsSearching(true);
    try {
      const hits = await sessionSearch(query);
      setSearchResults(hits);
    } catch (err) {
      console.error('Failed to search sessions:', err);
    } finally {
      setIsSearching(false);
    }
  };

  const handleStartEdit = (target: 'memory' | 'user') => {
    const doc = target === 'memory' ? memoryDoc : userDoc;
    if (!doc) return;
    setEditingTarget(target);
    setEditContent(doc.content);
  };

  const handlePreviewDiff = () => {
    if (!editingTarget) return;
    const doc = editingTarget === 'memory' ? memoryDoc : userDoc;
    if (!doc) return;

    const diff: DiffData = {
      path: doc.path,
      original: doc.content,
      modified: editContent,
      diff: `--- a/${doc.name}\n+++ b/${doc.name}\n@@ -1,${doc.content.split('\n').length} +1,${editContent.split('\n').length} @@\n` +
        doc.content.split('\n').map((l) => `-${l}`).join('\n') + '\n' +
        editContent.split('\n').map((l) => `+${l}`).join('\n'),
    };
    setActiveDiff(diff);
  };

  const handleSaveMemory = async () => {
    if (!editingTarget) return;
    const res = await memoryEdit(editingTarget, editContent);
    if (res.ok) {
      showToast(`Updated ${res.name} successfully (${res.char_count}/${res.char_limit} chars)`);
      setActiveDiff(null);
      setEditingTarget(null);
      loadMemoryDocs();
    } else {
      showToast(`Error: ${res.error || 'Failed to save memory'}`);
    }
  };

  const handleApproveDraft = async (id: string) => {
    try {
      await learnDraftApprove(id);
      showToast('Learned skill approved and added to active skills');
    } catch (err) {
      showToast(`Error: draft approval unavailable (${err})`);
    }
    loadDrafts();
  };

  const handleRejectDraft = async (id: string) => {
    try {
      await learnDraftReject(id);
      showToast('Draft rejected');
    } catch (err) {
      showToast(`Error: draft rejection unavailable (${err})`);
    }
    loadDrafts();
  };

  const handleOpenInChat = (snippet: string) => {
    const cleanText = snippet.replace(/<[^>]+>/g, '');
    setDraft(`Regarding previous session context: "${cleanText}"`);
    setActiveTab('Chat');
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
          <h1 className="font-display text-[22px] font-medium text-ink leading-tight">Memory &amp; Past Sessions</h1>
          <p className="text-xs text-muted">
            Curated notes (<code className="font-mono text-[11px]">MEMORY.md</code>, <code className="font-mono text-[11px]">USER.md</code>) &middot; SQLite FTS5 conversation recall &middot; /learn approval queue
          </p>
        </div>
        <div className="text-xs px-3 py-1.5 rounded-md bg-canvas border border-hairline font-mono text-muted">
          State: ~/.pain-ai/state.db
        </div>
      </div>

      <div className="max-w-[1000px] w-full mx-auto p-6 space-y-8">
        {/* Top Section: MEMORY.md and USER.md Cards */}
        <div>
          <div className="flex items-center justify-between mb-3">
            <h2 className="text-xs font-semibold uppercase tracking-wider text-muted">
              Curated Long-Term Memory Notes
            </h2>
            <span className="text-[11px] text-muted">
              Snapshot frozen at session start &middot; Mid-session writes hit disk
            </span>
          </div>

          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            {/* MEMORY.md Card */}
            <div className="bg-surface-card border border-hairline-soft rounded-xl p-5 flex flex-col justify-between shadow-2xs hover:border-hairline transition-colors">
              <div>
                <div className="flex items-center justify-between mb-2">
                  <div className="flex items-center space-x-2">
                    <span className="font-mono font-bold text-ink text-sm">MEMORY.md</span>
                    <span className="text-[10px] font-mono px-1.5 py-0.2 rounded bg-surface-cream-strong text-body">
                      facts &amp; context
                    </span>
                  </div>
                  <div className="text-xs font-mono text-muted">
                    {memoryDoc?.char_count || 0} / {memoryDoc?.char_limit || 2200} chars
                  </div>
                </div>

                <div className="w-full bg-hairline-soft h-1.5 rounded-full overflow-hidden mb-3">
                  <div
                    className={`h-full ${
                      (memoryDoc?.char_count || 0) > 2200 ? 'bg-error' : 'bg-primary'
                    }`}
                    style={{
                      width: `${Math.min(100, ((memoryDoc?.char_count || 0) / 2200) * 100)}%`,
                    }}
                  />
                </div>

                <div className="p-3 bg-canvas/80 rounded-lg border border-hairline text-xs font-mono text-body whitespace-pre-wrap max-h-36 overflow-y-auto leading-relaxed">
                  {memoryDoc?.content || 'Loading memory notes...'}
                </div>
              </div>

              <div className="pt-3 mt-3 border-t border-hairline flex items-center justify-between">
                <span className="text-[11px] text-muted font-mono">
                  Hard cap: 2,200 chars (~550 tokens)
                </span>
                <button
                  type="button"
                  onClick={() => handleStartEdit('memory')}
                  className="px-3 py-1 rounded bg-canvas border border-hairline hover:border-muted text-xs font-medium text-body transition-colors cursor-pointer"
                >
                  Edit MEMORY.md &rarr;
                </button>
              </div>
            </div>

            {/* USER.md Card */}
            <div className="bg-surface-card border border-hairline-soft rounded-xl p-5 flex flex-col justify-between shadow-2xs hover:border-hairline transition-colors">
              <div>
                <div className="flex items-center justify-between mb-2">
                  <div className="flex items-center space-x-2">
                    <span className="font-mono font-bold text-ink text-sm">USER.md</span>
                    <span className="text-[10px] font-mono px-1.5 py-0.2 rounded bg-accent-teal/15 text-accent-teal font-medium">
                      profile &amp; preferences
                    </span>
                  </div>
                  <div className="text-xs font-mono text-muted">
                    {userDoc?.char_count || 0} / {userDoc?.char_limit || 1375} chars
                  </div>
                </div>

                <div className="w-full bg-hairline-soft h-1.5 rounded-full overflow-hidden mb-3">
                  <div
                    className={`h-full ${
                      (userDoc?.char_count || 0) > 1375 ? 'bg-error' : 'bg-accent-teal'
                    }`}
                    style={{
                      width: `${Math.min(100, ((userDoc?.char_count || 0) / 1375) * 100)}%`,
                    }}
                  />
                </div>

                <div className="p-3 bg-canvas/80 rounded-lg border border-hairline text-xs font-mono text-body whitespace-pre-wrap max-h-36 overflow-y-auto leading-relaxed">
                  {userDoc?.content || 'Loading user preferences...'}
                </div>
              </div>

              <div className="pt-3 mt-3 border-t border-hairline flex items-center justify-between">
                <span className="text-[11px] text-muted font-mono">
                  Hard cap: 1,375 chars (~350 tokens)
                </span>
                <button
                  type="button"
                  onClick={() => handleStartEdit('user')}
                  className="px-3 py-1 rounded bg-canvas border border-hairline hover:border-muted text-xs font-medium text-body transition-colors cursor-pointer"
                >
                  Edit USER.md &rarr;
                </button>
              </div>
            </div>
          </div>
        </div>

        {/* Edit Modal with Diff Preview and Gate Approval */}
        {editingTarget && (
          <div className="fixed inset-0 z-50 bg-black/40 flex items-center justify-center p-4">
            <div className="bg-canvas border border-hairline rounded-xl max-w-[640px] w-full p-6 shadow-2xl space-y-4 animate-in fade-in duration-150">
              <div className="flex items-center justify-between pb-3 border-b border-hairline">
                <div>
                  <h3 className="font-semibold text-ink text-base">
                    Edit {editingTarget === 'memory' ? 'MEMORY.md' : 'USER.md'}
                  </h3>
                  <p className="text-xs text-muted">
                    Limit: {editingTarget === 'memory' ? '2,200' : '1,375'} characters. Edits require gate approval.
                  </p>
                </div>
                <button
                  type="button"
                  onClick={() => {
                    setEditingTarget(null);
                    setActiveDiff(null);
                  }}
                  className="text-muted hover:text-ink cursor-pointer p-1"
                >
                  &times;
                </button>
              </div>

              <div>
                <textarea
                  value={editContent}
                  onChange={(e) => setEditContent(e.target.value)}
                  rows={8}
                  className="w-full p-3 rounded-lg bg-surface-soft border border-hairline font-mono text-xs text-ink focus:outline-none focus:border-primary resize-y"
                  placeholder="Enter markdown content..."
                />
                <div className="flex items-center justify-between text-xs font-mono text-muted mt-1">
                  <span>Characters: {editContent.length}</span>
                  <span
                    className={
                      editContent.length > (editingTarget === 'memory' ? 2200 : 1375)
                        ? 'text-error font-semibold'
                        : 'text-muted'
                    }
                  >
                    Max allowed: {editingTarget === 'memory' ? 2200 : 1375}
                  </span>
                </div>
              </div>

              {activeDiff && (
                <div className="pt-2">
                  <span className="text-xs font-semibold text-muted block mb-1">
                    Diff Preview (Before Gate Approval):
                  </span>
                  <DiffView
                    diffData={activeDiff}
                    onAccept={handleSaveMemory}
                    onReject={() => setActiveDiff(null)}
                  />
                </div>
              )}

              {!activeDiff && (
                <div className="pt-3 border-t border-hairline flex items-center justify-end space-x-2">
                  <button
                    type="button"
                    onClick={() => setEditingTarget(null)}
                    className="px-3 py-1.5 rounded text-xs font-medium text-body hover:bg-surface-soft cursor-pointer"
                  >
                    Cancel
                  </button>
                  <button
                    type="button"
                    onClick={handlePreviewDiff}
                    className="px-3 py-1.5 rounded bg-surface-cream-strong text-ink text-xs font-medium hover:bg-surface-soft transition-colors cursor-pointer border border-hairline"
                  >
                    Preview Diff
                  </button>
                  <button
                    type="button"
                    onClick={handleSaveMemory}
                    className="px-3 py-1.5 rounded bg-primary text-white text-xs font-medium hover:bg-primary-active transition-colors cursor-pointer"
                  >
                    Save &amp; Persist
                  </button>
                </div>
              )}
            </div>
          </div>
        )}

        {/* Middle Section: Session Search (SQLite FTS5 + BM25 ranking) */}
        <div>
          <div className="flex items-center justify-between mb-3">
            <h2 className="text-xs font-semibold uppercase tracking-wider text-muted">
              Conversation Recall (SQLite FTS5 Full-Text Search)
            </h2>
            <span className="text-[11px] text-muted">
              3-Step: Discovery (~20ms, BM25) &rarr; Scroll &rarr; Read
            </span>
          </div>

          <div className="p-4 rounded-xl bg-surface-soft border border-hairline space-y-3">
            <div className="flex items-center space-x-2">
              <div className="relative flex-1">
                <input
                  type="text"
                  value={searchQuery}
                  onChange={(e) => {
                    setSearchQuery(e.target.value);
                    runSearch(e.target.value);
                  }}
                  placeholder="Search previous sessions by keyword (e.g., 'gate', 'accessibility', 'briefing')..."
                  className="w-full h-9 pl-9 pr-3 rounded-lg bg-canvas border border-hairline text-xs text-ink placeholder:text-muted-soft focus:outline-none focus:border-primary"
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
                >
                  <circle cx="11" cy="11" r="8" />
                  <line x1="21" y1="21" x2="16.65" y2="16.65" />
                </svg>
              </div>
              <button
                type="button"
                onClick={() => runSearch(searchQuery)}
                className="px-3 py-2 rounded-lg bg-canvas border border-hairline hover:border-muted text-xs font-medium text-ink cursor-pointer"
              >
                {isSearching ? 'Searching...' : 'Search'}
              </button>
            </div>

            {/* Results List */}
            <div className="space-y-2 pt-2">
              {searchResults.length === 0 ? (
                <div className="text-center py-6 text-xs text-muted">
                  No previous sessions matched "{searchQuery}".
                </div>
              ) : (
                searchResults.map((hit) => (
                  <div
                    key={`${hit.session_id}-${hit.message_id}`}
                    className="p-3 rounded-lg bg-canvas border border-hairline hover:border-hairline-strong transition-all flex flex-col sm:flex-row sm:items-center sm:justify-between gap-2 text-xs"
                  >
                    <div className="space-y-1">
                      <div className="flex items-center space-x-2">
                        <span className="font-semibold text-ink">{hit.session_title}</span>
                        <span className="text-[10px] font-mono px-1.5 py-0.2 rounded bg-surface-soft text-muted">
                          {hit.role}
                        </span>
                        <span
                          className={`text-[10px] font-mono uppercase px-1.5 py-0.2 rounded ${
                            hit.session_source === 'cron'
                              ? 'bg-hairline text-muted'
                              : 'bg-accent-teal/15 text-accent-teal font-medium'
                          }`}
                        >
                          {hit.session_source}
                        </span>
                        <span className="text-[11px] font-mono text-muted">
                          BM25: {hit.rank_score.toFixed(2)}
                        </span>
                      </div>
                      <div
                        className="text-muted leading-relaxed"
                        dangerouslySetInnerHTML={{ __html: hit.snippet }}
                      />
                    </div>

                    <button
                      type="button"
                      onClick={() => handleOpenInChat(hit.snippet)}
                      className="px-2.5 py-1.5 rounded bg-surface-soft border border-hairline hover:bg-surface-cream-strong text-body text-[11px] font-medium transition-colors cursor-pointer flex-shrink-0 self-start sm:self-center"
                    >
                      Open in Chat &rarr;
                    </button>
                  </div>
                ))
              )}
            </div>
          </div>
        </div>

        {/* Bottom Section: /learn Drafts Queue */}
        {learnDrafts.length > 0 && (
          <div>
            <div className="flex items-center justify-between mb-3">
              <h2 className="text-xs font-semibold uppercase tracking-wider text-muted">
                Agent-Created Skill Drafts (/learn queue)
              </h2>
              <span className="text-[11px] text-muted">
                write_approval=True &middot; guard_agent_created=True
              </span>
            </div>

            <div className="space-y-3">
              {learnDrafts.map((draft) => (
                <div
                  key={draft.draft_id}
                  className="p-4 rounded-xl bg-surface-card border border-hairline text-xs space-y-2"
                >
                  <div className="flex items-center justify-between">
                    <span className="font-mono font-bold text-ink text-sm">{draft.name}</span>
                    <span className="text-muted font-mono text-[11px]">
                      Created: {draft.created_at}
                    </span>
                  </div>
                  <p className="text-body leading-relaxed">{draft.description}</p>
                  <div className="p-3 bg-surface-dark text-on-dark font-mono rounded-lg text-xs max-h-32 overflow-y-auto whitespace-pre-wrap">
                    {draft.skill_md_content}
                  </div>
                  <div className="flex items-center justify-end space-x-2 pt-2">
                    <button
                      type="button"
                      onClick={() => handleRejectDraft(draft.draft_id)}
                      className="px-3 py-1.5 rounded bg-canvas border border-hairline hover:bg-surface-soft text-body text-xs font-medium cursor-pointer transition-colors"
                    >
                      Reject
                    </button>
                    <button
                      type="button"
                      onClick={() => handleApproveDraft(draft.draft_id)}
                      className="px-3 py-1.5 rounded bg-primary text-white text-xs font-medium hover:bg-primary-active transition-colors cursor-pointer"
                    >
                      Approve &amp; Add to Skills
                    </button>
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
