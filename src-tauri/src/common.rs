//! pain ai — Shared Desktop DTOs (common.rs)
//!
//! SINGLE SOURCE OF TRUTH for desktop UI envelope types.
//! Owners:
//! - RectDto / UiResult: Rust desktop boundary (used by capture, context, uia, atspi).
//! - Agent execution / memory / tools: Hermes Agent (vendored, do not duplicate here).
//! - Sidecar transport structs: sidecar/*.py (thin HTTP adapters over Hermes + Rust).
//!
//! Formerly duplicated in capture.rs, context.rs, uia.rs, atspi.rs with
//! identical JSON shapes. All consumers now import from here; per-module
//! `pub use crate::common::{RectDto, UiResult}` re-exports preserve
//! `super::*` test imports.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RectDto {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl RectDto {
    pub fn center(&self) -> (i32, i32) {
        (self.x + self.w / 2, self.y + self.h / 2)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiResult<T> {
    pub ok: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub code: Option<String>,
    pub hint: Option<String>,
    pub hit_type: Option<String>, // "tree" | "fallback" | "capture"
}

#[allow(dead_code)]
pub fn to_physical(rect: &RectDto, scale_factor: f64) -> RectDto {
    RectDto {
        x: (rect.x as f64 * scale_factor).round() as i32,
        y: (rect.y as f64 * scale_factor).round() as i32,
        w: (rect.w as f64 * scale_factor).round() as i32,
        h: (rect.h as f64 * scale_factor).round() as i32,
    }
}
