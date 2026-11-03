//! Hold files as facts. Parsing the files (and `expires`) is the sources track's work (Phase 4, `file`); the core
//! track turns these facts and agent sessions into holds.
//!
//! Owner: contracts card. Each type's own doc names the track that owns its meaning.

pub mod file;

use crate::process::Alive;

/// A hold file's name in `holds\`, so the core can ask for exactly that file to be deleted.
///
/// Owner: sources track (Phase 4). The core may rely on it being a bare file name such as `run-4242.json` or
/// `keep-awake.json`, never a path.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HoldId(pub String);

/// What kind of thing holds, because the rules differ: Automatic off stops `Agent`, `Run` and `Script` but not
/// `KeepAwake`, and only `KeepAwake` may hold the screen.
///
/// Owner: core track (Phase 2). Hold files give `Run` (`run-<pid>.json`), `KeepAwake` (`keep-awake.json`) or
/// `Script` (any other name); `Agent` comes only from sources.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HoldKind {
    Agent,
    Run,
    Script,
    KeepAwake,
}

/// One parsed hold file with its liveness, a full snapshot entry of `Event::Holds`.
///
/// Owner: sources track (Phase 4) fills it from `HoldFile`; the core track (Phase 2) decides from it. The core may
/// rely on `reason` being cut to 60 characters, `screen` being false outside `keep-awake.json`, and `alive` coming
/// from the one liveness check (architecture section 5).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HoldFacts {
    pub id: HoldId,
    pub kind: HoldKind,
    pub alive: Alive,
    pub expires_wall_ms: Option<i64>,
    pub reason: Option<String>,
    pub screen: bool,
}
