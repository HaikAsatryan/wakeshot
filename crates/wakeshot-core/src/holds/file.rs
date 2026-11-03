//! Hold files: parsing them into `HoldFacts`, and the RFC 3339 form of `expires` (architecture section 6).
//!
//! Owner: sources track (`p4-hold-files`; fixes by `p4-live-claude`). The two signatures below are contract: the
//! core's Keep awake (Phase 2) formats `expires` with `format_expires`, so the core writes no date code of its own.

/// `expires` text (RFC 3339, UTC) to wall ms; `None` when it does not parse.
pub fn parse_expires(text: &str) -> Option<i64> {
    let _ = text;
    todo!("p4-hold-files")
}

/// Wall ms to `expires` text (RFC 3339, UTC), the form `parse_expires` reads back.
pub fn format_expires(wall_ms: i64) -> String {
    let _ = wall_ms;
    todo!("p4-hold-files")
}
