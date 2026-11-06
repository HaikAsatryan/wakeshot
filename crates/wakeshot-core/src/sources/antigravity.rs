//! Antigravity sessions, CLI and IDE, from the files Wakeshot's own Antigravity hook writes in `agents\antigravity\`
//! (architecture section 7; `docs/agent-hooks.md`, "Antigravity"; signals in `docs/research/antigravity-signals.md`).
//! The table is all the hook and the source know of Antigravity; the shared rules are in `sources::hooked`.
//!
//! Owner: `p4-antigravity-source`.

use std::path::Path;

use crate::sources::SourceId;
use crate::sources::hooked::{
    Action, Agent, Base, EventRule, HookedSource, ImageRule, InputFields, State, Under,
};

/// What the tooltip names a session of the IDE (its hub) or of any `agy.exe` outside the CLI's folder, and
/// Antigravity's plain name.
pub const LABEL: &str = "Antigravity";

/// What the tooltip names a session of the Antigravity CLI.
pub const CLI_LABEL: &str = "Antigravity CLI";

/// The CLI's install folder, `%LocalAppData%\agy\bin\` (its install script, `research/antigravity-signals.md`). The
/// IDE runs its own copy from `%USERPROFILE%\.gemini\bin\`.
const CLI_FOLDER: Under = Under {
    base: Base::LocalAppData,
    parts: &["agy", "bin"],
};

/// Antigravity's table (`docs/agent-hooks.md`, "Antigravity"). `agy` 1.5.2 runs each hook as `cmd /c <command>` with
/// the input on stdin, and the input has no event name, turn id or version (`research.md`, "Antigravity hook payload
/// form").
pub static AGENT: Agent = Agent {
    source: SourceId::Antigravity,
    name: "antigravity",
    label: LABEL,
    fields: InputFields {
        chat: "conversationId",
        turn: None,
        event: None,
        version: None,
        idle: Some("fullyIdle"),
    },
    events: &[
        // Before every model call, so one turn runs it many times: the write is skipped while the file already says
        // working for this process, and a file a late end deleted comes back at the next call.
        EventRule {
            name: "PreInvocation",
            action: Action::Write(State::Working),
            prune: true,
        },
        // The loop ended; `fullyIdle` false says background tasks (a dev server) still run, so the chat holds while
        // the process lives.
        EventRule {
            name: "Stop",
            action: Action::StopIfIdle,
            prune: false,
        },
    ],
    images: &[
        // First, so the CLI's own folder names its label before the rule for any `agy.exe`.
        ImageRule {
            image: "agy.exe",
            under: Some(CLI_FOLDER),
            key: "cli",
            label: CLI_LABEL,
        },
        // The IDE's hub and any other install: `agy.exe` is only Antigravity's name, and Antigravity has no session
        // end event, so the hold must end with this process.
        ImageRule {
            image: "agy.exe",
            under: None,
            key: "app",
            label: LABEL,
        },
    ],
    payload_key: "conversationId",
    input_problem: "Antigravity hook input",
    blocked_problem: "Antigravity hook command line",
};

/// Antigravity's source: `data_root` is Wakeshot's data folder, the one that holds `agents`.
pub fn source(data_root: &Path) -> HookedSource {
    HookedSource::new(&AGENT, data_root)
}
