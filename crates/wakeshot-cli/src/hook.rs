//! `wakeshot hook <agent> <event> [--checked <value>...]`: the one command every agent's hook runs
//! (`docs/agent-hooks.md`, "The hook command"). A stub until that card builds it here: it answers `{}` and exits 0, so
//! a hook that runs it before then never blocks or changes the agent's prompt.
//!
//! Owner: `p4-cursor-source`.

use std::io::Write;
use std::process::ExitCode;

use crate::command::Global;

/// Cursor merges hook answers field by field, so `{}` changes nothing, and a non-empty answer keeps it from counting
/// the hook as failed (`docs/agent-hooks.md`, step 6).
const ANSWER: &[u8] = b"{}";

pub fn main(_global: &Global, _agent: &str, _event: &str, _values: &[String]) -> ExitCode {
    let mut out = std::io::stdout().lock();
    // A write error is ignored: the exit code must stay 0 whatever happens to the pipe.
    let _ = out.write_all(ANSWER).and_then(|()| out.flush());
    ExitCode::SUCCESS
}
