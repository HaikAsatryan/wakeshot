//! Split a `JournalFile` into the entries this version knows and the rest, and merge them back for a save (always
//! `JournalFile::VERSION`; unknown keys kept untouched).
//!
//! Owner: `p2-ownership-restore`.
