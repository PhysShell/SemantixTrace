//! `trace-viewer` — TUI viewer for `SemantxTrace` traces.
//!
//! Deferred to post-v1.0; the placeholder exists to keep the workspace
//! shape stable. The TUI lands when a real use case justifies the
//! `ratatui` dependency footprint.

/// Placeholder symbol so the crate builds during S0.
#[must_use]
pub const fn placeholder() -> &'static str {
    "trace-viewer (S0 placeholder, deferred to post-v1.0)"
}

#[cfg(test)]
mod tests {
    #[test]
    fn returns_placeholder_string() {
        assert_eq!(
            super::placeholder(),
            "trace-viewer (S0 placeholder, deferred to post-v1.0)"
        );
    }
}
