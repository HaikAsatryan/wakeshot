//! Read-only SQLite queries for `WantedFile::Query` (architecture section 7), through Windows' own `winsqlite3.dll`
//! loaded at run time (`LoadLibraryExW` with `LOAD_LIBRARY_SEARCH_SYSTEM32`, then `GetProcAddress`), so no SQLite is
//! built into Wakeshot. The watch thread calls [`query`] and feeds its answer as it is.
//!
//! A stub for now: every call answers the `no_sqlite` error, which the source reads as "no signal". Part B of
//! `p4-opencode-source` fills [`query`] (`docs/tasks/p4-opencode-source.md`, "Design for Part B"): open
//! `SQLITE_OPEN_READONLY | SQLITE_OPEN_NOMUTEX`, a 100 ms busy timeout, finalize and close on every call.
//!
//! Owner: `p4-opencode-source`; the stub and the signature by `p4-contracts-r2`.

use std::path::Path;

/// One query's answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Answer {
    /// The bytes to feed as `Feed::Whole`, in the contract's JSON: `{"rows":[[...],...]}` with integer, text or null
    /// cells, or `{"error":"<kind>"}` with a fixed kind (`open`, `prepare`, `no_sqlite`), never SQLite's own text.
    Json(Vec<u8>),
    /// `SQLITE_BUSY`: no feed, so the source keeps the last answer.
    Busy,
}

/// Runs `sql` read only on the database `db`.
pub fn query(_db: &Path, _sql: &str) -> Answer {
    Answer::Json(br#"{"error":"no_sqlite"}"#.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_stub_answers_no_sqlite_and_opens_nothing() {
        let missing = Path::new(r"Z:\no such folder\opencode.db");
        assert_eq!(
            query(missing, "SELECT 1"),
            Answer::Json(br#"{"error":"no_sqlite"}"#.to_vec())
        );
    }
}
