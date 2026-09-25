//! Phase 3 — Provider endpoint/model override tests.
//! All persistence tests use an isolated temp file, never the real config.

use super::*;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_config_path(tag: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    std::env::temp_dir().join(format!("pain-ai-providers-test-{}-{}", tag, nonce))
}

#[test]
fn test_validate_https_ok_and_trims_slash() {
    assert_eq!(
        validate_base_url("https://example.com/v1/").unwrap(),
        "https://example.com/v1"
    );
    assert_eq!(
        validate_base_url("  https://api.openai.com/v1  ").unwrap(),
        "https://api.openai.com/v1"
    );
}

#[test]
fn test_validate_localhost_http_allowed() {
    assert_eq!(
        validate_base_url("http://localhost:11434/v1").unwrap(),
        "http://localhost:11434/v1"
    );
    assert_eq!(
        validate_base_url("http://127.0.0.1:11434/v1").unwrap(),
        "http://127.0.0.1:11434/v1"
    );
    assert_eq!(
        validate_base_url("http://[::1]:11434/v1").unwrap(),
        "http://[::1]:11434/v1"
    );
}

#[test]
fn test_validate_private_lan_http_allowed() {
    assert!(validate_base_url("http://192.168.1.50:11434/v1").is_ok());
    assert!(validate_base_url("http://10.0.0.5:8000/v1").is_ok());
    assert!(validate_base_url("http://172.16.5.4/v1").is_ok());
}

#[test]
fn test_validate_rejects_empty_and_bad_protocol() {
    assert!(validate_base_url("").is_err());
    assert!(validate_base_url("   ").is_err());
    assert!(validate_base_url("ftp://example.com/v1").is_err());
    assert!(validate_base_url("file:///etc/passwd").is_err());
    assert!(validate_base_url("example.com/v1").is_err());
    assert!(validate_base_url("://missing-scheme").is_err());
}

#[test]
fn test_validate_rejects_public_http() {
    assert!(validate_base_url("http://example.com/v1").is_err());
    assert!(validate_base_url("http://93.184.216.34/v1").is_err());
    assert!(validate_base_url("http://172.32.0.1/v1").is_err());
}

#[test]
fn test_validate_scheme_case_insensitive() {
    assert_eq!(
        validate_base_url("HTTPS://example.com/v1").unwrap(),
        "HTTPS://example.com/v1"
    );
}

#[test]
fn test_overrides_roundtrip_and_effective_resolution() {
    let path = temp_config_path("roundtrip");
    let mut cfg = load_config_from(&path);
    assert!(cfg.overrides.is_empty());

    // Simulate provider_set_base_url logic against the temp path.
    let normalized = validate_base_url("https://example.com/v1").unwrap();
    cfg.overrides
        .entry("custom".to_string())
        .or_default()
        .base_url = Some(normalized);
    cfg.overrides
        .entry("custom".to_string())
        .or_default()
        .model = Some("my-model".to_string());
    save_config_to(&cfg, &path).unwrap();

    let reloaded = load_config_from(&path);
    let eff = effective_provider("custom", &reloaded).unwrap();
    assert_eq!(eff.base_url, "https://example.com/v1");
    assert_eq!(eff.model, "my-model");
    assert!(eff.customized);

    // Static default untouched for other providers.
    let ollama = effective_provider("ollama", &reloaded).unwrap();
    assert_eq!(ollama.base_url, "http://localhost:11434/v1");
    assert!(!ollama.customized);

    let _ = fs::remove_file(&path);
}

#[test]
fn test_legacy_config_without_overrides_loads() {
    let path = temp_config_path("legacy");
    fs::write(
        &path,
        r#"{"active":"ollama","fallbacks":[],"targetModel":"llama3.3"}"#,
    )
    .unwrap();
    let cfg = load_config_from(&path);
    assert_eq!(cfg.active, "ollama");
    assert!(cfg.overrides.is_empty());
    let eff = effective_provider("ollama", &cfg).unwrap();
    assert_eq!(eff.model, "llama3.3");
    let _ = fs::remove_file(&path);
}

#[test]
fn test_p13_url_with_embedded_credentials_rejected() {
    // P13: user:pass@host would ride every request and risk diagnostics.
    assert!(validate_base_url("https://user:pass@example.com/v1").is_err());
    assert!(validate_base_url("http://token:sk-abc123@localhost:11434/v1").is_err());
    assert!(validate_base_url("https://example.com/v1").is_ok());
}

#[test]
fn test_p13_config_persist_refuses_key_material() {
    // P13: always-on (release builds included), not debug_assert-only.
    let path = temp_config_path("secret-refuse");
    let mut cfg = load_config_from(&path);
    cfg.target_model = "gpt-4o".into();
    cfg.overrides.entry("custom".into()).or_default().base_url =
        Some("https://example.com/v1".into());
    save_config_to(&cfg, &path).unwrap();
    // Smuggle key material through the model field: write must fail and no
    // file may be created.
    let evil_path = temp_config_path("secret-evil");
    let mut evil = load_config_from(&evil_path);
    evil.target_model = "sk-live-should-never-persist".into();
    assert!(save_config_to(&evil, &evil_path).is_err());
    assert!(!evil_path.exists());
    let _ = fs::remove_file(&path);
}

#[test]
fn test_p13_sanitize_error_redacts_families() {
    assert_eq!(sanitize_error("401 sk-abc123XYZ"), "401 [REDACTED]");
    assert!(sanitize_error("token ghp_abcdefghij1234 leaked").contains("[REDACTED]"));
    assert!(sanitize_error("auth Bearer mytoken1234567890 here").contains("Bearer [REDACTED]"));
    assert_eq!(sanitize_error("clean timeout error"), "clean timeout error");
}

#[test]
fn test_unknown_provider_rejected() {
    let cfg = load_config_from(&temp_config_path("unknown"));
    assert!(effective_provider("nope", &cfg).is_err());
    assert!(validate_base_url("https://example.com/v1").is_ok());
}

#[test]
fn test_invalid_url_never_touches_disk() {
    let path = temp_config_path("invalid");
    assert!(!path.exists());
    assert!(validate_base_url("ftp://example.com/v1").is_err());
    // No write happens on validation failure by construction.
    assert!(!path.exists());
}

#[test]
fn test_config_schema_carries_no_secret_fields() {
    let mut cfg = ActiveConfig {
        active: "custom".into(),
        fallbacks: vec![],
        target_model: "my-model".into(),
        overrides: Default::default(),
    };
    cfg.overrides.entry("custom".into()).or_default().base_url =
        Some("https://example.com/v1".into());
    let v: serde_json::Value = serde_json::to_value(&cfg).unwrap();
    let obj = v.as_object().unwrap();
    for key in obj.keys() {
        assert!(
            ["active", "fallbacks", "targetModel", "overrides"].contains(&key.as_str()),
            "unexpected config key '{}'",
            key
        );
    }
    let s = serde_json::to_string(&cfg).unwrap();
    for marker in ["sk-", "ghp_", "xoxb-", "AIza", "api_key", "secret", "token"] {
        assert!(!s.contains(marker), "config must not contain '{}'", marker);
    }
}
