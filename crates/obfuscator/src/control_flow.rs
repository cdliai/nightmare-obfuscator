//! Control flow flattener (roadmap placeholder).
//!
//! This is an experimental, not-yet-implemented transform. Every method
//! currently returns the input unchanged. It is intentionally kept behind the
//! `flatten_control_flow` feature, which is disabled by default and emits a
//! warning when enabled, so callers are never led to believe control flow was
//! transformed. A real build-preserving implementation needs full AST rewriting
//! into match-based state machines and is tracked as future work.

use nightmare_core::{Language, Result};

pub struct ControlFlowFlattener {
    #[allow(dead_code)]
    intensity: u8,
}

impl ControlFlowFlattener {
    pub fn new(intensity: u8) -> Self {
        Self { intensity }
    }

    /// Returns the input unchanged. Not yet implemented; see module docs.
    pub fn flatten(&self, content: &str, _lang: Language) -> Result<String> {
        Ok(content.to_string())
    }
}
