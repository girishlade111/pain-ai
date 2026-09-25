use serde::{Deserialize, Serialize};
use std::fs;
// SSOT (Phase 1): Provider credential storage (OS keychain) + persistent
// configuration (providers.json) owner is this file. Sidecar receives
// LSC_PROVIDER/BASE_URL/MODEL/API_KEY env copies per spawn; frontend
// SettingsProviders/ModelPicker are views only.
use std::path::PathBuf;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderDef {
    pub id: String,
    pub label: String,
    #[serde(rename = "baseURL")]
    pub base_url: String,
    pub auth: String,
    #[serde(rename = "modelsHint")]
    pub models_hint: Vec<String>,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveConfig {
    pub active: String,
    pub fallbacks: Vec<String>,
    #[serde(rename = "targetModel")]
    pub target_model: String,
    /// Phase 3: per-provider endpoint/model overrides. Absent for fresh installs
    /// and old configs (serde default) — static defaults apply.
    #[serde(default)]
    pub overrides: std::collections::HashMap<String, ProviderOverride>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderOverride {
    #[serde(rename = "baseUrl", skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

/// Effective (resolved) provider view returned to the UI and the sidecar.
/// base_url/model = override if set, else the static default.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffectiveProvider {
    pub id: String,
    pub label: String,
    #[serde(rename = "baseURL")]
    pub base_url: String,
    pub auth: String,
    pub model: String,
    #[serde(rename = "modelsHint")]
    pub models_hint: Vec<String>,
    pub description: String,
    pub customized: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingResult {
    pub ok: bool,
    #[serde(rename = "latencyMs")]
    pub latency_ms: Option<u64>,
    pub error: Option<String>,
}

pub fn get_static_providers() -> Vec<ProviderDef> {
    vec![
        ProviderDef {
            id: "openai".into(),
            label: "OpenAI".into(),
            base_url: "https://api.openai.com/v1".into(),
            auth: "key".into(),
            models_hint: vec!["gpt-4o".into(), "gpt-4o-mini".into(), "o1".into(), "o3-mini".into()],
            description: "Direct OpenAI API access with standard GPT-4o & reasoning models.".into(),
        },
        ProviderDef {
            id: "anthropic".into(),
            label: "Anthropic".into(),
            base_url: "https://api.anthropic.com/v1".into(),
            auth: "key".into(),
            models_hint: vec!["claude-3-7-sonnet".into(), "claude-3-5-sonnet".into(), "claude-3-5-haiku".into()],
            description: "Claude 3.7 Sonnet hybrid reasoning and standard Claude models.".into(),
        },
        ProviderDef {
            id: "gemini".into(),
            label: "Google Gemini".into(),
            base_url: "https://generativelanguage.googleapis.com/v1beta/openai/".into(),
            auth: "key".into(),
            models_hint: vec!["gemini-2.5-pro".into(), "gemini-2.5-flash".into(), "gemini-2.0-flash".into()],
            description: "Google AI Studio Gemini OpenAI-compatible API endpoint.".into(),
        },
        ProviderDef {
            id: "openrouter".into(),
            label: "OpenRouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            auth: "key".into(),
            models_hint: vec!["anthropic/claude-3.7-sonnet".into(), "openai/gpt-4o".into(), "deepseek/deepseek-r1".into()],
            description: "Unified multi-provider routing gateway with fallback capabilities.".into(),
        },
        ProviderDef {
            id: "ollama".into(),
            label: "Ollama (Local)".into(),
            base_url: "http://localhost:11434/v1".into(),
            auth: "none".into(),
            models_hint: vec!["llama3.3".into(), "qwen2.5-coder".into(), "deepseek-r1:8b".into(), "mistral".into()],
            description: "Locally served open-weight models running on your machine.".into(),
        },
        ProviderDef {
            id: "lmstudio".into(),
            label: "LM Studio (Local)".into(),
            base_url: "http://localhost:1234/v1".into(),
            auth: "none".into(),
            models_hint: vec!["local-model".into()],
            description: "Local OpenAI-compatible inference server provided by LM Studio.".into(),
        },
        ProviderDef {
            id: "custom".into(),
            label: "Custom OpenAI-Compat".into(),
            base_url: "http://localhost:8000/v1".into(),
            auth: "none".into(),
            models_hint: vec!["default".into()],
            description: "User-defined OpenAI-compatible API server endpoint.".into(),
        },
        ProviderDef {
            id: "deepseek".into(),
            label: "DeepSeek".into(),
            base_url: "https://api.deepseek.com/v1".into(),
            auth: "key".into(),
            models_hint: vec!["deepseek-chat".into(), "deepseek-reasoner".into()],
            description: "DeepSeek V3 and R1 reasoning API.".into(),
        },
        ProviderDef {
            id: "xai".into(),
            label: "xAI Grok".into(),
            base_url: "https://api.x.ai/v1".into(),
            auth: "key".into(),
            models_hint: vec!["grok-2-1212".into(), "grok-2-vision-1212".into()],
            description: "xAI API access for Grok-2 models.".into(),
        },
        ProviderDef {
            id: "qwen".into(),
            label: "Qwen Cloud".into(),
            base_url: "https://dashscope-intl.aliyuncs.com/compatible-mode/v1".into(),
            auth: "key".into(),
            models_hint: vec!["qwen-max".into(), "qwen-plus".into(), "qwen-turbo".into()],
            description: "Alibaba Cloud DashScope international compatible API endpoint.".into(),
        },
        ProviderDef {
            id: "minimax".into(),
            label: "MiniMax".into(),
            base_url: "https://api.minimax.io/anthropic".into(),
            auth: "key".into(),
            models_hint: vec!["MiniMax-Text-01".into()],
            description: "MiniMax Anthropic-compatible API endpoint.".into(),
        },
        ProviderDef {
            id: "nous".into(),
            label: "Nous Portal".into(),
            base_url: "https://inference.nousresearch.com/v1".into(),
            auth: "oauth-pending".into(),
            models_hint: vec!["hermes-3-llama-3.1-405b".into()],
            description: "Nous Research Portal OAuth authentication (coming in v2).".into(),
        },
        ProviderDef {
            id: "openai-codex".into(),
            label: "OpenAI Codex".into(),
            base_url: "https://chatgpt.com/backend-api".into(),
            auth: "oauth-pending".into(),
            models_hint: vec!["codex-beta".into()],
            description: "ChatGPT backend OAuth authentication (coming in v2).".into(),
        },
        ProviderDef {
            id: "xai-oauth".into(),
            label: "xAI Grok OAuth".into(),
            base_url: "https://api.x.ai/v1".into(),
            auth: "oauth-pending".into(),
            models_hint: vec!["grok-oauth".into()],
            description: "SuperGrok / Premium+ OAuth authentication (coming in v2).".into(),
        },
    ]
}

fn config_path() -> PathBuf {
    let base = std::env::var("LOCALAPPDATA")
        .or_else(|_| std::env::var("APPDATA"))
        .unwrap_or_else(|_| ".".into());
    let mut p = PathBuf::from(base);
    p.push("pain-ai");
    let _ = fs::create_dir_all(&p);
    p.push("providers.json");
    p
}

pub(crate) fn load_config() -> ActiveConfig {
    load_config_from(&config_path())
}

pub(crate) fn load_config_from(path: &std::path::Path) -> ActiveConfig {
    if let Ok(data) = fs::read_to_string(path) {
        if let Ok(cfg) = serde_json::from_str::<ActiveConfig>(&data) {
            return cfg;
        }
    }
    ActiveConfig {
        active: "openai".into(),
        fallbacks: vec!["openrouter".into(), "ollama".into()],
        target_model: "gpt-4o".into(),
        overrides: Default::default(),
    }
}

fn save_config(cfg: &ActiveConfig) -> Result<(), String> {
    save_config_to(cfg, &config_path())
}

fn save_config_to(cfg: &ActiveConfig, path: &std::path::Path) -> Result<(), String> {
    // SECURITY: ActiveConfig must never carry key material. This assert runs on
    // every write so a future field cannot silently persist a secret to disk.
    let json = serde_json::to_string_pretty(cfg).map_err(|e| e.to_string())?;
    // SECURITY (P13): always-on in every profile (debug_assert compiles out
    // of release builds) so a future field cannot silently persist a secret.
    if let Some(marker) = find_secret_marker(&json) {
        return Err(format!(
            "refusing to persist provider config containing key material ({}…)",
            marker
        ));
    }
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    fs::write(path, json).map_err(|e| e.to_string())?;
    Ok(())
}

/// Best-effort secret sniff over serialized config. Never logs the value.
fn find_secret_marker(json: &str) -> Option<&'static str> {
    for marker in [
        "sk-",
        "ghp_",
        "gho_",
        "github_pat_",
        "xoxb-",
        "xoxa-",
        "xoxp-",
        "xoxo-",
        "AIza",
        "PRIVATE KEY",
    ] {
        if json.contains(marker) {
            return Some(marker);
        }
    }
    None
}

/// Validate a provider base URL without new dependencies.
/// Rules: non-empty; scheme http/https only; host required; plain http only
/// for loopback or RFC1918 private hosts (covers localhost:11434, LAN Ollama);
/// public http and all other schemes rejected.
pub fn validate_base_url(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("Endpoint URL must not be empty".into());
    }
    let lower = trimmed.to_lowercase();
    let is_https = lower.starts_with("https://");
    let is_http = lower.starts_with("http://");
    if !is_https && !is_http {
        return Err(format!(
            "Unsupported endpoint protocol in '{}': only https:// and http:// are supported",
            redact_url(raw)
        ));
    }
    let scheme_len = if is_https { "https://".len() } else { "http://".len() };
    let rest = &trimmed[scheme_len..];
    // Strip path/query/fragment → authority → host (drop optional port).
    let authority = rest
        .split(['/', '?', '#'])
        .next()
        .unwrap_or("");
    // SECURITY (P13): embedded credentials (user:pass@host) would ride along
    // on every request AND risk landing in diagnostics. Reject outright —
    // key material belongs in the OS keychain, never in URLs.
    if authority.contains('@') {
        return Err("Endpoint URL must not embed credentials (user:pass@host); store secrets in the OS keychain".into());
    }
    let host_port = authority;
    // Drop an optional :port suffix (exactly one colon + numeric port).
    // IPv6 literals ([::1]) keep their brackets until normalization below.
    let host_raw = if host_port.starts_with('[') {
        host_port.split(']').next().unwrap_or(host_port)
    } else if host_port.matches(':').count() == 1 {
        let mut split = host_port.split(':');
        let h = split.next().unwrap_or("");
        let port = split.next().unwrap_or("");
        if port.is_empty() || port.parse::<u16>().is_ok() {
            h
        } else {
            host_port
        }
    } else {
        host_port
    };
    // Normalize IPv6 brackets.
    let host = host_raw.trim_matches(|c| c == '[' || c == ']').to_lowercase();
    if host.is_empty() {
        return Err("Endpoint URL must include a host".into());
    }
    if !is_https && !is_local_http_host(&host) {
        return Err(format!(
            "Plain http:// is only allowed for local hosts (localhost or private LAN); use https:// for '{}'",
            redact_url(raw)
        ));
    }
    // Normalized: trimmed, no trailing slashes.
    Ok(trimmed.trim_end_matches('/').to_string())
}

fn is_local_http_host(host: &str) -> bool {
    if host == "localhost" || host == "127.0.0.1" || host == "::1" {
        return true;
    }
    // RFC1918 private IPv4 ranges.
    let parts: Vec<&str> = host.split('.').collect();
    if parts.len() == 4 {
        if let (Ok(a), Ok(b)) = (parts[0].parse::<u8>(), parts[1].parse::<u8>()) {
            if a == 10 {
                return true;
            }
            if a == 192 && b == 168 {
                return true;
            }
            if a == 172 && (16..=31).contains(&b) {
                return true;
            }
        }
    }
    false
}

/// Redact an endpoint for error messages (drops any embedded userinfo).
fn redact_url(raw: &str) -> String {
    match raw.split_once("://") {
        Some((scheme, rest)) => {
            let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
            format!("{}://{}", scheme, authority)
        }
        None => "[unparseable-url]".to_string(),
    }
}

fn find_static(id: &str) -> Option<ProviderDef> {
    get_static_providers().into_iter().find(|p| p.id == id)
}

/// Resolve the effective base URL + model for a provider id.
pub fn effective_provider(id: &str, cfg: &ActiveConfig) -> Result<EffectiveProvider, String> {
    let def = find_static(id)
        .ok_or_else(|| format!("Unknown provider '{}'", id))?;
    let ov = cfg.overrides.get(id);
    let base_url = ov
        .and_then(|o| o.base_url.clone())
        .unwrap_or(def.base_url.clone());
    let model = ov
        .and_then(|o| o.model.clone())
        .unwrap_or_else(|| {
            if cfg.active == id && !cfg.target_model.trim().is_empty() {
                cfg.target_model.clone()
            } else {
                def.models_hint.first().cloned().unwrap_or_default()
            }
        });
    let customized = ov.map(|o| o.base_url.is_some() || o.model.is_some()).unwrap_or(false);
    Ok(EffectiveProvider {
        id: def.id,
        label: def.label,
        base_url,
        auth: def.auth,
        model,
        models_hint: def.models_hint,
        description: def.description,
        customized,
    })
}

pub(crate) fn keyring_entry(provider_id: &str) -> Result<keyring::Entry, String> {
    let service = format!("in.ladestack.painai/{}", provider_id);
    keyring::Entry::new(&service, "api-key").map_err(|e| e.to_string())
}

#[cfg(test)]
#[path = "providers_tests.rs"]
mod providers_tests;

#[tauri::command]
pub fn provider_list() -> Vec<ProviderDef> {
    get_static_providers()
}

#[tauri::command]
pub fn provider_get_active() -> ActiveConfig {
    load_config()
}

#[tauri::command]
pub fn provider_set_active(id: String) -> Result<(), String> {
    if find_static(&id).is_none() {
        return Err(format!("Unknown provider '{}'", id));
    }
    let mut cfg = load_config();
    cfg.active = id.clone();
    // Default the shared target model from the static hint only when this
    // provider has no persisted model override (preserves custom models).
    if cfg.overrides.get(&id).and_then(|o| o.model.clone()).is_none() {
        let providers = get_static_providers();
        if let Some(p) = providers.iter().find(|p| p.id == id) {
            if let Some(first_model) = p.models_hint.first() {
                cfg.target_model = first_model.clone();
            }
        }
    } else if let Some(m) = cfg.overrides.get(&id).and_then(|o| o.model.clone()) {
        cfg.target_model = m;
    }
    save_config(&cfg)
}

#[tauri::command]
pub fn provider_set_target_model(target_model: String) -> Result<(), String> {
    let model = target_model.trim().to_string();
    if model.is_empty() {
        return Err("Model name must not be empty".into());
    }
    let mut cfg = load_config();
    cfg.target_model = model.clone();
    // Persist per-provider so the model survives provider switches + restarts.
    cfg.overrides
        .entry(cfg.active.clone())
        .or_default()
        .model = Some(model);
    save_config(&cfg)
}

/// Phase 3: persist a custom base URL for any provider (validates first —
/// invalid input never touches disk).
#[tauri::command]
pub fn provider_set_base_url(provider_id: String, base_url: String) -> Result<EffectiveProvider, String> {
    if find_static(&provider_id).is_none() {
        return Err(format!("Unknown provider '{}'", provider_id));
    }
    let normalized = validate_base_url(&base_url)?;
    let mut cfg = load_config();
    cfg.overrides
        .entry(provider_id.clone())
        .or_default()
        .base_url = Some(normalized);
    save_config(&cfg)?;
    effective_provider(&provider_id, &cfg)
}

/// Phase 3: drop the custom base URL override, restoring the static default.
#[tauri::command]
pub fn provider_reset_base_url(provider_id: String) -> Result<EffectiveProvider, String> {
    if find_static(&provider_id).is_none() {
        return Err(format!("Unknown provider '{}'", provider_id));
    }
    let mut cfg = load_config();
    if let Some(ov) = cfg.overrides.get_mut(&provider_id) {
        ov.base_url = None;
        if ov.model.is_none() {
            cfg.overrides.remove(&provider_id);
        }
    }
    save_config(&cfg)?;
    effective_provider(&provider_id, &cfg)
}

/// Phase 3: resolved view (static default + persisted overrides) for one provider.
#[tauri::command]
pub fn provider_get_effective(provider_id: String) -> Result<EffectiveProvider, String> {
    let cfg = load_config();
    effective_provider(&provider_id, &cfg)
}

/// Phase 3: resolved views for all providers (drives the Settings grid —
/// replaces React-only URL state).
#[tauri::command]
pub fn provider_list_effective() -> Vec<EffectiveProvider> {
    let cfg = load_config();
    get_static_providers()
        .iter()
        .filter_map(|p| effective_provider(&p.id, &cfg).ok())
        .collect()
}

#[tauri::command]
pub fn provider_set_fallbacks(fallbacks: Vec<String>) -> Result<(), String> {
    let mut cfg = load_config();
    cfg.fallbacks = fallbacks;
    save_config(&cfg)
}

#[tauri::command]
pub fn key_set(provider_id: String, secret: String) -> Result<(), String> {
    if secret.trim().is_empty() {
        return key_delete(provider_id);
    }
    let entry = keyring_entry(&provider_id)?;
    entry.set_password(&secret).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn key_status(provider_id: String) -> bool {
    if let Ok(entry) = keyring_entry(&provider_id) {
        entry.get_password().is_ok()
    } else {
        false
    }
}

#[tauri::command]
pub fn key_delete(provider_id: String) -> Result<(), String> {
    let entry = keyring_entry(&provider_id)?;
    let _ = entry.delete_credential();
    Ok(())
}

fn sanitize_error(err_str: &str) -> String {
    // Redact any potential tokens or secrets. Markers cover OpenAI-style,
    // GitHub, Slack, Google keys plus Bearer credential echoes.
    let mut out = err_str.to_string();
    for marker in [
        "sk-",
        "ghp_",
        "gho_",
        "github_pat_",
        "xoxb-",
        "xoxa-",
        "xoxp-",
        "xoxo-",
        "AIza",
        "Bearer ",
        "bearer ",
    ] {
        loop {
            let Some(start) = out.find(marker) else {
                break;
            };
            // Scan for the token end AFTER the marker (the marker itself may
            // end in a terminator, e.g. the space in "Bearer ").
            let scan_from = start + marker.len();
            let end = out[scan_from..]
                .find(|c: char| !(c.is_alphanumeric() || c == '_' || c == '-' || c == '.'))
                .map(|offset| scan_from + offset)
                .unwrap_or(out.len());
            // Never redact the whole message when nothing follows the marker.
            if end == scan_from {
                break;
            }
            // Scheme words ("Bearer ") are not secret — keep the label so
            // diagnostics stay readable while the credential is dropped.
            if marker.ends_with(' ') {
                out.replace_range(scan_from..end, "[REDACTED]");
            } else {
                out.replace_range(start..end, "[REDACTED]");
            }
        }
    }
    out
}

#[tauri::command]
pub async fn provider_ping(provider_id: String) -> PingResult {
    // Phase 3: ping the EFFECTIVE endpoint (custom overrides honored), so a
    // user-supplied base URL is actually exercised before use.
    let cfg = load_config();
    let provider = match effective_provider(&provider_id, &cfg) {
        Ok(p) => p,
        Err(e) => {
            return PingResult {
                ok: false,
                latency_ms: None,
                error: Some(e),
            }
        }
    };

    if provider.auth == "oauth-pending" {
        return PingResult {
            ok: false,
            latency_ms: None,
            error: Some("OAuth authentication available in v2".into()),
        };
    }

    let maybe_key = if provider.auth == "key" {
        match keyring_entry(&provider_id).and_then(|e| e.get_password().map_err(|e| e.to_string())) {
            Ok(k) => Some(k),
            Err(_) => {
                return PingResult {
                    ok: false,
                    latency_ms: None,
                    error: Some("API key not configured in keychain".into()),
                };
            }
        }
    } else {
        None
    };

    let base = provider.base_url.trim_end_matches('/');
    let url = format!("{}/models", base);

    let client_builder = reqwest::Client::builder().timeout(Duration::from_secs(10));
    let client = match client_builder.build() {
        Ok(c) => c,
        Err(e) => {
            return PingResult {
                ok: false,
                latency_ms: None,
                error: Some(sanitize_error(&e.to_string())),
            }
        }
    };

    let mut req = client.get(&url);
    if let Some(key) = &maybe_key {
        if provider_id == "anthropic" {
            req = req.header("x-api-key", key);
            req = req.header("anthropic-version", "2023-06-01");
        } else {
            req = req.header("Authorization", format!("Bearer {}", key));
        }
    }

    let start = Instant::now();
    match req.send().await {
        Ok(resp) => {
            let latency_ms = start.elapsed().as_millis() as u64;
            if resp.status().is_success() {
                PingResult {
                    ok: true,
                    latency_ms: Some(latency_ms),
                    error: None,
                }
            } else {
                let status = resp.status();
                PingResult {
                    ok: false,
                    latency_ms: Some(latency_ms),
                    error: Some(format!("HTTP {}: {}", status.as_u16(), status.canonical_reason().unwrap_or("Error"))),
                }
            }
        }
        Err(e) => {
            let latency_ms = start.elapsed().as_millis() as u64;
            let msg = if e.is_timeout() {
                "Connection timed out after 10s".into()
            } else if e.is_connect() {
                "Connection refused / host unreachable".into()
            } else {
                sanitize_error(&e.to_string())
            };
            PingResult {
                ok: false,
                latency_ms: Some(latency_ms),
                error: Some(msg),
            }
        }
    }
}
