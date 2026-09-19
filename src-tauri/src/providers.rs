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
    let path = config_path();
    if let Ok(data) = fs::read_to_string(&path) {
        if let Ok(cfg) = serde_json::from_str::<ActiveConfig>(&data) {
            return cfg;
        }
    }
    ActiveConfig {
        active: "openai".into(),
        fallbacks: vec!["openrouter".into(), "ollama".into()],
        target_model: "gpt-4o".into(),
    }
}

fn save_config(cfg: &ActiveConfig) -> Result<(), String> {
    let path = config_path();
    let json = serde_json::to_string_pretty(cfg).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| e.to_string())?;
    Ok(())
}

pub(crate) fn keyring_entry(provider_id: &str) -> Result<keyring::Entry, String> {
    let service = format!("in.ladestack.painai/{}", provider_id);
    keyring::Entry::new(&service, "api-key").map_err(|e| e.to_string())
}

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
    let mut cfg = load_config();
    cfg.active = id.clone();
    let providers = get_static_providers();
    if let Some(p) = providers.iter().find(|p| p.id == id) {
        if let Some(first_model) = p.models_hint.first() {
            cfg.target_model = first_model.clone();
        }
    }
    save_config(&cfg)
}

#[tauri::command]
pub fn provider_set_target_model(target_model: String) -> Result<(), String> {
    let mut cfg = load_config();
    cfg.target_model = target_model;
    save_config(&cfg)
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
    // Redact any potential tokens or secrets
    let mut out = err_str.to_string();
    if let Some(start) = out.find("sk-") {
        let end = out[start..]
            .find(|c: char| !c.is_alphanumeric() && c != '_' && c != '-')
            .map(|offset| start + offset)
            .unwrap_or(out.len());
        out.replace_range(start..end, "[REDACTED_API_KEY]");
    }
    out
}

#[tauri::command]
pub async fn provider_ping(provider_id: String) -> PingResult {
    let providers = get_static_providers();
    let provider = match providers.into_iter().find(|p| p.id == provider_id) {
        Some(p) => p,
        None => {
            return PingResult {
                ok: false,
                latency_ms: None,
                error: Some(format!("Unknown provider '{}'", provider_id)),
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
