//! Cursor sessions, editor and CLI, from the files Wakeshot's own Cursor hook writes in `agents\cursor\` (architecture
//! sections 7 and 12; `docs/agent-hooks.md`, "Cursor"). The table is all the hook and the source know of Cursor; the
//! shared rules are in `sources::hooked`. The editor's wakelock lines in `main.log` are not read
//! (`docs/agent-hooks.md`, "Cursor's `main.log`: dropped").
//!
//! Not started by the v1 tray, and its hook folder is not added, per "No guessing" in `docs/product.md`: a chat deleted
//! while it runs sends no `stop`, only the `sessionEnd` that Ctrl+F4 also sends, so its end cannot be seen. Kept, with
//! its tests, for a later version.

use std::path::Path;

use crate::sources::SourceId;
use crate::sources::hooked::{
    Action, Agent, Base, EventRule, HookedSource, ImageRule, InputFields, Running, State, Under,
};

/// What the tooltip names a session of the editor, and Cursor's plain name.
pub const LABEL: &str = "Cursor";

/// What the tooltip names a session of the Cursor CLI.
pub const CLI_LABEL: &str = "Cursor CLI";

/// The CLI's install folder, `%LocalAppData%\cursor-agent\` (`docs/research/other-agent-signals.md`; its `node.exe`
/// lies in `versions\<version>\`, read 2026-11-06).
const CLI_FOLDER: Under = Under {
    base: Base::LocalAppData,
    parts: &["cursor-agent"],
};

/// Cursor's table (`docs/agent-hooks.md`, "Events and states" and "The hook command").
pub static AGENT: Agent = Agent {
    source: SourceId::Cursor,
    name: "cursor",
    label: LABEL,
    fields: InputFields {
        chat: "conversation_id",
        turn: Some("generation_id"),
        event: Some("hook_event_name"),
        version: Some("cursor_version"),
        idle: None,
    },
    events: &[
        EventRule {
            name: "beforeSubmitPrompt",
            action: Action::Write(State::Working),
            prune: true,
        },
        // Any `status` (completed, aborted, error): each ends the turn.
        EventRule {
            name: "stop",
            action: Action::DeleteIfTurn,
            prune: false,
        },
        // Closing a running chat's tab fires this with `final_status` `generating`; the turn goes on and its own
        // `stop` comes when it ends (seen live, editor 3.27.12), so the chat stays working until then.
        EventRule {
            name: "sessionEnd",
            action: Action::EndUnlessRunning(Running {
                field: "final_status",
                value: "generating",
            }),
            prune: false,
        },
    ],
    images: &[
        // The editor runs hooks from its extension host or agent host daemon, both `Cursor.exe`.
        ImageRule {
            image: "Cursor.exe",
            under: None,
            key: "editor",
            label: LABEL,
        },
        // `node.exe` is any Node, so only the CLI's own folder counts.
        ImageRule {
            image: "node.exe",
            under: Some(CLI_FOLDER),
            key: "cli",
            label: CLI_LABEL,
        },
        ImageRule {
            image: "cursor-agent.exe",
            under: Some(CLI_FOLDER),
            key: "cli",
            label: CLI_LABEL,
        },
    ],
    payload_key: "conversation_id",
    input_problem: "Cursor hook input",
    blocked_problem: "Cursor hook command line",
};

/// Cursor's source: `data_root` is Wakeshot's data folder, the one that holds `agents`.
pub fn source(data_root: &Path) -> HookedSource {
    HookedSource::new(&AGENT, data_root)
}
