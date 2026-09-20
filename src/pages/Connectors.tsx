import { useState, useEffect } from 'react';
import {
  mcpList,
  mcpEnable,
  mcpConfigure,
  mcpConnect,
  mcpDisconnect,
  mcpTools,
  type McpServerInfo,
  type McpToolInfo,
} from '../lib/skills';

export function Connectors() {
  // Phase 2: start empty; servers/tools load from the desktop. Never seed
  // mock connectors as live data. Failures surface in loadError, not fakes.
  // Phase 9: servers are Hermes-configured (config.yaml mcp_servers); the
  // list is empty until one is connected — no phantom entries.
  const [servers, setServers] = useState<McpServerInfo[]>([]);
  const [tools, setTools] = useState<McpToolInfo[]>([]);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [configuringServer, setConfiguringServer] = useState<McpServerInfo | null>(null);
  const [addingNew, setAddingNew] = useState(false);
  const [configForm, setConfigForm] = useState<{
    id: string;
    transport: 'stdio' | 'sse';
    command: string;
    args: string;
    url: string;
    apiKey: string;
  }>({
    id: '',
    transport: 'stdio',
    command: '',
    args: '',
    url: '',
    apiKey: '',
  });
  const [saving, setSaving] = useState(false);
  const [toastMsg, setToastMsg] = useState<string | null>(null);

  useEffect(() => {
    loadData();
  }, []);

  const showToast = (msg: string) => {
    setToastMsg(msg);
    setTimeout(() => setToastMsg(null), 3500);
  };

  const loadData = async () => {
    try {
      setLoadError(null);
      const serverList = await mcpList();
      setServers(serverList);
      const toolList = await mcpTools();
      setTools(toolList);
    } catch (err) {
      console.error('Failed to load MCP servers:', err);
      setLoadError(`MCP connectors unavailable: ${err}`);
      setServers([]);
      setTools([]);
    }
  };

  const handleToggle = async (serverId: string, currentEnabled: boolean) => {
    const nextState = !currentEnabled;
    try {
      const ok = await mcpEnable(serverId, nextState);
      if (ok) {
        setServers((prev) =>
          prev.map((s) => (s.id === serverId ? { ...s, enabled: nextState } : s))
        );
        showToast(`${nextState ? 'Enabled' : 'Disabled'} connector "${serverId}"`);
        const updatedTools = await mcpTools();
        setTools(updatedTools);
      }
    } catch (err) {
      showToast(`Error: failed to update connector (${err})`);
    }
  };

  const openConfig = (server: McpServerInfo) => {
    setConfiguringServer(server);
    setAddingNew(false);
    setConfigForm({
      id: server.id,
      transport: server.transport === 'sse' ? 'sse' : 'stdio',
      command: server.command || '',
      args: (server.args || []).join(' '),
      url: server.url || '',
      apiKey: '',
    });
  };

  const openAddNew = () => {
    setConfiguringServer(null);
    setAddingNew(true);
    setConfigForm({ id: '', transport: 'stdio', command: '', args: '', url: '', apiKey: '' });
  };

  const handleSaveConfig = async () => {
    // Phase 9: Save = real Hermes connect (upsert + discover). An optional
    // API key is stored separately (server's own ${VAR} slot, .env only).
    const id = (addingNew ? configForm.id : configuringServer?.id || '').trim();
    if (!id) {
      showToast('Error: server id is required.');
      return;
    }
    setSaving(true);
    try {
      await mcpConnect(id, {
        transport: configForm.transport,
        command: configForm.command.trim() || undefined,
        args: configForm.args.split(' ').filter(Boolean),
        url: configForm.url.trim() || undefined,
      });
      if (configForm.apiKey.trim()) {
        await mcpConfigure(id, configForm.apiKey.trim());
      }
      showToast(`Connected MCP server "${id}"`);
      setConfiguringServer(null);
      setAddingNew(false);
      loadData();
    } catch (err) {
      showToast(`Error: failed to connect server (${err})`);
    } finally {
      setSaving(false);
    }
  };

  const handleDisconnect = async (serverId: string, serverName: string) => {
    if (!window.confirm(`Disconnect MCP server "${serverName}"? It stays configured but disabled.`)) return;
    try {
      await mcpDisconnect(serverId);
      showToast(`Disconnected "${serverId}"`);
      loadData();
    } catch (err) {
      showToast(`Error: failed to disconnect (${err})`);
    }
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
          <h1 className="font-display text-[22px] font-medium text-ink leading-tight">MCP Connectors</h1>
          <p className="text-xs text-muted">
            Model Context Protocol servers &middot; Stdio &amp; SSE Transports &middot; Safe Env Allowlist &middot; Namespace Prefixing
          </p>
        </div>
        <div className="text-xs px-3 py-1.5 rounded-md bg-canvas border border-hairline font-mono text-muted">
          Active tools: {tools.length}
        </div>
      </div>

      <div className="max-w-[1100px] w-full mx-auto p-6 space-y-8">
        {/* Load error — explicit, never fake servers */}
        {loadError && (
          <div className="p-4 rounded-xl bg-error/10 border border-error/30 text-xs text-body" role="alert">
            <span className="font-semibold text-error">Unavailable: </span>
            {loadError}
          </div>
        )}
        {/* Informational Invariant Banner */}
        <div className="p-4 rounded-xl bg-surface-soft border border-hairline flex items-start space-x-3 text-xs text-body">
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="text-primary flex-shrink-0 mt-0.5">
            <circle cx="12" cy="12" r="10" />
            <line x1="12" y1="16" x2="12" y2="12" />
            <line x1="12" y1="8" x2="12.01" y2="8" />
          </svg>
          <div className="space-y-1">
            <div className="font-semibold text-ink">
              Namespace &amp; Tool Filtering Security Model
            </div>
            <p className="leading-relaxed">
              Discovered tools are exposed under runtime prefix <code className="font-mono bg-canvas px-1 py-0.5 rounded border border-hairline">mcp_&lt;server&gt;_&lt;tool&gt;</code> with numeric collision resolution. Whitelist filters (<code className="font-mono text-[11px]">tools.include</code>) strictly take precedence over exclude filters. OAuth tokens reside in <code className="font-mono text-[11px]">~/.pain-ai/mcp-tokens/</code> (mode 0600).
            </p>
          </div>
        </div>

        {/* Server Cards Grid */}
        <div>
          <div className="flex items-center justify-between mb-4">
            <h2 className="text-xs font-semibold uppercase tracking-wider text-muted">
              Available &amp; Configured Connectors ({servers.length})
            </h2>
            <button
              type="button"
              onClick={openAddNew}
              className="px-3 py-1.5 rounded-md bg-primary hover:bg-primary-active text-white font-sans text-[12px] font-medium transition-colors cursor-pointer"
              title="Connect a new MCP server through Hermes"
            >
              + Connect Server
            </button>
          </div>
          {servers.length === 0 && !loadError && (
            <p className="mb-4 p-4 rounded-xl bg-surface-soft border border-hairline text-xs text-muted">
              No MCP servers configured. Hermes provides the MCP client — connect a server to discover its tools live. Nothing is shown until a real server is connected.
            </p>
          )}
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            {servers.map((server) => {
              const letter = server.name.charAt(0).toUpperCase();
              return (
                <div
                  key={server.id}
                  className="p-5 rounded-xl bg-surface-soft/80 border border-hairline flex flex-col justify-between hover:border-hairline-strong transition-all shadow-2xs"
                >
                  <div>
                    {/* Header with avatar & toggle */}
                    <div className="flex items-center justify-between mb-3">
                      <div className="flex items-center space-x-3">
                        <div className="w-9 h-9 rounded-lg bg-surface-cream-strong flex items-center justify-center font-mono font-bold text-ink border border-hairline">
                          {letter}
                        </div>
                        <div>
                          <div className="font-semibold text-ink text-sm leading-tight flex items-center space-x-1.5">
                            <span>{server.name}</span>
                          </div>
                          <div className="flex items-center space-x-1.5 mt-0.5">
                            <span className="text-[10px] font-mono uppercase tracking-wider px-1.5 py-0.2 rounded bg-canvas border border-hairline text-muted">
                              {server.transport}
                            </span>
                            {server.enabled ? (
                              <span className="flex items-center space-x-1 text-[11px] text-emerald-700 font-medium">
                                <span className="w-1.5 h-1.5 rounded-full bg-emerald-500" />
                                <span>Active</span>
                              </span>
                            ) : (
                              <span className="flex items-center space-x-1 text-[11px] text-muted">
                                <span className="w-1.5 h-1.5 rounded-full bg-muted-soft" />
                                <span>Disabled</span>
                              </span>
                            )}
                          </div>
                        </div>
                      </div>

                      {/* Enable / Disable Switch */}
                      <button
                        type="button"
                        onClick={() => handleToggle(server.id, server.enabled)}
                        className={`w-11 h-6 flex items-center rounded-full p-1 cursor-pointer transition-colors ${
                          server.enabled ? 'bg-primary' : 'bg-hairline'
                        }`}
                        aria-label={`Toggle ${server.name}`}
                      >
                        <div
                          className={`bg-white w-4 h-4 rounded-full shadow-md transform transition-transform ${
                            server.enabled ? 'translate-x-5' : 'translate-x-0'
                          }`}
                        />
                      </button>
                    </div>

                    <p className="text-xs text-body line-clamp-2 leading-relaxed mb-3">
                      {server.description}
                    </p>
                  </div>

                  {/* Footer actions */}
                  <div className="pt-3 border-t border-hairline flex items-center justify-between text-xs">
                    <span className="text-[11px] font-mono text-muted" title={server.status}>
                      status: {server.status || (server.enabled ? 'enabled' : 'disabled')}
                    </span>

                    <div className="flex items-center space-x-1.5">
                      <button
                        type="button"
                        onClick={() => openConfig(server)}
                        className="px-2.5 py-1 rounded bg-canvas border border-hairline hover:border-muted font-medium text-body cursor-pointer transition-colors text-[11px]"
                      >
                        Configure &rarr;
                      </button>
                      <button
                        type="button"
                        onClick={() => handleDisconnect(server.id, server.name)}
                        className="px-2.5 py-1 rounded text-error border border-error/30 hover:bg-error/10 font-medium cursor-pointer transition-colors text-[11px]"
                        title="Shut the server down via Hermes (stays configured but disabled)"
                      >
                        Disconnect
                      </button>
                    </div>
                  </div>
                </div>
              );
            })}
          </div>
        </div>

        {/* Configuration Modal (Phase 9: real Hermes connect) */}
        {(configuringServer || addingNew) && (
          <div className="fixed inset-0 z-50 bg-black/40 flex items-center justify-center p-4">
            <div className="bg-canvas border border-hairline rounded-xl max-w-[540px] w-full p-6 shadow-2xl space-y-4 animate-in fade-in duration-150">
              <div className="flex items-center justify-between pb-3 border-b border-hairline">
                <div>
                  <h3 className="font-semibold text-ink text-base">
                    {addingNew ? 'Connect MCP Server' : `Configure ${configuringServer?.name}`}
                  </h3>
                  <p className="text-xs text-muted">
                    {addingNew
                      ? 'Registers with Hermes and discovers tools live'
                      : `Transport: ${configuringServer?.transport.toUpperCase()} · ID: ${configuringServer?.id}`}
                  </p>
                </div>
                <button
                  type="button"
                  onClick={() => { setConfiguringServer(null); setAddingNew(false); }}
                  className="text-muted hover:text-ink cursor-pointer p-1"
                >
                  &times;
                </button>
              </div>

              <div className="space-y-3 text-xs">
                {addingNew && (
                  <>
                    <div>
                      <label className="font-medium text-ink block mb-1">Server ID</label>
                      <input
                        type="text"
                        value={configForm.id}
                        onChange={(e) => setConfigForm({ ...configForm, id: e.target.value })}
                        placeholder="my-server (letters, digits, -, _)"
                        className="w-full h-8 px-3 rounded bg-surface-soft border border-hairline font-mono text-xs text-ink focus:outline-none focus:border-primary"
                      />
                    </div>
                    <div>
                      <label className="font-medium text-ink block mb-1">Transport</label>
                      <select
                        value={configForm.transport}
                        onChange={(e) => setConfigForm({ ...configForm, transport: e.target.value as 'stdio' | 'sse' })}
                        className="w-full h-8 px-3 rounded bg-surface-soft border border-hairline font-mono text-xs text-ink focus:outline-none focus:border-primary cursor-pointer"
                      >
                        <option value="stdio">stdio (local command)</option>
                        <option value="sse">SSE (remote URL)</option>
                      </select>
                    </div>
                  </>
                )}
                {configForm.transport === 'stdio' ? (
                  <>
                    <div>
                      <label className="font-medium text-ink block mb-1">Executable Command</label>
                      <input
                        type="text"
                        value={configForm.command}
                        onChange={(e) => setConfigForm({ ...configForm, command: e.target.value })}
                        placeholder="python or /path/to/binary"
                        className="w-full h-8 px-3 rounded bg-surface-soft border border-hairline font-mono text-xs text-ink focus:outline-none focus:border-primary"
                      />
                    </div>
                    <div>
                      <label className="font-medium text-ink block mb-1">Arguments</label>
                      <input
                        type="text"
                        value={configForm.args}
                        onChange={(e) => setConfigForm({ ...configForm, args: e.target.value })}
                        placeholder="server.py --stdio"
                        className="w-full h-8 px-3 rounded bg-surface-soft border border-hairline font-mono text-xs text-ink focus:outline-none focus:border-primary"
                      />
                    </div>
                  </>
                ) : (
                  <div>
                    <label className="font-medium text-ink block mb-1">SSE Endpoint URL</label>
                    <input
                      type="text"
                      value={configForm.url}
                      onChange={(e) => setConfigForm({ ...configForm, url: e.target.value })}
                      placeholder="https://..."
                      className="w-full h-8 px-3 rounded bg-surface-soft border border-hairline font-mono text-xs text-ink focus:outline-none focus:border-primary"
                    />
                  </div>
                )}

                <div className="pt-2 border-t border-hairline">
                  <label className="font-medium text-ink block mb-1">API Key <span className="text-muted font-normal">(optional, server&apos;s own key slot)</span></label>
                  <input
                    type="password"
                    value={configForm.apiKey}
                    onChange={(e) => setConfigForm({ ...configForm, apiKey: e.target.value })}
                    placeholder="Stored in HERMES_HOME/.env (0600), never in JSON"
                    className="w-full h-8 px-3 rounded bg-surface-soft border border-hairline font-mono text-xs text-ink focus:outline-none focus:border-primary"
                  />
                </div>
              </div>

              <div className="pt-3 border-t border-hairline flex items-center justify-end space-x-2">
                <button
                  type="button"
                  onClick={() => { setConfiguringServer(null); setAddingNew(false); }}
                  className="px-3 py-1.5 rounded text-xs font-medium text-body hover:bg-surface-soft cursor-pointer"
                >
                  Cancel
                </button>
                <button
                  type="button"
                  onClick={handleSaveConfig}
                  disabled={saving}
                  className="px-3 py-1.5 rounded bg-primary text-white text-xs font-medium hover:bg-primary-active transition-colors cursor-pointer disabled:opacity-60"
                >
                  {saving ? 'Connecting…' : addingNew ? 'Connect Server' : 'Reconnect Server'}
                </button>
              </div>
            </div>
          </div>
        )}

        {/* Active Discovered Tools Panel */}
        <div className="p-5 rounded-xl bg-surface-soft border border-hairline">
          <div className="flex items-center justify-between mb-4">
            <div>
              <h3 className="font-semibold text-ink text-sm">Discovered MCP Tools</h3>
              <p className="text-xs text-muted">
                Namespaced tools currently exposed to the pain-ai reasoning engine
              </p>
            </div>
            <span className="font-mono text-xs px-2 py-0.5 rounded bg-canvas border border-hairline text-muted">
              {tools.length} tools registered
            </span>
          </div>

          <div className="space-y-2">
            {tools.map((t) => (
              <div
                key={t.name}
                className="p-3 rounded-lg bg-canvas border border-hairline flex items-start justify-between text-xs"
              >
                <div>
                  <div className="flex items-center space-x-2">
                    <span className="font-mono font-semibold text-primary">{t.name}</span>
                    <span className="text-[10px] font-mono px-1.5 py-0.2 rounded bg-surface-soft text-muted">
                      server: {t.server_id}
                    </span>
                  </div>
                  <p className="text-body mt-1">{t.description}</p>
                </div>
                <span className="text-[11px] font-mono text-muted bg-surface-soft px-2 py-0.5 rounded border border-hairline-soft">
                  callable
                </span>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}
