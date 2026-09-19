//! pain ai — Unit Tests for Memory, Cron, and Subagents IPC (memory_cron_tests.rs)

use super::*;

#[test]
fn test_memory_character_limits_constants() {
    assert_eq!(MEMORY_CHAR_LIMIT, 2200);
    assert_eq!(USER_CHAR_LIMIT, 1375);
}

#[test]
fn test_platform_delivery_strictly_rejected() {
    // In-app delivery variants are permitted
    assert!(validate_delivery("in_app").is_ok());
    assert!(validate_delivery("in_app ").is_ok());
    assert!(validate_delivery("app").is_ok());
    assert!(validate_delivery("ui").is_ok());

    // External platforms are strictly rejected
    let tg = validate_delivery("telegram");
    assert!(tg.is_err());
    assert!(tg.unwrap_err().contains("strictly disabled in pain ai desktop v1"));

    let discord = validate_delivery("discord");
    assert!(discord.is_err());

    let slack = validate_delivery("slack");
    assert!(slack.is_err());
}

#[test]
fn test_subagent_max_parallel_clamping() {
    assert_eq!(clamp_max_parallel(0), 1);
    assert_eq!(clamp_max_parallel(1), 1);
    assert_eq!(clamp_max_parallel(2), 2);
    assert_eq!(clamp_max_parallel(3), 3);
    assert_eq!(clamp_max_parallel(4), 3);
    assert_eq!(clamp_max_parallel(10), 3);
    assert_eq!(clamp_max_parallel(100), 3);
}

#[test]
fn test_fts_query_sanitizer_removes_special_characters() {
    let raw = "rm -rf / + {}: dangerous^query? [123]";
    let sanitized = sanitize_fts_query(raw);
    assert!(!sanitized.contains('{'));
    assert!(!sanitized.contains('}'));
    assert!(!sanitized.contains('+'));
    assert!(!sanitized.contains('^'));
    assert!(!sanitized.contains('?'));
    assert!(!sanitized.contains('['));
    assert!(!sanitized.contains(']'));
    assert!(sanitized.contains("\"rm\"*"));
    assert!(sanitized.contains("\"dangerous\"*"));
}
