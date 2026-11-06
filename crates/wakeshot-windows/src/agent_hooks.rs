//! Adds, keeps and removes the hook folders Wakeshot gives an agent, so the agent itself tells Wakeshot when it works
//! (`docs/agent-hooks.md`, "Adding" and "Removing"). Not Velopack's install hooks: those are `hooks.rs`, whose
//! uninstall step (`HookEnv::remove_agent_hooks`) calls [`on_uninstall`]. The tray's start calls [`at_start`] through
//! `app::startup::add_agent_hooks`; the CLI's `wakeshot hook <agent> remove` calls [`remove_command`].
//!
//! Owner: `p4-agent-hooks-install`. `p4-antigravity-source` adds its agent to [`AGENTS`].
//!
//! Every read, write and delete goes by an [`Agent`]'s fixed names, and each part of each path, from the agent's own
//! folder down to the file, must be a plain folder or file: a reparse point of any kind (a junction, a link, a cloud
//! file) stops the work for that agent. The marker names files by fixed keys and holds only hashes, so a crafted one
//! steers nothing. All of it runs only at Medium integrity: these folders inherit the profile's rights, so any Medium
//! process of the user can swap a part for a junction. At Medium that gains it nothing; an elevated write or delete
//! there would hand it a right it does not have (`docs/agent-hooks.md`, step 4).

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::{self, Read, Write};
use std::iter;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde_json::Value;
use windows::Win32::Foundation::{ERROR_ALREADY_EXISTS, ERROR_DIR_NOT_EMPTY, ERROR_FILE_EXISTS};
use windows::Win32::Storage::FileSystem::{
    FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_OPEN_REPARSE_POINT, GetShortPathNameW,
    MOVEFILE_WRITE_THROUGH, MoveFileExW,
};
use windows::Win32::System::Com::CoTaskMemFree;
use windows::Win32::UI::Shell::{FOLDERID_Profile, KF_FLAG_DEFAULT, SHGetKnownFolderPath};
use windows::core::PCWSTR;

use crate::WinError;
use crate::log::{Level, Log};
use crate::process;
use crate::store;

const AREA: &str = "agent-hooks";

/// The plugin folder's name, the same for every agent.
pub const FOLDER: &str = "wakeshot";
/// Wakeshot's marker in each plugin folder: the hash of each fixed file it wrote, by fixed key.
pub const MARKER: &str = "wakeshot-hooks.json";
/// A plugin folder while it is written, then renamed in one step, so the agent never sees half a plugin. The pid
/// follows. A dot name, which Cursor's CLI skips.
pub const STAGING_PREFIX: &str = ".wakeshot-new-";
/// A plugin folder the uninstall could not empty: a dot name, so the agent stops loading it, and the user's edits stay.
pub const REMOVED: &str = ".wakeshot-removed";
/// Wakeshot's record in the data folder: which agents it added a folder to, and which the user removed since.
pub const RECORD: &str = "agent-hooks.json";
/// The hook command's exe, beside the tray exe in the install folder.
pub const HOOK_EXE: &str = "wakeshot.exe";
/// Velopack's install folder under `%LocalAppData%`, and its folder whose path stays the same across updates.
const INSTALL_DIR: &str = "HaikAsatryan.Wakeshot";
const CURRENT_DIR: &str = "current";
/// The uninstall waits this long for all agents' Medium copies together.
pub const COPY_WAIT_MS: u32 = 5_000;
/// A bigger fixed file is not Wakeshot's: what it writes is under 2 KB.
const FILE_MAX: u64 = 64 * 1024;
/// The marker's `wakeshot` value: the version that wrote the folder.
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// One file Wakeshot writes in an agent's plugin folder.
#[derive(Debug)]
pub struct FixedFile {
    /// Its key in the marker's `files`.
    pub key: &'static str,
    /// The subfolder of the plugin folder it lives in, one of [`Agent::folders`]; `None`: the plugin folder itself.
    pub folder: Option<&'static str>,
    pub name: &'static str,
}

/// One agent Wakeshot adds hooks to: where its plugin folder goes, its fixed files and their content.
#[derive(Debug)]
pub struct Agent {
    /// The word in `wakeshot hook <agent> ...` and the record's key.
    pub key: &'static str,
    /// The notice's name for it (`Event::HooksAdded`).
    pub label: &'static str,
    /// From the profile folder down to the folder that holds the plugin folder.
    pub parents: &'static [&'static str],
    /// How many leading `parents` the agent makes itself: they must exist for the agent to count as present, and
    /// Wakeshot never makes them. It makes the rest.
    pub agent_made: usize,
    /// What else the agent must show to count as present. Cursor needs nothing more than its folder.
    pub present: fn(&Folders) -> bool,
    /// The plugin folder's subfolders, made before the files.
    pub folders: &'static [&'static str],
    /// The files but the marker, in the order they are written; the marker comes last.
    pub files: &'static [FixedFile],
    /// The bytes of each of `files`, in order, for the hook exe `exe`. `None`: this path cannot go into the agent's
    /// hook command safely, so the agent gets no hooks.
    pub contents: fn(exe: &Path) -> Option<Vec<Vec<u8>>>,
}

/// Every agent Wakeshot adds hooks to, in the order a start handles them.
pub static AGENTS: &[Agent] = &[CURSOR, ANTIGRAVITY];

/// The agent with this command word, if this version knows it.
pub fn find(key: &str) -> Option<&'static Agent> {
    AGENTS.iter().find(|agent| agent.key == key)
}

/// `docs/agent-hooks.md`, "The folder".
const CURSOR: Agent = Agent {
    key: "cursor",
    label: "Cursor",
    parents: &[".cursor", "plugins", "local"],
    agent_made: 1,
    present: always,
    folders: &[".cursor-plugin", "hooks"],
    files: &[
        FixedFile {
            key: "plugin",
            folder: Some(".cursor-plugin"),
            name: "plugin.json",
        },
        FixedFile {
            key: "hooks",
            folder: Some("hooks"),
            name: "hooks.json",
        },
    ],
    contents: cursor_files,
};

fn always(_: &Folders) -> bool {
    true
}

/// Cursor's manifest needs only `name` (`cursor.com/docs/reference/plugins`); the rest tells a user who finds the
/// folder where it came from.
const CURSOR_PLUGIN: &str = r#"{"name":"wakeshot","version":"1.0.0","description":"Lets Wakeshot keep the PC awake while the agent works. Added by Wakeshot.","author":{"name":"Wakeshot"},"homepage":"https://github.com/HaikAsatryan/wakeshot"}"#;

/// The Cursor events Wakeshot hooks, in the order `hooks.json` lists them.
const CURSOR_EVENTS: [&str; 3] = ["beforeSubmitPrompt", "stop", "sessionEnd"];

/// Cursor waits this many seconds for one hook run, instead of its 60 s default, whatever happens to the exe.
const HOOK_TIMEOUT_S: u32 = 5;

fn cursor_files(exe: &Path) -> Option<Vec<Vec<u8>>> {
    let quoted = powershell_quoted(exe)?;
    let hooks: Vec<String> = CURSOR_EVENTS
        .iter()
        .map(|event| {
            let command = format!("{quoted} hook cursor {event}");
            format!(
                "    {}: [ {{ \"command\": {}, \"timeout\": {HOOK_TIMEOUT_S} }} ]",
                json_string(event),
                json_string(&command)
            )
        })
        .collect();
    let hooks = format!(
        "{{\n  \"version\": 1,\n  \"hooks\": {{\n{}\n  }}\n}}\n",
        hooks.join(",\n")
    );
    Some(vec![
        format!("{CURSOR_PLUGIN}\n").into_bytes(),
        hooks.into_bytes(),
    ])
}

/// `'<path>'`, the one shape that works in both of Cursor's PowerShell forms (`& <command>` and `$input | <command>`):
/// single quotes expand nothing, and an ASCII `'` is doubled, which PowerShell reads as one and Cursor's own path scan
/// skips. `None` for a path PowerShell could still split: one with U+2018 to U+201B, which it also counts as single
/// quotes, or a control character, or one that is not Unicode.
fn powershell_quoted(exe: &Path) -> Option<String> {
    let text = exe.to_str()?;
    let unsafe_char = |c: char| c.is_control() || ('\u{2018}'..='\u{201B}').contains(&c);
    if text.is_empty() || text.chars().any(unsafe_char) {
        return None;
    }
    Some(format!("'{}'", text.replace('\'', "''")))
}

fn json_string(text: &str) -> String {
    Value::String(text.to_owned()).to_string()
}

/// `docs/agent-hooks.md`, "Antigravity": one plugin folder, which the CLI and the IDE's hub (the same `agy.exe`) both
/// load. `hooks.json` sits in the plugin folder itself: `agy` reads no `hooks\hooks.json` in a plugin.
const ANTIGRAVITY: Agent = Agent {
    key: "antigravity",
    label: "Antigravity",
    parents: &[".gemini", "config", "plugins"],
    // `.gemini` is also Gemini CLI's folder, so only Antigravity's own `config` in it shows Antigravity was here.
    agent_made: 2,
    present: antigravity_present,
    folders: &[],
    // The manifest last: `agy` 1.5.2's plugin listing skips no dot name (`research.md`, "Antigravity hook payload
    // form"), so a staging folder becomes a plugin only with its last file, just before the rename.
    files: &[
        FixedFile {
            key: "hooks",
            folder: None,
            name: "hooks.json",
        },
        FixedFile {
            key: "plugin",
            folder: None,
            name: "plugin.json",
        },
    ],
    contents: antigravity_files,
};

/// Moves the `.gemini` folder (a string in `agy`; what it moves exactly is not verified), so the folders above may not
/// be the ones Antigravity reads: no hooks then.
const GEMINI_DIR_ENV: &str = "ANTIGRAVITY_GEMINI_DIR";

fn antigravity_present(folders: &Folders) -> bool {
    agy_present(folders, std::env::var_os(GEMINI_DIR_ENV).is_some())
}

/// `agy.exe` where the CLI's install script puts it, or where the IDE downloads its copy. A `.gemini` with no `agy.exe`
/// is Gemini CLI's, which Wakeshot does not hook.
fn agy_present(folders: &Folders, gemini_dir_moved: bool) -> bool {
    let places = [
        folders.local_app_data.join("agy").join("bin"),
        folders.profile.join(".gemini").join("bin"),
    ];
    !gemini_dir_moved && places.iter().any(|dir| dir.join("agy.exe").is_file())
}

/// The manifest needs `name`; the other fields are the ones `agy` 1.5.2's manifest type reads, and tell a user who
/// finds the folder where it came from.
const ANTIGRAVITY_PLUGIN: &str = r#"{"name":"wakeshot","version":"1.0.0","description":"Lets Wakeshot keep the PC awake while the agent works. Added by Wakeshot.","author":{"name":"Wakeshot"}}"#;

/// The Antigravity events Wakeshot hooks, in the order `hooks.json` lists them.
const ANTIGRAVITY_EVENTS: [&str; 2] = ["PreInvocation", "Stop"];

/// One named hook group, `wakeshot`, with one command per event: `agy` merges the groups of every plugin by event.
fn antigravity_files(exe: &Path) -> Option<Vec<Vec<u8>>> {
    let bare = cmd_bare(exe, false).or_else(|| cmd_bare(&short_path(exe)?, true))?;
    let hooks: Vec<String> = ANTIGRAVITY_EVENTS
        .iter()
        .map(|event| {
            let command = format!("{bare} hook antigravity {event}");
            format!(
                "    {}: [ {{ \"type\": \"command\", \"command\": {}, \"timeout\": {HOOK_TIMEOUT_S} }} ]",
                json_string(event),
                json_string(&command)
            )
        })
        .collect();
    let hooks = format!("{{\n  \"wakeshot\": {{\n{}\n  }}\n}}\n", hooks.join(",\n"));
    Some(vec![
        hooks.into_bytes(),
        format!("{ANTIGRAVITY_PLUGIN}\n").into_bytes(),
    ])
}

/// The exe path as it is, for `agy`, which runs a hook as `cmd /c <command>` through Go's `os/exec`: Go wraps the whole
/// command in quotes and writes a quote in it as `\"`, which `cmd` does not read, and `cmd` gives `% ^ & ( ) ! @`
/// their meaning (`research.md`, "Antigravity hook payload form"). So only a path that needs no quoting at all: each
/// character one of `A-Z a-z 0-9 \ : . _ -` or a letter above ASCII. `None` for anything else (a space, a quote, `~`),
/// or a path that is not Unicode. `short`: an 8.3 form, whose `~` inside a name means nothing to `cmd` or `agy` (which
/// expands only a leading `~/`).
fn cmd_bare(exe: &Path, short: bool) -> Option<String> {
    let text = exe.to_str()?;
    let plain = |c: char| {
        c.is_ascii_alphanumeric()
            || matches!(c, '\\' | ':' | '.' | '_' | '-')
            || (short && c == '~')
            || (!c.is_ascii() && c.is_alphabetic())
    };
    let shaped = !text.is_empty() && !text.starts_with('~');
    (shaped && text.chars().all(plain)).then(|| text.to_owned())
}

/// The 8.3 form of `exe` (`GetShortPathNameW`), so a profile path with a space can still get Antigravity's hooks.
/// Where a volume makes no short names, Windows gives the long form back, which [`cmd_bare`] then refuses again.
fn short_path(exe: &Path) -> Option<PathBuf> {
    let long = wide(exe);
    let mut buffer = vec![0u16; 1024];
    // SAFETY: `long` is NUL-terminated and lives for the call; the buffer's length goes with it.
    let length = unsafe { GetShortPathNameW(PCWSTR(long.as_ptr()), Some(&mut buffer)) } as usize;
    (length > 0 && length < buffer.len())
        .then(|| PathBuf::from(OsString::from_wide(&buffer[..length])))
}

/// Where the agents' folders are: the real ones, or temp folders in tests.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Folders {
    /// The profile folder (`FOLDERID_Profile`), which holds `.cursor`.
    pub profile: PathBuf,
    /// `%LocalAppData%`, where an agent's install folder can show that the agent is here.
    pub local_app_data: PathBuf,
}

impl Folders {
    /// Both from their known folder ids, never from the environment, which the process that started this one sets.
    pub fn real() -> Result<Self, WinError> {
        Ok(Self {
            profile: profile_folder()?,
            local_app_data: store::local_app_data()?,
        })
    }
}

fn profile_folder() -> Result<PathBuf, WinError> {
    // SAFETY: the folder id is a static; no token means the calling user.
    let path = unsafe { SHGetKnownFolderPath(&FOLDERID_Profile, KF_FLAG_DEFAULT, None) }
        .map_err(|e| core_error("SHGetKnownFolderPath", &e))?;
    // SAFETY: on success the shell gives a NUL-terminated string it allocated; it is copied once, then freed once.
    let copy = unsafe { OsString::from_wide(path.as_wide()) };
    unsafe { CoTaskMemFree(Some(path.0 as *const _)) };
    Ok(PathBuf::from(copy))
}

/// The hook command's exe beside `tray_exe`, when `tray_exe` runs from Velopack's install folder,
/// `%LocalAppData%\HaikAsatryan.Wakeshot\current\`, whose path updates keep, so the hook files never name a folder an
/// update removes. `None` for a dev build, a copy elsewhere or a Store build: those touch no agent's folders
/// (`docs/agent-hooks.md`, "Adding").
pub fn installed_hook_exe(tray_exe: &Path, local_app_data: &Path) -> Option<PathBuf> {
    let current = local_app_data.join(INSTALL_DIR).join(CURRENT_DIR);
    let folder = tray_exe.parent()?;
    let same = folder == current
        || folder
            .to_str()
            .zip(current.to_str())
            .is_some_and(|(a, b)| a.eq_ignore_ascii_case(b));
    (same && folder.is_absolute()).then(|| folder.join(HOOK_EXE))
}

/// The tray's start (`app::startup::add_agent_hooks`): for each agent of [`AGENTS`] that is on this PC, the table of
/// `docs/agent-hooks.md`, "Adding": add the folder once, keep it current, never touch it again once the user removed
/// or changed it. `record` is `agent-hooks.json` in the data folder; `hook_exe` the installed `wakeshot.exe`
/// ([`installed_hook_exe`]); `above_medium` this process's level (`process::current_above_medium`). Above Medium, or
/// with the level unknown, nothing is touched. Returns the labels of the agents whose folder it added, for
/// `Event::HooksAdded`. Never fails: each problem is a log line.
pub fn at_start(
    folders: &Folders,
    record: &Path,
    hook_exe: &Path,
    above_medium: Result<bool, WinError>,
    log: &dyn Log,
) -> Vec<&'static str> {
    if !medium(above_medium, "touches no agent's folder", log) {
        return Vec::new();
    }
    if !matches!(kind(hook_exe), Ok(Kind::File)) {
        let text =
            format!("{HOOK_EXE} is not beside the tray, so no agent gets hooks that would run it");
        line(log, Level::Warn, &text);
        return Vec::new();
    }
    let mut marks = match read_record(record) {
        Ok(marks) => Some(marks),
        Err(why) => {
            let text = format!(
                "{RECORD} {why}, so no agent gets a new hook folder; delete it and restart Wakeshot to start over"
            );
            line(log, Level::Warn, &text);
            None
        }
    };
    let before = marks.clone();
    let mut added = Vec::new();
    for agent in AGENTS {
        match start_one(folders, agent, hook_exe, &mut marks, log) {
            Ok(true) => added.push(agent.label),
            Ok(false) => {}
            Err(e) => {
                let text = format!("{}: hook folder work stopped: {}", agent.label, failed(e));
                line(log, Level::Warn, &text);
            }
        }
    }
    if let Some(marks) = &marks
        && Some(marks) != before.as_ref()
        && let Err(e) = store::write_atomic(record, &record_bytes(marks))
    {
        let text = format!("{RECORD} not written: {}", failed(e));
        line(log, Level::Warn, &text);
    }
    added
}

/// One agent's row of the table. `Ok(true)`: the folder was added.
fn start_one(
    folders: &Folders,
    agent: &'static Agent,
    hook_exe: &Path,
    marks: &mut Option<Marks>,
    log: &dyn Log,
) -> Result<bool, WinError> {
    let chain = walk(folders, agent);
    // The agent's own folder gone (`.cursor` deleted with Cursor) is the agent gone, not the user saying no to the hooks
    // (orchestrator, 2026-11-06, for Haik's review): so the record forgets it, and a later install gets the hooks again.
    // Only Wakeshot's folder gone with the agent's folder still there stays "removed".
    if matches!(chain, Ok(Chain::Absent)) {
        if let Some(marks) = marks
            && marks.0.remove(agent.key).is_some()
        {
            let text = format!(
                "{}: {} is gone, so Wakeshot forgets its hook folder there and adds it again when the agent is back",
                agent.label,
                shown(&agent.parents[..agent.agent_made])
            );
            line(log, Level::Info, &text);
        }
        return Ok(false);
    }
    if !(agent.present)(folders) {
        return Ok(false);
    }
    let made = match chain? {
        Chain::Absent => return Ok(false),
        Chain::Blocked { at } => {
            blocked(agent, &shown(&agent.parents[..=at]), log);
            return Ok(false);
        }
        Chain::Plain { made } => made,
    };
    let Some(want) = wanted(agent, hook_exe) else {
        let text = format!(
            "{}: the hook exe's path holds a character the hook command cannot quote, so it gets no hooks",
            agent.label
        );
        line(log, Level::Warn, &text);
        return Ok(false);
    };
    let local = parent_dir(folders, agent);
    if made == agent.parents.len() {
        clean_staging(&local, agent, Some(&want), log);
    }
    let dir = local.join(FOLDER);
    let shown_dir = shown_folder(agent);
    match kind(&dir)? {
        Kind::Missing => {
            let Some(marks) = marks else {
                return Ok(false);
            };
            match marks.0.get(agent.key) {
                None => {
                    add(folders, agent, made, &want)?;
                    marks.0.insert(agent.key.to_owned(), Mark::Added);
                    let text = format!("{}: hook folder added ({shown_dir})", agent.label);
                    line(log, Level::Info, &text);
                    Ok(true)
                }
                Some(Mark::Added) => {
                    marks.0.insert(agent.key.to_owned(), Mark::Removed);
                    let text = format!(
                        "{}: {shown_dir} is gone, so the user removed it; Wakeshot does not add it again",
                        agent.label
                    );
                    line(log, Level::Info, &text);
                    Ok(false)
                }
                Some(Mark::Removed) => Ok(false),
            }
        }
        Kind::Folder => {
            let look = look(&dir, agent, Some(&want), false)?;
            if look.marker.is_none() {
                not_ours(agent, log);
                return Ok(false);
            }
            if !look.unchanged() {
                let text = format!(
                    "{}: {shown_dir} was changed since Wakeshot wrote it, so Wakeshot leaves it as it is",
                    agent.label
                );
                line(log, Level::Info, &text);
            } else if !look.current {
                rewrite(&dir, agent, &want, &look)?;
                let text = format!("{}: {shown_dir} rewritten for this version", agent.label);
                line(log, Level::Info, &text);
            }
            if let Some(marks) = marks {
                marks.0.insert(agent.key.to_owned(), Mark::Added);
            }
            Ok(false)
        }
        Kind::Link => {
            blocked(agent, &shown_dir, log);
            Ok(false)
        }
        Kind::File | Kind::Other => {
            not_ours(agent, log);
            Ok(false)
        }
    }
}

/// Makes the parents Wakeshot may make, then the plugin folder under a staging name that must not exist yet, fills
/// it, and renames it into place in one step.
fn add(folders: &Folders, agent: &Agent, made: usize, want: &Wanted) -> Result<(), WinError> {
    let mut local = folders.profile.clone();
    for (i, part) in agent.parents.iter().enumerate() {
        local.push(part);
        if i < made {
            continue;
        }
        match fs::create_dir(&local) {
            Ok(()) => {}
            // Made by someone else since the walk: still fine if it is a plain folder.
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                if kind(&local)? != Kind::Folder {
                    return Err(io_error("CreateDirectoryW", &e));
                }
            }
            Err(e) => return Err(io_error("CreateDirectoryW", &e)),
        }
    }
    let staging = local.join(format!("{STAGING_PREFIX}{}", std::process::id()));
    // `CreateDirectoryW` fails when the name exists, so a folder someone else put there is never filled.
    fs::create_dir(&staging).map_err(|e| io_error("CreateDirectoryW", &e))?;
    let done =
        fill(&staging, agent, want).and_then(|()| move_folder(&staging, &local.join(FOLDER)));
    if let Err(e) = done {
        // The same rule as for a leftover at the next start: by the fixed names only.
        if let Ok(look) = look(&staging, agent, Some(want), true) {
            delete_ours(&staging, agent, &look);
        }
        return Err(e);
    }
    Ok(())
}

fn fill(staging: &Path, agent: &Agent, want: &Wanted) -> Result<(), WinError> {
    for folder in agent.folders {
        fs::create_dir(staging.join(folder)).map_err(|e| io_error("CreateDirectoryW", &e))?;
    }
    for (fixed, bytes) in agent.files.iter().zip(&want.files) {
        create_new(&file_path(staging, fixed), bytes)?;
    }
    create_new(&staging.join(MARKER), &want.marker)
}

/// `CREATE_NEW`: a name that exists, a link at it too, is never opened.
fn create_new(path: &Path, bytes: &[u8]) -> Result<(), WinError> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| io_error("CreateFileW", &e))?;
    file.write_all(bytes)
        .map_err(|e| io_error("WriteFile", &e))?;
    file.sync_all()
        .map_err(|e| io_error("FlushFileBuffers", &e))
}

/// Each fixed file that differs from this version's, then the marker, each with `store::write_atomic`. A stop between
/// two writes leaves each file either as the marker names it or as this version writes it, so the folder stays
/// Wakeshot's and the next start finishes the job.
fn rewrite(dir: &Path, agent: &Agent, want: &Wanted, look: &Look) -> Result<(), WinError> {
    for temp in &look.leftovers {
        remove_file(temp)?;
    }
    for (fixed, bytes) in agent.files.iter().zip(&want.files) {
        let path = file_path(dir, fixed);
        if read_file(&path)?.as_deref() != Some(bytes.as_slice()) {
            store::write_atomic(&path, bytes)?;
        }
    }
    let marker = dir.join(MARKER);
    if read_file(&marker)?.as_deref() != Some(want.marker.as_slice()) {
        store::write_atomic(&marker, &want.marker)?;
    }
    Ok(())
}

/// Removes the staging folders a stop left in `local` (`.wakeshot-new-*`), each only when it holds nothing but fixed
/// files that are Wakeshot's: what this version writes, the start of it (a stop during the write), or what the
/// folder's own marker names. Anything else stays, one log line. A leftover never blocks the main folder's work (review
/// m3), so each problem here is a log line, never an error.
fn clean_staging(local: &Path, agent: &Agent, want: Option<&Wanted>, log: &dyn Log) {
    let names = match list(local) {
        Ok(names) => names,
        Err(e) => {
            let text = format!(
                "{}: {} does not list, so no staging folder is removed: {}",
                agent.label,
                shown(agent.parents),
                failed(e)
            );
            line(log, Level::Warn, &text);
            return;
        }
    };
    for name in names {
        let staging = name.to_str().is_some_and(|n| {
            n.get(..STAGING_PREFIX.len())
                .is_some_and(|start| start.eq_ignore_ascii_case(STAGING_PREFIX))
        });
        if !staging {
            continue;
        }
        let path = local.join(&name);
        let shown_staging = format!("{}\\{}", shown(agent.parents), name.to_string_lossy());
        let ours = kind(&path).and_then(|kind| {
            if kind != Kind::Folder {
                return Ok(false);
            }
            let look = look(&path, agent, want, true)?;
            let ours = !look.extra
                && look.marker_seen != Seen::Other
                && look.files.iter().all(|seen| *seen != Seen::Other);
            Ok(ours && delete_ours(&path, agent, &look).gone)
        });
        let (level, text) = match ours {
            Ok(true) => (
                Level::Info,
                format!(
                    "{}: removed {shown_staging}, which a stop left",
                    agent.label
                ),
            ),
            Ok(false) => (
                Level::Warn,
                format!(
                    "{}: {shown_staging} holds more than Wakeshot's own files, so Wakeshot leaves it",
                    agent.label
                ),
            ),
            Err(e) => (
                Level::Warn,
                format!(
                    "{}: {shown_staging} does not read, so Wakeshot leaves it: {}",
                    agent.label,
                    failed(e)
                ),
            ),
        };
        line(log, level, &text);
    }
}

/// The uninstall step (`hooks::on_uninstall`, right after the notice app id): at Medium in this process. Above
/// Medium never in this process: for each agent whose folders are there, a copy of the hook command at Medium,
/// `wakeshot.exe hook <agent> remove`, started under Explorer by `copy` ([`medium_copies`]). With the level unknown,
/// nothing is removed. `hook_exe`: the `wakeshot.exe` the hook files name. Each outcome is a log line.
pub fn on_uninstall(
    folders: &Folders,
    hook_exe: Option<&Path>,
    above_medium: Result<bool, WinError>,
    copy: &mut dyn FnMut(&Path, &[&'static Agent]) -> Vec<CopyRun>,
    log: &dyn Log,
) {
    match above_medium {
        Ok(false) => {
            for agent in AGENTS {
                remove(folders, agent, hook_exe, log);
            }
        }
        Ok(true) => {
            // A look at High, which writes nothing.
            let present: Vec<&'static Agent> = AGENTS
                .iter()
                .filter(|agent| !matches!(kind(&parent_dir(folders, agent)), Ok(Kind::Missing)))
                .collect();
            let Some(exe) = hook_exe else {
                for agent in present {
                    let text = format!(
                        "{}: hook folder not removed: the uninstall runs above Medium and has no {HOOK_EXE} to hand it to",
                        agent.label
                    );
                    line(log, Level::Warn, &text);
                }
                return;
            };
            if present.is_empty() {
                return;
            }
            // One call for every agent, so all copies share one wait (review m1): the restore after this step counts
            // its retries from the hook's start.
            let runs = copy(exe, &present);
            for (agent, run) in present.iter().zip(runs) {
                let text = match run {
                    CopyRun::Ended { outside_tree: true } => format!(
                        "{}: hook folder removal handed to a Medium copy, which ended; its lines are in the cli log",
                        agent.label
                    ),
                    CopyRun::Ended {
                        outside_tree: false,
                    }
                    | CopyRun::StillRuns {
                        outside_tree: false,
                    } => format!(
                        "{}: hook folder not removed: its copy could not start under Explorer, so it kept this token and removes nothing",
                        agent.label
                    ),
                    CopyRun::StillRuns { outside_tree: true } => format!(
                        "{}: hook folder removal handed to a Medium copy, which did not end within {} s",
                        agent.label,
                        COPY_WAIT_MS / 1000
                    ),
                    CopyRun::Failed(e) => format!(
                        "{}: hook folder not removed: the Medium copy did not run: {}",
                        agent.label,
                        failed(e)
                    ),
                };
                line(log, Level::Info, &text);
            }
        }
        Err(e) => {
            let text = format!(
                "this process's integrity level does not read ({}), so no agent's hook folder is removed",
                failed(e)
            );
            line(log, Level::Warn, &text);
        }
    }
}

/// What the uninstall's Medium copy for one agent did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CopyRun {
    /// It ended within [`COPY_WAIT_MS`]. `outside_tree: false`: it was not started under Explorer, so it kept the
    /// caller's token, and its own check made it remove nothing.
    Ended {
        outside_tree: bool,
    },
    StillRuns {
        outside_tree: bool,
    },
    Failed(WinError),
}

/// The real copies, one per agent in `agents`, in order: `<exe> hook <agent> remove` through
/// `process::spawn_outside_tree`, which gives an elevated caller a child with Explorer's Medium token. All start before
/// any wait, and the waits share [`COPY_WAIT_MS`].
pub fn medium_copies(exe: &Path, agents: &[&'static Agent]) -> Vec<CopyRun> {
    let started = Instant::now();
    let spawned: Vec<_> = agents
        .iter()
        .map(|agent| {
            let args = ["hook", agent.key, "remove"].map(OsString::from);
            process::spawn_outside_tree(exe, &args)
        })
        .collect();
    spawned
        .into_iter()
        .map(|spawned| match spawned {
            Ok(spawned) => {
                let outside_tree = spawned.outside_tree;
                match spawned.handle.wait(wait_left(started.elapsed())) {
                    Ok(true) => CopyRun::Ended { outside_tree },
                    Ok(false) => CopyRun::StillRuns { outside_tree },
                    Err(e) => CopyRun::Failed(e),
                }
            }
            Err(e) => CopyRun::Failed(e),
        })
        .collect()
}

/// What is left of the copies' shared wait after `elapsed`. A wait of 0 still tells whether a copy ended.
fn wait_left(elapsed: Duration) -> u32 {
    COPY_WAIT_MS.saturating_sub(u32::try_from(elapsed.as_millis()).unwrap_or(u32::MAX))
}

/// `wakeshot hook <agent> remove` (`docs/agent-hooks.md`, "Removing"): the uninstall's Medium copy, and the hook's own
/// removal once it finds its input on a command line. Only at Medium: above it, or with the level unknown, it removes
/// nothing (one log line). An agent this version does not know does nothing, as the frozen command form promises.
pub fn remove_command(agent: &str, log: &dyn Log) {
    let Some(agent) = find(agent) else {
        return;
    };
    if !medium(
        process::current_above_medium(),
        "removes no hook folder",
        log,
    ) {
        return;
    }
    let folders = match Folders::real() {
        Ok(folders) => folders,
        Err(e) => {
            let text = format!(
                "{}: hook folder not removed: the profile folder does not read: {}",
                agent.label,
                failed(e)
            );
            line(log, Level::Warn, &text);
            return;
        }
    };
    let exe = std::env::current_exe().ok();
    remove(&folders, agent, exe.as_deref(), log);
}

/// Removes `agent`'s plugin folder and its leftover staging folders by the fixed names, in this process: the caller
/// checked the level, so it stays private (review m6). `hook_exe`: files this version would write for it count as
/// Wakeshot's, also under an older marker. A folder that cannot be emptied (a file the user changed or added, a file
/// held open) is renamed to the first free name of [`removed_names`].
fn remove(folders: &Folders, agent: &'static Agent, hook_exe: Option<&Path>, log: &dyn Log) {
    if let Err(e) = remove_one(folders, agent, hook_exe, log) {
        let text = format!("{}: hook folder not removed: {}", agent.label, failed(e));
        line(log, Level::Warn, &text);
    }
}

fn remove_one(
    folders: &Folders,
    agent: &'static Agent,
    hook_exe: Option<&Path>,
    log: &dyn Log,
) -> Result<(), WinError> {
    match walk(folders, agent)? {
        Chain::Plain { made } if made == agent.parents.len() => {}
        Chain::Blocked { at } => {
            blocked(agent, &shown(&agent.parents[..=at]), log);
            return Ok(());
        }
        Chain::Absent | Chain::Plain { .. } => return Ok(()),
    }
    let want = hook_exe.and_then(|exe| wanted(agent, exe));
    let local = parent_dir(folders, agent);
    clean_staging(&local, agent, want.as_ref(), log);
    let dir = local.join(FOLDER);
    let shown_dir = shown_folder(agent);
    match kind(&dir)? {
        Kind::Missing => return Ok(()),
        Kind::Folder => {}
        Kind::Link => {
            blocked(agent, &shown_dir, log);
            return Ok(());
        }
        Kind::File | Kind::Other => {
            not_ours(agent, log);
            return Ok(());
        }
    }
    let look = look(&dir, agent, want.as_ref(), false)?;
    if look.marker.is_none() {
        not_ours(agent, log);
        return Ok(());
    }
    let emptied = delete_ours(&dir, agent, &look);
    let why = emptied.failed.map_or_else(
        || "it holds files Wakeshot did not write".to_owned(),
        |e| format!("a delete failed: {}", failed(e)),
    );
    let (level, text) = if emptied.gone {
        (Level::Info, format!("{}: {shown_dir} removed", agent.label))
    } else {
        match move_aside(&dir, &local) {
            Ok(name) => (
                Level::Info,
                format!(
                    "{}: {shown_dir} renamed {name}, so the agent stops loading it ({why})",
                    agent.label
                ),
            ),
            Err(e) => (
                Level::Warn,
                format!(
                    "{}: {shown_dir} could not be removed ({why}) or renamed ({}); delete it yourself",
                    agent.label,
                    failed(e)
                ),
            ),
        }
    };
    line(log, level, &text);
    Ok(())
}

/// What [`delete_ours`] got done.
#[derive(Debug)]
struct Emptied {
    /// The folder is gone.
    gone: bool,
    /// The first delete that failed.
    failed: Option<WinError>,
}

/// Deletes what `look` found to be Wakeshot's in `dir`, by the fixed names only: its leftover temp files, each fixed
/// file that is Wakeshot's, the marker once no fixed file is left (while one is, it still tells what Wakeshot wrote),
/// then each fixed subfolder and `dir` when empty. Never recursive, never through a reparse point, never a name read
/// from a file.
fn delete_ours(dir: &Path, agent: &Agent, look: &Look) -> Emptied {
    let mut failed = None;
    let mut done = |result: Result<bool, WinError>| match result {
        Ok(gone) => gone,
        Err(e) => {
            failed.get_or_insert(e);
            false
        }
    };
    let mut left = false;
    for temp in &look.leftovers {
        left |= !done(remove_file(temp).map(|()| true));
    }
    for (fixed, seen) in agent.files.iter().zip(&look.files) {
        left |= match seen {
            Seen::Ours => !done(remove_file(&file_path(dir, fixed)).map(|()| true)),
            Seen::Missing => false,
            Seen::Other => true,
        };
    }
    if !left && look.marker_seen == Seen::Ours {
        done(remove_file(&dir.join(MARKER)).map(|()| true));
    }
    for folder in agent.folders.iter().rev() {
        let path = dir.join(folder);
        if kind(&path).is_ok_and(|kind| kind == Kind::Folder) {
            done(remove_empty_dir(&path));
        }
    }
    let gone = done(remove_empty_dir(dir));
    Emptied { gone, failed }
}

/// A plain folder only (the caller checked). `Ok(false)`: it is not empty; `RemoveDirectoryW` never deletes what is in
/// it.
fn remove_empty_dir(path: &Path) -> Result<bool, WinError> {
    match fs::remove_dir(path) {
        Ok(()) => Ok(true),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(true),
        Err(e) if e.raw_os_error() == Some(ERROR_DIR_NOT_EMPTY.0 as i32) => Ok(false),
        Err(e) => Err(io_error("RemoveDirectoryW", &e)),
    }
}

/// `DeleteFileW` deletes a link itself, never its target.
fn remove_file(path: &Path) -> Result<(), WinError> {
    match fs::remove_file(path) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        other => other.map_err(|e| io_error("DeleteFileW", &e)),
    }
}

/// The names a folder the uninstall cannot empty is renamed to, in order: [`REMOVED`], then `-2` to `-9` after it, so
/// a second uninstall never meets a name an earlier one took (review m4). All dot names, which the agent skips.
fn removed_names() -> impl Iterator<Item = String> {
    iter::once(REMOVED.to_owned()).chain((2..=9).map(|n| format!("{REMOVED}-{n}")))
}

/// Renames `dir` in `local` to the first free name of [`removed_names`], never over a folder that is there.
fn move_aside(dir: &Path, local: &Path) -> Result<String, WinError> {
    let mut taken = None;
    for name in removed_names() {
        match move_folder(dir, &local.join(&name)) {
            Ok(()) => return Ok(name),
            Err(e) if e.code == ERROR_ALREADY_EXISTS.0 || e.code == ERROR_FILE_EXISTS.0 => {
                taken = Some(e);
            }
            Err(e) => return Err(e),
        }
    }
    Err(taken.unwrap_or(WinError {
        call: "MoveFileExW",
        code: ERROR_ALREADY_EXISTS.0,
    }))
}

/// A rename that fails when `to` exists: the folder the user may have left there is never replaced.
fn move_folder(from: &Path, to: &Path) -> Result<(), WinError> {
    let (from, to) = (wide(from), wide(to));
    // SAFETY: both names are NUL-terminated and live for the call.
    unsafe {
        MoveFileExW(
            PCWSTR(from.as_ptr()),
            PCWSTR(to.as_ptr()),
            MOVEFILE_WRITE_THROUGH,
        )
    }
    .map_err(|e| core_error("MoveFileExW", &e))
}

/// One fixed file as found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Seen {
    Missing,
    /// Wakeshot's: it hashes to the marker's value or holds what this version writes (in a staging folder also the
    /// start of it).
    Ours,
    /// Other bytes, too big, or not a plain file.
    Other,
}

/// A plugin or staging folder as found, by the fixed names only, never through a reparse point.
#[derive(Debug)]
struct Look {
    /// The marker's hashes in `Agent::files` order, when it parses as version 1 with exactly the fixed keys.
    marker: Option<Vec<u64>>,
    marker_seen: Seen,
    files: Vec<Seen>,
    /// A name that is not one of the fixed set, or a fixed subfolder that is not a plain folder.
    extra: bool,
    /// `store::write_atomic` temp files of the fixed files, which a stop during a rewrite leaves.
    leftovers: Vec<PathBuf>,
    /// Every fixed file and the marker hold exactly what this version writes, and nothing else is there.
    current: bool,
}

impl Look {
    /// Wakeshot's and as it left it: the marker parses, the folder holds the fixed set and nothing more, and each
    /// fixed file is Wakeshot's.
    fn unchanged(&self) -> bool {
        self.marker.is_some() && !self.extra && self.files.iter().all(|seen| *seen == Seen::Ours)
    }
}

/// `partial`: a staging folder, where a missing file or the start of one is a stop during the write.
fn look(dir: &Path, agent: &Agent, want: Option<&Wanted>, partial: bool) -> Result<Look, WinError> {
    let mut look = Look {
        marker: None,
        marker_seen: Seen::Missing,
        files: vec![Seen::Missing; agent.files.len()],
        extra: false,
        leftovers: Vec::new(),
        current: false,
    };
    let in_root = |name: &str| {
        name.eq_ignore_ascii_case(MARKER)
            || agent
                .files
                .iter()
                .any(|f| f.folder.is_none() && name.eq_ignore_ascii_case(f.name))
    };
    let mut plain = Vec::new();
    for name in list(dir)? {
        let Some(name) = name.to_str() else {
            look.extra = true;
            continue;
        };
        if in_root(name) {
            continue;
        }
        if let Some(folder) = agent.folders.iter().find(|f| name.eq_ignore_ascii_case(f)) {
            let path = dir.join(folder);
            if kind(&path)? != Kind::Folder {
                look.extra = true;
                continue;
            }
            plain.push(*folder);
            let fixed = |n: &str| {
                agent
                    .files
                    .iter()
                    .any(|f| f.folder == Some(*folder) && n.eq_ignore_ascii_case(f.name))
            };
            for inner in list(&path)? {
                match inner.to_str() {
                    Some(n) if fixed(n) => {}
                    Some(n) if store::temp_target(n).is_some_and(fixed) => {
                        leftover(&mut look, path.join(n))?;
                    }
                    _ => look.extra = true,
                }
            }
            continue;
        }
        if store::temp_target(name).is_some_and(in_root) {
            leftover(&mut look, dir.join(name))?;
            continue;
        }
        look.extra = true;
    }

    let mut current = false;
    let marker = dir.join(MARKER);
    match kind(&marker)? {
        Kind::Missing => {}
        Kind::File => match read_file(&marker)? {
            Some(bytes) => {
                look.marker = parse_marker(&bytes, agent);
                let start = partial && want.is_some_and(|w| w.marker.starts_with(&bytes));
                look.marker_seen = if look.marker.is_some() || start {
                    Seen::Ours
                } else {
                    Seen::Other
                };
                current = want.is_some_and(|w| w.marker == bytes);
            }
            None => look.marker_seen = Seen::Other,
        },
        _ => look.marker_seen = Seen::Other,
    }
    for (i, fixed) in agent.files.iter().enumerate() {
        let wanted = want.map(|w| w.files[i].as_slice());
        let seen = match fixed.folder {
            // Never a look inside a subfolder that is not plain: it may lead anywhere.
            Some(folder) if !plain.contains(&folder) => match kind(&dir.join(folder))? {
                Kind::Missing => Seen::Missing,
                _ => Seen::Other,
            },
            _ => {
                let path = file_path(dir, fixed);
                match kind(&path)? {
                    Kind::Missing => Seen::Missing,
                    Kind::File => match read_file(&path)? {
                        Some(bytes) => {
                            let is_current = wanted == Some(bytes.as_slice());
                            current &= is_current;
                            let hashed = look
                                .marker
                                .as_ref()
                                .is_some_and(|hashes| hashes[i] == fnv1a(&bytes));
                            let start = partial && wanted.is_some_and(|w| w.starts_with(&bytes));
                            if hashed || is_current || start {
                                Seen::Ours
                            } else {
                                Seen::Other
                            }
                        }
                        None => Seen::Other,
                    },
                    _ => Seen::Other,
                }
            }
        };
        if seen != Seen::Ours {
            current = false;
        }
        look.files[i] = seen;
    }
    look.current = current && !look.extra && look.leftovers.is_empty();
    Ok(look)
}

/// A temp file of `store::write_atomic` for a fixed file; anything but a plain file at that name is an extra.
fn leftover(look: &mut Look, path: PathBuf) -> Result<(), WinError> {
    if kind(&path)? == Kind::File {
        look.leftovers.push(path);
    } else {
        look.extra = true;
    }
    Ok(())
}

/// What this version writes for one agent: each fixed file, then the marker with their hashes.
#[derive(Debug)]
struct Wanted {
    files: Vec<Vec<u8>>,
    marker: Vec<u8>,
}

fn wanted(agent: &Agent, exe: &Path) -> Option<Wanted> {
    let files = (agent.contents)(exe).filter(|files| files.len() == agent.files.len())?;
    let marker = marker_bytes(agent, &files, VERSION);
    Some(Wanted { files, marker })
}

/// `{"version":1,"wakeshot":"<version>","files":{"<key>":"<FNV-1a 64>",...}}`: hashes by fixed key, never a path.
fn marker_bytes(agent: &Agent, files: &[Vec<u8>], version: &str) -> Vec<u8> {
    let hashes: Vec<String> = agent
        .files
        .iter()
        .zip(files)
        .map(|(fixed, bytes)| format!("{}:\"{:016x}\"", json_string(fixed.key), fnv1a(bytes)))
        .collect();
    format!(
        "{{\"version\":1,\"wakeshot\":{},\"files\":{{{}}}}}\n",
        json_string(version),
        hashes.join(",")
    )
    .into_bytes()
}

/// The hashes in `agent.files` order. `None` for any other shape: another version, key or value type, or an extra
/// key at either level.
fn parse_marker(bytes: &[u8], agent: &Agent) -> Option<Vec<u64>> {
    let Value::Object(top) = serde_json::from_slice(bytes).ok()? else {
        return None;
    };
    let version = top.get("wakeshot")?.as_str()?;
    if top.len() != 3 || top.get("version")?.as_u64()? != 1 || !(1..=64).contains(&version.len()) {
        return None;
    }
    let Value::Object(files) = top.get("files")? else {
        return None;
    };
    if files.len() != agent.files.len() {
        return None;
    }
    agent
        .files
        .iter()
        .map(|fixed| hex64(files.get(fixed.key)?.as_str()?))
        .collect()
}

fn hex64(text: &str) -> Option<u64> {
    let hex = text.len() == 16
        && text
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
    hex.then(|| u64::from_str_radix(text, 16).ok()).flatten()
}

/// FNV-1a 64. Not a secret: it finds edits, not attacks; the fixed file set and the Medium rule stop those.
fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, &b| {
        (hash ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// One agent's mark in the record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mark {
    Added,
    /// The folder was gone at a start after Wakeshot added it.
    Removed,
}

/// `agent-hooks.json` without its version: each agent's mark, also of agents this version does not know, which it
/// keeps as they are.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Marks(BTreeMap<String, Mark>);

/// A missing record is an empty one; `Err` says why it does not read.
fn read_record(path: &Path) -> Result<Marks, String> {
    let bytes = match store::read_whole(path) {
        Ok(None) => return Ok(Marks::default()),
        Ok(Some(bytes)) => bytes,
        Err(e) => return Err(format!("does not read ({})", failed(e))),
    };
    let fail = || "is not version 1 of its format".to_owned();
    let Ok(Value::Object(top)) = serde_json::from_slice::<Value>(&bytes) else {
        return Err(fail());
    };
    if top.get("version").and_then(Value::as_u64) != Some(1) {
        return Err(fail());
    }
    let mut marks = Marks::default();
    for (key, value) in top.iter().filter(|(key, _)| *key != "version") {
        let mark = match value.as_str() {
            Some("added") => Mark::Added,
            Some("removed") => Mark::Removed,
            _ => return Err(fail()),
        };
        marks.0.insert(key.clone(), mark);
    }
    Ok(marks)
}

fn record_bytes(marks: &Marks) -> Vec<u8> {
    let mut text = String::from("{\"version\":1");
    for (key, mark) in &marks.0 {
        let mark = match mark {
            Mark::Added => "added",
            Mark::Removed => "removed",
        };
        text.push_str(&format!(",{}:\"{mark}\"", json_string(key)));
    }
    text.push_str("}\n");
    text.into_bytes()
}

/// One part of a path, from its own entry: a reparse point is never followed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Missing,
    Folder,
    File,
    /// A reparse point of any kind: a junction, a link, a cloud file.
    Link,
    Other,
}

fn kind(path: &Path) -> Result<Kind, WinError> {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0 => Ok(Kind::Link),
        Ok(meta) if meta.is_dir() => Ok(Kind::Folder),
        Ok(meta) if meta.is_file() => Ok(Kind::File),
        Ok(_) => Ok(Kind::Other),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Kind::Missing),
        Err(e) => Err(io_error("GetFileAttributesExW", &e)),
    }
}

/// Where the parents of an agent's plugin folder stand.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Chain {
    /// A folder the agent makes is missing: the agent is not on this PC.
    Absent,
    /// Plain folders up to `made`, the first missing part; Wakeshot may make the rest.
    Plain { made: usize },
    /// `parents[at]` is not a plain folder.
    Blocked { at: usize },
}

fn walk(folders: &Folders, agent: &Agent) -> Result<Chain, WinError> {
    let mut path = folders.profile.clone();
    for (i, part) in agent.parents.iter().enumerate() {
        path.push(part);
        match kind(&path)? {
            Kind::Folder => {}
            Kind::Missing if i < agent.agent_made => return Ok(Chain::Absent),
            Kind::Missing => return Ok(Chain::Plain { made: i }),
            _ => return Ok(Chain::Blocked { at: i }),
        }
    }
    Ok(Chain::Plain {
        made: agent.parents.len(),
    })
}

fn parent_dir(folders: &Folders, agent: &Agent) -> PathBuf {
    agent
        .parents
        .iter()
        .fold(folders.profile.clone(), |path, part| path.join(part))
}

fn file_path(dir: &Path, fixed: &FixedFile) -> PathBuf {
    match fixed.folder {
        Some(folder) => dir.join(folder).join(fixed.name),
        None => dir.join(fixed.name),
    }
}

/// The whole file, never through a reparse point; `None` when it is bigger than [`FILE_MAX`] or not a plain file.
fn read_file(path: &Path) -> Result<Option<Vec<u8>>, WinError> {
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT.0)
        .open(path)
        .map_err(|e| io_error("CreateFileW", &e))?;
    let meta = file
        .metadata()
        .map_err(|e| io_error("GetFileInformationByHandle", &e))?;
    if meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0
        || !meta.is_file()
        || meta.len() > FILE_MAX
    {
        return Ok(None);
    }
    let mut bytes = Vec::new();
    file.take(FILE_MAX + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| io_error("ReadFile", &e))?;
    Ok((bytes.len() as u64 <= FILE_MAX).then_some(bytes))
}

fn list(dir: &Path) -> Result<Vec<OsString>, WinError> {
    let entries = fs::read_dir(dir).map_err(|e| io_error("FindFirstFileExW", &e))?;
    entries
        .map(|entry| {
            entry
                .map(|entry| entry.file_name())
                .map_err(|e| io_error("FindNextFileW", &e))
        })
        .collect()
}

/// `Ok(false)` of `process::current_above_medium`, else one log line.
fn medium(above_medium: Result<bool, WinError>, what: &str, log: &dyn Log) -> bool {
    let text = match above_medium {
        Ok(false) => return true,
        Ok(true) => format!(
            "this process runs above Medium integrity (started from an admin terminal), so it {what}"
        ),
        Err(e) => format!(
            "this process's integrity level does not read ({}), so it {what}",
            failed(e)
        ),
    };
    line(log, Level::Info, &text);
    false
}

/// A path from the profile folder, for the log: never the user's name.
fn shown(parts: &[&str]) -> String {
    parts.join("\\")
}

fn shown_folder(agent: &Agent) -> String {
    format!("{}\\{FOLDER}", shown(agent.parents))
}

fn blocked(agent: &Agent, shown_part: &str, log: &dyn Log) {
    let text = format!(
        "{}: {shown_part} is a link, a junction or not a plain folder, so Wakeshot touches nothing there",
        agent.label
    );
    line(log, Level::Warn, &text);
}

fn not_ours(agent: &Agent, log: &dyn Log) {
    let text = format!(
        "{}: {} was not written by Wakeshot, so Wakeshot leaves it as it is",
        agent.label,
        shown_folder(agent)
    );
    line(log, Level::Info, &text);
}

fn line(log: &dyn Log, level: Level, text: &str) {
    log.line(level, AREA, text);
}

fn failed(e: WinError) -> String {
    format!("{} failed with {}", e.call, e.code)
}

fn wide(path: &Path) -> Vec<u16> {
    path.as_os_str()
        .encode_wide()
        .chain(iter::once(0))
        .collect()
}

/// A std error without an OS code gets `u32::MAX`, which no Win32 error uses.
fn io_error(call: &'static str, e: &io::Error) -> WinError {
    WinError {
        call,
        code: e.raw_os_error().map_or(u32::MAX, |code| code as u32),
    }
}

/// Win32 errors stay Win32 codes (2, 5, 32); other failures keep the HRESULT.
fn core_error(call: &'static str, e: &windows::core::Error) -> WinError {
    let hresult = e.code().0 as u32;
    let code = if hresult & 0xFFFF_0000 == 0x8007_0000 {
        hresult & 0xFFFF
    } else {
        hresult
    };
    WinError { call, code }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::process::{Command, Stdio};
    use std::sync::atomic::{AtomicU64, Ordering};

    use windows::Win32::Storage::FileSystem::FILE_SHARE_READ;

    use super::*;
    use crate::log::MemoryLog;

    /// A fresh folder under `%TEMP%`, removed with everything in it when the test ends. `remove_dir_all` removes a
    /// junction without going through it.
    struct Temp(PathBuf);

    impl Temp {
        fn new(tag: &str) -> Self {
            static N: AtomicU64 = AtomicU64::new(0);
            let n = N.fetch_add(1, Ordering::Relaxed);
            let dir = std::env::temp_dir().join(format!(
                "wakeshot-agent-hooks-{tag}-{}-{n}",
                std::process::id()
            ));
            fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }
    }

    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    /// One PC under one temp folder: a profile with `.cursor`, an install folder with `wakeshot.exe`, a data folder,
    /// and `outside`, a folder no agent folder work may ever change.
    struct World {
        temp: Temp,
        folders: Folders,
        record: PathBuf,
        exe: PathBuf,
        log: MemoryLog,
    }

    impl World {
        fn new(tag: &str) -> Self {
            let temp = Temp::new(tag);
            let folders = Folders {
                profile: temp.0.join("profile"),
                local_app_data: temp.0.join("local"),
            };
            let exe = folders
                .local_app_data
                .join(r"HaikAsatryan.Wakeshot\current\wakeshot.exe");
            fs::create_dir_all(exe.parent().unwrap()).unwrap();
            fs::write(&exe, b"MZ").unwrap();
            fs::create_dir_all(folders.profile.join(".cursor")).unwrap();
            fs::create_dir_all(temp.0.join("outside")).unwrap();
            let record = temp.0.join(r"Wakeshot\agent-hooks.json");
            Self {
                temp,
                folders,
                record,
                exe,
                log: MemoryLog::default(),
            }
        }

        fn start(&self) -> Vec<&'static str> {
            at_start(&self.folders, &self.record, &self.exe, Ok(false), &self.log)
        }

        fn uninstall(&self) {
            let mut copy =
                |_: &Path, _: &[&'static Agent]| -> Vec<CopyRun> { panic!("no copy at Medium") };
            on_uninstall(
                &self.folders,
                Some(&self.exe),
                Ok(false),
                &mut copy,
                &self.log,
            );
        }

        fn local(&self) -> PathBuf {
            self.folders.profile.join(r".cursor\plugins\local")
        }

        fn plugin(&self) -> PathBuf {
            self.local().join(FOLDER)
        }

        fn outside(&self) -> PathBuf {
            self.temp.0.join("outside")
        }

        fn want(&self) -> Wanted {
            wanted(&CURSOR, &self.exe).unwrap()
        }

        /// What an older version wrote: other hooks, so a start rewrites them if the folder is unchanged.
        fn older(&self) -> Vec<Vec<u8>> {
            let mut files = self.want().files;
            files[1] = b"{\"version\":1,\"hooks\":{\"stop\":[]}}\n".to_vec();
            files
        }

        fn record_text(&self) -> Option<String> {
            fs::read_to_string(&self.record).ok()
        }

        fn has(&self, part: &str) -> bool {
            let lines = self.log.lines.lock().unwrap();
            lines
                .iter()
                .any(|(_, area, text)| *area == AREA && text.contains(part))
        }

        fn lines(&self) -> Vec<String> {
            let lines = self.log.lines.lock().unwrap();
            lines.iter().map(|(_, _, text)| text.clone()).collect()
        }
    }

    /// Everything under `root` by its path from there: a folder as `/`, a reparse point as `->`, a file as its text.
    /// Never through a reparse point.
    fn tree(root: &Path) -> BTreeMap<String, String> {
        fn walk(base: &Path, dir: &Path, out: &mut BTreeMap<String, String>) {
            let Ok(entries) = fs::read_dir(dir) else {
                return;
            };
            for entry in entries {
                let path = entry.unwrap().path();
                let name = path.strip_prefix(base).unwrap().display().to_string();
                let meta = fs::symlink_metadata(&path).unwrap();
                if meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0 {
                    out.insert(name, "->".to_owned());
                } else if meta.is_dir() {
                    out.insert(name, "/".to_owned());
                    walk(base, &path, out);
                } else {
                    let text = String::from_utf8_lossy(&fs::read(&path).unwrap()).into_owned();
                    out.insert(name, text);
                }
            }
        }
        let mut out = BTreeMap::new();
        walk(root, root, &mut out);
        out
    }

    fn names(root: &Path) -> Vec<String> {
        tree(root).into_keys().collect()
    }

    /// `mklink /J`: a junction needs no privilege, so it is what any Medium process can plant.
    fn junction(link: &Path, target: &Path) {
        let made = Command::new("cmd")
            .args(["/c", "mklink", "/J"])
            .arg(link)
            .arg(target)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap()
            .success();
        assert!(made, "mklink /J {}", link.display());
    }

    /// A plugin folder as Wakeshot writes it, with `files` and a marker of `version` that names their hashes.
    fn plant(dir: &Path, files: &[Vec<u8>], version: &str) {
        for folder in CURSOR.folders {
            fs::create_dir_all(dir.join(folder)).unwrap();
        }
        for (fixed, bytes) in CURSOR.files.iter().zip(files) {
            fs::write(file_path(dir, fixed), bytes).unwrap();
        }
        fs::write(dir.join(MARKER), marker_bytes(&CURSOR, files, version)).unwrap();
    }

    const FIXED_SET: [&str; 6] = [
        "wakeshot",
        r"wakeshot\.cursor-plugin",
        r"wakeshot\.cursor-plugin\plugin.json",
        r"wakeshot\hooks",
        r"wakeshot\hooks\hooks.json",
        r"wakeshot\wakeshot-hooks.json",
    ];

    #[test]
    fn a_fresh_start_adds_the_folder_once_and_a_second_adds_nothing() {
        let w = World::new("fresh");
        assert_eq!(w.start(), ["Cursor"]);
        let want = w.want();
        let dir = w.plugin();
        assert_eq!(
            names(&w.local()),
            FIXED_SET,
            "the fixed set, no staging folder"
        );
        assert_eq!(
            fs::read(dir.join(r".cursor-plugin\plugin.json")).unwrap(),
            want.files[0]
        );
        assert_eq!(
            fs::read(dir.join(r"hooks\hooks.json")).unwrap(),
            want.files[1]
        );
        assert_eq!(fs::read(dir.join(MARKER)).unwrap(), want.marker);

        let hooks: Value = serde_json::from_slice(&want.files[1]).unwrap();
        assert_eq!(hooks["version"], 1);
        assert_eq!(hooks["hooks"].as_object().unwrap().len(), 3);
        for event in CURSOR_EVENTS {
            let run = &hooks["hooks"][event];
            assert_eq!(run.as_array().unwrap().len(), 1);
            let command = format!("'{}' hook cursor {event}", w.exe.display());
            assert_eq!(run[0]["command"], command.as_str());
            assert_eq!(run[0]["timeout"], 5);
            assert_eq!(
                run[0].as_object().unwrap().len(),
                2,
                "no failClosed or matcher"
            );
        }
        let plugin: Value = serde_json::from_slice(&want.files[0]).unwrap();
        assert_eq!(plugin["name"], "wakeshot");
        let marker: Value = serde_json::from_slice(&want.marker).unwrap();
        assert_eq!(marker["wakeshot"], VERSION);
        assert_eq!(
            parse_marker(&want.marker, &CURSOR),
            Some(vec![fnv1a(&want.files[0]), fnv1a(&want.files[1])])
        );
        assert_eq!(
            w.record_text().as_deref(),
            Some("{\"version\":1,\"cursor\":\"added\"}\n")
        );
        assert!(w.has(r"Cursor: hook folder added (.cursor\plugins\local\wakeshot)"));
        assert!(
            !w.lines()
                .iter()
                .any(|l| l.contains(&*w.temp.0.to_string_lossy()))
        );

        let before = tree(&w.temp.0);
        assert!(w.start().is_empty());
        assert_eq!(tree(&w.temp.0), before, "a second start writes nothing");
    }

    #[test]
    fn without_cursors_folder_nothing_is_written() {
        let w = World::new("absent");
        fs::remove_dir(w.folders.profile.join(".cursor")).unwrap();
        let before = tree(&w.temp.0);
        assert!(w.start().is_empty());
        w.uninstall();
        assert_eq!(tree(&w.temp.0), before);
    }

    #[test]
    fn a_folder_the_user_removed_is_never_added_back() {
        let w = World::new("removed");
        assert_eq!(w.start(), ["Cursor"]);
        fs::remove_dir_all(w.plugin()).unwrap();
        assert!(w.start().is_empty());
        assert_eq!(
            w.record_text().as_deref(),
            Some("{\"version\":1,\"cursor\":\"removed\"}\n")
        );
        assert!(w.has("is gone, so the user removed it"));
        for _ in 0..2 {
            assert!(w.start().is_empty());
        }
        assert!(!w.plugin().exists());

        // The way back the README names: the record deleted.
        fs::remove_file(&w.record).unwrap();
        assert_eq!(w.start(), ["Cursor"]);
    }

    #[test]
    fn a_folder_the_user_changed_is_never_touched() {
        type Edit = fn(&Path);
        let edits: [(&str, Edit); 7] = [
            ("an edited hooks.json", |dir| {
                fs::write(dir.join(r"hooks\hooks.json"), b"{\"version\":1}").unwrap();
            }),
            ("a deleted hooks.json", |dir| {
                fs::remove_file(dir.join(r"hooks\hooks.json")).unwrap();
            }),
            ("an added mcp.json", |dir| {
                fs::write(dir.join("mcp.json"), b"{}").unwrap();
            }),
            ("an added rules folder", |dir| {
                fs::create_dir(dir.join("rules")).unwrap();
                fs::write(dir.join(r"rules\a.md"), b"x").unwrap();
            }),
            ("an added file in hooks", |dir| {
                fs::write(dir.join(r"hooks\more.json"), b"{}").unwrap();
            }),
            ("an edited marker", |dir| {
                fs::write(dir.join(MARKER), b"{\"version\":1}").unwrap();
            }),
            ("a folder for a fixed file", |dir| {
                fs::remove_file(dir.join(r"hooks\hooks.json")).unwrap();
                fs::create_dir(dir.join(r"hooks\hooks.json")).unwrap();
            }),
        ];
        for (what, edit) in edits {
            let w = World::new("changed");
            fs::create_dir_all(w.local()).unwrap();
            plant(&w.plugin(), &w.older(), "0.0.1");
            edit(&w.plugin());
            let before = tree(&w.plugin());
            assert!(w.start().is_empty(), "{what}");
            assert_eq!(tree(&w.plugin()), before, "{what}");
            let said = w.has("was changed since Wakeshot wrote it, so Wakeshot leaves it")
                || w.has("was not written by Wakeshot");
            assert!(said, "{what}: {:?}", w.lines());
        }
    }

    #[test]
    fn older_content_is_rewritten_and_shows_no_notice() {
        let w = World::new("older");
        fs::create_dir_all(w.local()).unwrap();
        plant(&w.plugin(), &w.older(), "0.0.1");
        assert!(w.start().is_empty());
        let want = w.want();
        assert_eq!(names(&w.local()), FIXED_SET);
        assert_eq!(
            fs::read(w.plugin().join(r"hooks\hooks.json")).unwrap(),
            want.files[1]
        );
        assert_eq!(fs::read(w.plugin().join(MARKER)).unwrap(), want.marker);
        assert!(w.has("rewritten for this version"));
        assert_eq!(
            w.record_text().as_deref(),
            Some("{\"version\":1,\"cursor\":\"added\"}\n")
        );
    }

    #[test]
    fn a_stop_during_a_rewrite_leaves_the_folder_wakeshots() {
        let w = World::new("stopped-rewrite");
        fs::create_dir_all(w.local()).unwrap();
        let want = w.want();
        // The old marker still names the old hooks, but `hooks.json` is this version's already, and the temp file of
        // the marker's write is left.
        plant(&w.plugin(), &w.older(), "0.0.1");
        fs::write(w.plugin().join(r"hooks\hooks.json"), &want.files[1]).unwrap();
        let temp = w
            .plugin()
            .join(format!("{MARKER}.4242.0.0123456789abcdef.tmp"));
        fs::write(&temp, b"{").unwrap();
        assert!(w.start().is_empty());
        assert_eq!(names(&w.local()), FIXED_SET, "the temp file is gone");
        assert_eq!(fs::read(w.plugin().join(MARKER)).unwrap(), want.marker);
    }

    #[test]
    fn a_folder_wakeshot_did_not_write_is_never_touched() {
        let hash = |bytes: &[u8]| format!("{:016x}", fnv1a(bytes));
        let w = World::new("not-ours");
        let want = w.want();
        let (plugin, hooks) = (hash(&want.files[0]), hash(&want.files[1]));
        let markers = [
            None,
            Some(format!(
                r#"{{"version":2,"wakeshot":"0.1.0","files":{{"plugin":"{plugin}","hooks":"{hooks}"}}}}"#
            )),
            Some(format!(
                r#"{{"version":1,"wakeshot":"0.1.0","files":{{"plugin":"{plugin}","hooks":"{hooks}"}},"more":1}}"#
            )),
            Some(format!(
                r#"{{"version":1,"wakeshot":"0.1.0","files":{{"plugin":"{plugin}"}}}}"#
            )),
            Some(format!(
                r#"{{"version":1,"wakeshot":"0.1.0","files":{{"plugin":"{plugin}","hooks":"{}"}}}}"#,
                hooks.to_uppercase()
            )),
            Some(format!(
                r#"{{"version":1,"wakeshot":"","files":{{"plugin":"{plugin}","hooks":"{hooks}"}}}}"#
            )),
        ];
        for marker in markers {
            let w = World::new("not-ours");
            fs::create_dir_all(w.local()).unwrap();
            plant(&w.plugin(), &w.older(), "0.0.1");
            match &marker {
                Some(text) => fs::write(w.plugin().join(MARKER), text).unwrap(),
                None => fs::remove_file(w.plugin().join(MARKER)).unwrap(),
            }
            let before = tree(&w.temp.0);
            assert!(w.start().is_empty(), "{marker:?}");
            w.uninstall();
            assert_eq!(tree(&w.temp.0), before, "{marker:?}");
            assert!(w.has("was not written by Wakeshot"), "{marker:?}");
        }

        // A file where the folder goes.
        let w = World::new("not-ours-file");
        fs::create_dir_all(w.local()).unwrap();
        fs::write(w.plugin(), b"x").unwrap();
        let before = tree(&w.temp.0);
        assert!(w.start().is_empty());
        w.uninstall();
        assert_eq!(tree(&w.temp.0), before);
    }

    /// The parts a junction may stand in for, each checked on its own: the agent's folders above the plugin folder,
    /// the plugin folder, its subfolders, a staging folder, and a fixed file as a link or a hard link.
    #[test]
    fn a_link_anywhere_on_the_path_stops_every_write_and_delete() {
        let parts = [
            ".cursor",
            "plugins",
            "local",
            "wakeshot",
            ".cursor-plugin",
            "hooks",
            ".wakeshot-new-7",
            "hooks.json link",
            "hooks.json hard link",
        ];
        let parents = [".cursor", "plugins", "local"];
        for part in parts {
            let w = World::new("link");
            let out = w.outside();
            let (old, want) = (w.older(), w.want());
            let mut said = "is a link, a junction or not a plain folder";
            let mut adds = false;
            if let Some(depth) = parents.iter().position(|p| *p == part) {
                // What the junction leads to: a whole tree below it, older, with a staging folder.
                let target = out.join(part);
                let local = parents[depth + 1..]
                    .iter()
                    .fold(target.clone(), |path, p| path.join(p));
                plant(&local.join(FOLDER), &old, "0.0.1");
                plant(&local.join(".wakeshot-new-7"), &want.files, VERSION);
                let link = parents[..=depth]
                    .iter()
                    .fold(w.folders.profile.clone(), |path, p| path.join(p));
                let _ = fs::remove_dir(&link);
                fs::create_dir_all(link.parent().unwrap()).unwrap();
                junction(&link, &target);
            } else {
                fs::create_dir_all(w.local()).unwrap();
                match part {
                    "wakeshot" => {
                        plant(&out.join(FOLDER), &old, "0.0.1");
                        junction(&w.plugin(), &out.join(FOLDER));
                    }
                    ".cursor-plugin" | "hooks" => {
                        plant(&w.plugin(), &old, "0.0.1");
                        fs::rename(w.plugin().join(part), out.join(part)).unwrap();
                        junction(&w.plugin().join(part), &out.join(part));
                        said = "was changed since Wakeshot wrote it";
                    }
                    ".wakeshot-new-7" => {
                        plant(&out.join("staging"), &want.files, VERSION);
                        junction(&w.local().join(part), &out.join("staging"));
                        said = "holds more than Wakeshot's own files";
                        adds = true;
                    }
                    "hooks.json link" => {
                        plant(&w.plugin(), &old, "0.0.1");
                        let link = w.plugin().join(r"hooks\hooks.json");
                        fs::write(out.join("hooks.json"), &old[1]).unwrap();
                        fs::remove_file(&link).unwrap();
                        if let Err(e) =
                            std::os::windows::fs::symlink_file(out.join("hooks.json"), &link)
                        {
                            // A file link needs the create-link privilege or developer mode.
                            println!("skipped {part}: {e}");
                            continue;
                        }
                        said = "was changed since Wakeshot wrote it";
                    }
                    "hooks.json hard link" => {
                        // Same bytes as the marker names, so Wakeshot counts the name as its own: it may replace or
                        // delete the name, never write into the file another name shares.
                        plant(&w.plugin(), &old, "0.0.1");
                        let link = w.plugin().join(r"hooks\hooks.json");
                        fs::write(out.join("hooks.json"), &old[1]).unwrap();
                        fs::remove_file(&link).unwrap();
                        fs::hard_link(out.join("hooks.json"), &link).unwrap();
                        said = "rewritten for this version";
                    }
                    _ => unreachable!(),
                }
            }
            let before = tree(&out);
            let added = w.start();
            w.uninstall();
            assert_eq!(
                tree(&out),
                before,
                "{part}: nothing changed through the link"
            );
            assert_eq!(added.is_empty(), !adds, "{part}");
            assert!(w.has(said), "{part}: {:?}", w.lines());
        }
    }

    #[test]
    fn a_crafted_marker_or_record_deletes_nothing_outside_the_fixed_set() {
        // A marker that names another file by path, with its true hash: the folder is not Wakeshot's.
        let w = World::new("crafted");
        let victim = w.folders.profile.join("victim.txt");
        fs::write(&victim, b"mine").unwrap();
        let want = w.want();
        fs::create_dir_all(w.local()).unwrap();
        plant(&w.plugin(), &want.files, VERSION);
        let crafted = format!(
            r#"{{"version":1,"wakeshot":"0.1.0","files":{{"plugin":"{:016x}","hooks":"{:016x}","..\\..\\..\\..\\victim.txt":"{:016x}"}}}}"#,
            fnv1a(&want.files[0]),
            fnv1a(&want.files[1]),
            fnv1a(b"mine")
        );
        fs::write(w.plugin().join(MARKER), crafted).unwrap();
        let before = tree(&w.temp.0);
        assert!(w.start().is_empty());
        w.uninstall();
        assert_eq!(tree(&w.temp.0), before);

        // Wakeshot's marker, with files another program dropped in: only the fixed files go, the rest is renamed away.
        let w = World::new("crafted-extra");
        assert_eq!(w.start(), ["Cursor"]);
        fs::write(w.plugin().join("mcp.json"), b"{}").unwrap();
        fs::create_dir(w.plugin().join("rules")).unwrap();
        fs::write(w.plugin().join(r"rules\a.md"), b"x").unwrap();
        fs::write(w.plugin().join(r"hooks\other.json"), b"{}").unwrap();
        fs::write(w.folders.profile.join(r".cursor\hooks.json"), b"{}").unwrap();
        w.uninstall();
        assert_eq!(
            tree(&w.local()),
            BTreeMap::from(
                [
                    (REMOVED, "/"),
                    (r".wakeshot-removed\hooks", "/"),
                    (r".wakeshot-removed\hooks\other.json", "{}"),
                    (r".wakeshot-removed\mcp.json", "{}"),
                    (r".wakeshot-removed\rules", "/"),
                    (r".wakeshot-removed\rules\a.md", "x"),
                ]
                .map(|(k, v)| (k.to_owned(), v.to_owned()))
            )
        );
        assert!(w.folders.profile.join(r".cursor\hooks.json").is_file());

        // A record that is not version 1 of its format adds nothing and stays as it is.
        for record in [
            r#"{"version":2,"cursor":"added"}"#,
            r#"{"version":1,"cursor":"..\\x"}"#,
            r#"{"version":1,"cursor":1}"#,
            r#"{"cursor":"added"}"#,
            "not json",
        ] {
            let w = World::new("crafted-record");
            fs::create_dir_all(w.record.parent().unwrap()).unwrap();
            fs::write(&w.record, record).unwrap();
            assert!(w.start().is_empty(), "{record}");
            assert!(
                !w.folders.profile.join(r".cursor\plugins").exists(),
                "{record}"
            );
            assert_eq!(fs::read_to_string(&w.record).unwrap(), record);
            assert!(w.has("agent-hooks.json is not version 1"), "{record}");
        }
        // Another agent's mark is kept as it is.
        let w = World::new("record-other");
        fs::create_dir_all(w.record.parent().unwrap()).unwrap();
        fs::write(&w.record, r#"{"version":1,"zed":"removed"}"#).unwrap();
        assert_eq!(w.start(), ["Cursor"]);
        assert_eq!(
            w.record_text().as_deref(),
            Some("{\"version\":1,\"cursor\":\"added\",\"zed\":\"removed\"}\n")
        );
    }

    #[test]
    fn uninstall_removes_exactly_the_fixed_set() {
        let w = World::new("uninstall");
        assert_eq!(w.start(), ["Cursor"]);
        fs::write(w.folders.profile.join(r".cursor\hooks.json"), b"{}").unwrap();
        fs::create_dir_all(w.local().join("other")).unwrap();
        fs::write(w.local().join(r"other\plugin.json"), b"{}").unwrap();
        let mut before = tree(&w.temp.0);
        before.retain(|path, _| !path.starts_with(r"profile\.cursor\plugins\local\wakeshot"));
        w.uninstall();
        assert_eq!(tree(&w.temp.0), before);
        assert!(w.has(r"Cursor: .cursor\plugins\local\wakeshot removed"));

        // A second uninstall finds nothing and says nothing.
        let lines = w.lines().len();
        w.uninstall();
        assert_eq!(w.lines().len(), lines);
    }

    #[test]
    fn uninstall_renames_a_folder_it_cannot_empty() {
        let w = World::new("rename");
        assert_eq!(w.start(), ["Cursor"]);
        fs::write(w.plugin().join(r"hooks\hooks.json"), b"{\"mine\":1}").unwrap();
        w.uninstall();
        assert!(!w.plugin().exists());
        let removed = w.local().join(REMOVED);
        assert_eq!(names(&removed), ["hooks", r"hooks\hooks.json", MARKER]);
        assert_eq!(
            fs::read(removed.join(r"hooks\hooks.json")).unwrap(),
            b"{\"mine\":1}"
        );
        assert!(w.has(r".cursor\plugins\local\wakeshot renamed .wakeshot-removed"));

        // A later install adds a new folder; its uninstall takes the next free name, never the first one's (review m4),
        // so the plugin stops loading again.
        fs::remove_file(&w.record).unwrap();
        assert_eq!(w.start(), ["Cursor"]);
        fs::write(w.plugin().join("mcp.json"), b"{}").unwrap();
        w.uninstall();
        assert!(!w.plugin().exists());
        assert_eq!(names(&w.local().join(".wakeshot-removed-2")), ["mcp.json"]);
        assert_eq!(names(&removed), ["hooks", r"hooks\hooks.json", MARKER]);
        assert!(w.has(r".cursor\plugins\local\wakeshot renamed .wakeshot-removed-2"));

        // Every name taken: the folder stays, and the line says so.
        for n in 3..=9 {
            fs::create_dir(w.local().join(format!(".wakeshot-removed-{n}"))).unwrap();
        }
        fs::remove_file(&w.record).unwrap();
        assert_eq!(w.start(), ["Cursor"]);
        fs::write(w.plugin().join("mcp.json"), b"{}").unwrap();
        w.uninstall();
        assert_eq!(names(&w.plugin()), ["mcp.json"]);
        assert!(w.has("or renamed (MoveFileExW failed with 183); delete it yourself"));
    }

    #[test]
    fn the_copies_share_one_wait() {
        assert_eq!(wait_left(Duration::ZERO), COPY_WAIT_MS);
        assert_eq!(wait_left(Duration::from_millis(4_200)), 800);
        assert_eq!(wait_left(Duration::from_secs(9)), 0);
    }

    #[test]
    fn a_staging_folder_that_does_not_read_never_blocks_the_main_folder() {
        let w = World::new("staging-unread");
        assert_eq!(w.start(), ["Cursor"]);
        let staging = w.local().join(".wakeshot-new-77");
        fs::create_dir_all(staging.join("hooks")).unwrap();
        let held = staging.join(r"hooks\hooks.json");
        fs::write(&held, w.want().files[1].as_slice()).unwrap();
        // Shares nothing, so the look's read fails (32).
        let file = OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(&held)
            .unwrap();
        w.uninstall();
        assert!(!w.plugin().exists(), "the live plugin goes anyway");
        assert!(
            w.has(r".cursor\plugins\local\.wakeshot-new-77 does not read, so Wakeshot leaves it")
        );
        // The same at a start: the rewrite of the main folder is not skipped.
        fs::remove_file(&w.record).unwrap();
        assert_eq!(w.start(), ["Cursor"]);
        drop(file);
        assert!(staging.is_dir());
    }

    #[test]
    fn uninstall_leaves_a_file_held_open_and_says_so() {
        let w = World::new("held");
        assert_eq!(w.start(), ["Cursor"]);
        let held = w.plugin().join(r"hooks\hooks.json");
        // Opened as a reader that shares no delete.
        let file = OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ.0)
            .open(&held)
            .unwrap();
        w.uninstall();
        drop(file);
        // The delete fails on the sharing, and the rename of a folder with an open file in it fails too (tested
        // 2026-11-06 on build 26300).
        assert!(w.has(
            r"Cursor: .cursor\plugins\local\wakeshot could not be removed (a delete failed: DeleteFileW failed with 32) or renamed (MoveFileExW failed with 5); delete it yourself"
        ));
        assert_eq!(names(&w.plugin()), [r"hooks", r"hooks\hooks.json", MARKER]);
        assert_eq!(fs::read(&held).unwrap(), w.want().files[1]);
    }

    #[test]
    fn a_staging_folder_a_stop_left_is_removed_by_the_fixed_names() {
        let w = World::new("staging");
        let want = w.want();
        fs::create_dir_all(w.local()).unwrap();
        // A stop during the write: the first file whole, the second begun, no marker yet.
        let stopped = |w: &World| {
            let dir = w.local().join(".wakeshot-new-4242");
            fs::create_dir_all(dir.join(".cursor-plugin")).unwrap();
            fs::create_dir_all(dir.join("hooks")).unwrap();
            fs::write(dir.join(r".cursor-plugin\plugin.json"), &want.files[0]).unwrap();
            fs::write(dir.join(r"hooks\hooks.json"), &want.files[1][..20]).unwrap();
            dir
        };
        let first = stopped(&w);
        // One that holds a file more, and one with a changed file: both stay.
        let more = w.local().join(".wakeshot-new-4343");
        plant(&more, &want.files, VERSION);
        fs::write(more.join("rules.md"), b"x").unwrap();
        let changed = w.local().join(".WAKESHOT-NEW-4444");
        plant(&changed, &want.files, VERSION);
        fs::write(changed.join(r"hooks\hooks.json"), b"{}").unwrap();
        let kept = (tree(&more), tree(&changed));

        assert_eq!(w.start(), ["Cursor"]);
        assert!(!first.exists());
        assert_eq!((tree(&more), tree(&changed)), kept);
        assert!(w.has(r"removed .cursor\plugins\local\.wakeshot-new-4242, which a stop left"));
        assert!(
            w.has(r".cursor\plugins\local\.wakeshot-new-4343 holds more than Wakeshot's own files")
        );
        assert!(
            w.has(r".cursor\plugins\local\.WAKESHOT-NEW-4444 holds more than Wakeshot's own files")
        );

        // The uninstall runs the same rule.
        let second = stopped(&w);
        w.uninstall();
        assert!(!second.exists());
        assert!(!w.plugin().exists());
        assert_eq!((tree(&more), tree(&changed)), kept);
    }

    #[test]
    fn the_add_never_fills_a_staging_folder_someone_else_made() {
        let w = World::new("taken");
        fs::create_dir_all(w.local()).unwrap();
        let taken = w
            .local()
            .join(format!("{STAGING_PREFIX}{}", std::process::id()));
        fs::create_dir(&taken).unwrap();
        fs::write(taken.join("x.txt"), b"x").unwrap();
        assert!(w.start().is_empty());
        assert_eq!(names(&taken), ["x.txt"]);
        assert!(!w.plugin().exists());
        assert!(w.has("Cursor: hook folder work stopped: CreateDirectoryW failed with 183"));
        assert_eq!(
            w.record_text(),
            None,
            "a failed add is tried again at the next start"
        );
    }

    #[test]
    fn above_medium_or_unknown_nothing_is_touched_and_the_uninstall_hands_off() {
        let w = World::new("elevated");
        let before = tree(&w.temp.0);
        let unknown = WinError {
            call: "OpenProcessToken",
            code: 5,
        };
        for level in [Ok(true), Err(unknown)] {
            assert!(at_start(&w.folders, &w.record, &w.exe, level, &w.log).is_empty());
            assert_eq!(tree(&w.temp.0), before);
        }
        assert!(
            w.has("runs above Medium integrity (started from an admin terminal), so it touches no")
        );
        assert!(w.has("integrity level does not read (OpenProcessToken failed with 5)"));

        // A folder from an earlier start at Medium: an elevated uninstall never deletes in this process.
        assert_eq!(w.start(), ["Cursor"]);
        let before = tree(&w.temp.0);
        let mut asked = Vec::new();
        let answers = [
            CopyRun::Ended { outside_tree: true },
            CopyRun::Ended {
                outside_tree: false,
            },
            CopyRun::StillRuns { outside_tree: true },
            CopyRun::Failed(unknown),
        ];
        for answer in answers {
            let mut copy = |exe: &Path, agents: &[&'static Agent]| {
                asked.extend(agents.iter().map(|agent| (exe.to_path_buf(), agent.key)));
                vec![answer; agents.len()]
            };
            on_uninstall(&w.folders, Some(&w.exe), Ok(true), &mut copy, &w.log);
        }
        assert_eq!(asked, vec![(w.exe.clone(), "cursor"); 4]);
        assert_eq!(tree(&w.temp.0), before);
        assert!(w.has("handed to a Medium copy, which ended"));
        assert!(w.has("its copy could not start under Explorer, so it kept this token"));
        assert!(w.has("which did not end within 5 s"));
        assert!(w.has("the Medium copy did not run: OpenProcessToken failed with 5"));

        let mut never = |_: &Path, _: &[&'static Agent]| -> Vec<CopyRun> { panic!("no copy") };
        on_uninstall(&w.folders, Some(&w.exe), Err(unknown), &mut never, &w.log);
        assert_eq!(tree(&w.temp.0), before);
        // No agent folder: nothing to hand off.
        let absent = World::new("elevated-absent");
        fs::remove_dir(absent.folders.profile.join(".cursor")).unwrap();
        on_uninstall(
            &absent.folders,
            Some(&absent.exe),
            Ok(true),
            &mut never,
            &absent.log,
        );
    }

    #[test]
    fn the_real_token_decides_whether_a_start_writes() {
        let w = World::new("token");
        let above = process::current_above_medium().unwrap();
        println!("test runner above Medium: {above}");
        let added = at_start(
            &w.folders,
            &w.record,
            &w.exe,
            process::current_above_medium(),
            &w.log,
        );
        assert_eq!(added.is_empty(), above);
        assert_eq!(w.plugin().exists(), !above);
    }

    const REPORT_ARG: &str = "wakeshot-test-report=";

    /// The child role of the test below, which starts it with Explorer's token: a start with the real level.
    #[test]
    #[ignore = "a child role for the test below"]
    fn medium_role() {
        let Some(report) =
            std::env::args().find_map(|a| a.strip_prefix(REPORT_ARG).map(PathBuf::from))
        else {
            return;
        };
        let w = World::new("medium-role");
        let above = process::current_above_medium();
        let added = at_start(&w.folders, &w.record, &w.exe, above, &w.log);
        let text = format!(
            "above Medium {above:?}; added {added:?}; marker {}",
            w.plugin().join(MARKER).is_file()
        );
        fs::write(report, text).unwrap();
    }

    /// The Medium rule both ways, with real tokens: a start under Explorer's token (Medium, as the installed tray
    /// runs) writes the folder; this runner, elevated on the test laptop, writes nothing. Run with `cargo test --locked
    /// -p wakeshot-windows -- --ignored agent_hooks::`. Needs Explorer as the shell.
    #[test]
    #[ignore = "needs Explorer as the shell (track note 3)"]
    fn a_medium_start_writes_and_an_elevated_one_does_not() {
        let temp = Temp::new("medium");
        let report = temp.0.join("report.txt");
        let mut args: Vec<OsString> = [
            "agent_hooks::tests::medium_role",
            "--exact",
            "--ignored",
            "--test-threads=1",
            "--quiet",
        ]
        .map(OsString::from)
        .into();
        let mut report_arg = OsString::from(REPORT_ARG);
        report_arg.push(&report);
        args.push(report_arg);
        let exe = std::env::current_exe().unwrap();
        let spawned = process::spawn_outside_tree(&exe, &args).unwrap();
        assert!(spawned.outside_tree);
        assert!(spawned.handle.wait(60_000).unwrap(), "the nested run ended");
        let text = fs::read_to_string(&report).expect("the nested run wrote its report");
        println!("nested run: {text}");
        assert_eq!(
            text,
            r#"above Medium Ok(false); added ["Cursor"]; marker true"#
        );

        let above = process::current_above_medium().unwrap();
        println!("this runner above Medium: {above}");
        if above {
            let w = World::new("elevated-real");
            let added = at_start(&w.folders, &w.record, &w.exe, Ok(above), &w.log);
            assert!(added.is_empty());
            assert!(!w.folders.profile.join(r".cursor\plugins").exists());
        }
    }

    #[test]
    fn only_velopacks_current_folder_counts_as_installed() {
        let local = Path::new(r"C:\Users\Ա Բ\AppData\Local");
        let current = local.join(r"HaikAsatryan.Wakeshot\current");
        assert_eq!(
            installed_hook_exe(&current.join("wakeshot-tray.exe"), local),
            Some(current.join("wakeshot.exe"))
        );
        let other_case = Path::new(r"c:\users\Ա Բ\appdata\local\haikasatryan.wakeshot\CURRENT");
        assert_eq!(
            installed_hook_exe(&other_case.join("wakeshot-tray.exe"), local),
            Some(other_case.join("wakeshot.exe"))
        );
        for tray in [
            r"C:\code\wakeshot\target\release\wakeshot-tray.exe",
            r"C:\Users\Ա Բ\AppData\Local\HaikAsatryan.Wakeshot\app-0.1.0\wakeshot-tray.exe",
            r"C:\Users\Ա Բ\AppData\Local\HaikAsatryan.Wakeshot\wakeshot-tray.exe",
            r"C:\Program Files\WindowsApps\HaikAsatryan.Wakeshot_0.1.0.0_x64__abc\wakeshot-tray.exe",
            "wakeshot-tray.exe",
        ] {
            assert_eq!(installed_hook_exe(Path::new(tray), local), None, "{tray}");
        }
    }

    #[test]
    fn the_hook_command_quotes_the_exe_path_for_powershell() {
        let command = |exe: &str| {
            let files = cursor_files(Path::new(exe))?;
            let hooks: Value = serde_json::from_slice(&files[1]).unwrap();
            Some(hooks["hooks"]["stop"][0]["command"].as_str()?.to_owned())
        };
        assert_eq!(
            command(r"C:\Users\O'Brien\AppData\Local\wakeshot.exe").as_deref(),
            Some(r"'C:\Users\O''Brien\AppData\Local\wakeshot.exe' hook cursor stop")
        );
        assert_eq!(
            command(r"C:\Users\Արթուր Ёлка\$env:x `n {a}\wakeshot.exe").as_deref(),
            Some(r"'C:\Users\Արթուր Ёлка\$env:x `n {a}\wakeshot.exe' hook cursor stop")
        );
        for bad in [
            "C:\\Users\\a\u{2018}b\\wakeshot.exe",
            "C:\\Users\\a\u{2019}b\\wakeshot.exe",
            "C:\\Users\\a\u{201A}b\\wakeshot.exe",
            "C:\\Users\\a\u{201B}b\\wakeshot.exe",
            "C:\\Users\\a\tb\\wakeshot.exe",
            "C:\\Users\\a\nb\\wakeshot.exe",
            "",
        ] {
            assert_eq!(command(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn fnv1a_matches_the_published_values() {
        assert_eq!(fnv1a(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a(b"a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(fnv1a(b"foobar"), 0x8594_4171_f739_67e8);
    }

    #[test]
    fn an_unknown_agent_or_a_missing_exe_does_nothing() {
        let log = MemoryLog::default();
        remove_command("zed", &log);
        assert!(log.lines.lock().unwrap().is_empty());
        let temp = Temp::new("missing-exe");
        let runs = medium_copies(&temp.0.join("wakeshot.exe"), &[&CURSOR, &ANTIGRAVITY]);
        assert_eq!(runs.len(), 2);
        for run in runs {
            assert!(
                matches!(run, CopyRun::Failed(e) if e.call == "CreateProcessW"),
                "{run:?}"
            );
        }
    }

    /// `path` by its long name, without the `\\?\` that `canonicalize` adds: a temp folder may come as a short name
    /// (`RUNNER~1` on CI), whose `~` the Antigravity command refuses.
    fn long(path: &Path) -> PathBuf {
        let full = fs::canonicalize(path).unwrap();
        let text = full.to_str().unwrap();
        PathBuf::from(text.strip_prefix(r"\\?\").unwrap_or(text))
    }

    impl World {
        /// A PC with Antigravity and no Cursor: the CLI's `agy.exe` and Antigravity's `.gemini\config`, under the
        /// temp folder's long name.
        fn gemini(tag: &str) -> Self {
            let w = Self::new(tag);
            fs::remove_dir(w.folders.profile.join(".cursor")).unwrap();
            let root = long(&w.temp.0);
            let folders = Folders {
                profile: root.join("profile"),
                local_app_data: root.join("local"),
            };
            let agy = folders.local_app_data.join(r"agy\bin\agy.exe");
            fs::create_dir_all(agy.parent().unwrap()).unwrap();
            fs::write(&agy, b"MZ").unwrap();
            fs::create_dir_all(folders.profile.join(r".gemini\config")).unwrap();
            let exe = folders
                .local_app_data
                .join(r"HaikAsatryan.Wakeshot\current\wakeshot.exe");
            assert!(
                cmd_bare(&exe, false).is_some(),
                "the test folder {} holds a character the Antigravity command refuses: set TEMP to a plain path",
                exe.display()
            );
            Self {
                record: root.join(r"Wakeshot\agent-hooks.json"),
                folders,
                exe,
                ..w
            }
        }

        fn gemini_plugins(&self) -> PathBuf {
            self.folders.profile.join(r".gemini\config\plugins")
        }

        fn gemini_plugin(&self) -> PathBuf {
            self.gemini_plugins().join(FOLDER)
        }
    }

    const ANTIGRAVITY_SET: [&str; 4] = [
        "wakeshot",
        r"wakeshot\hooks.json",
        r"wakeshot\plugin.json",
        r"wakeshot\wakeshot-hooks.json",
    ];

    #[test]
    fn antigravity_gets_its_plugin_folder_once_with_a_bare_command_per_event() {
        let w = World::gemini("antigravity");
        assert_eq!(w.start(), ["Antigravity"]);
        assert_eq!(names(&w.gemini_plugins()), ANTIGRAVITY_SET);
        let want = wanted(&ANTIGRAVITY, &w.exe).unwrap();
        let dir = w.gemini_plugin();
        assert_eq!(fs::read(dir.join("hooks.json")).unwrap(), want.files[0]);
        assert_eq!(fs::read(dir.join("plugin.json")).unwrap(), want.files[1]);
        assert_eq!(fs::read(dir.join(MARKER)).unwrap(), want.marker);

        let hooks: Value = serde_json::from_slice(&want.files[0]).unwrap();
        let groups = hooks.as_object().unwrap();
        assert_eq!(
            groups.keys().collect::<Vec<_>>(),
            ["wakeshot"],
            "one named group"
        );
        let group = groups["wakeshot"].as_object().unwrap();
        assert_eq!(group.len(), 2, "no tool events, no enabled key");
        for event in ANTIGRAVITY_EVENTS {
            let run = group[event].as_array().unwrap();
            assert_eq!(run.len(), 1);
            let command = format!("{} hook antigravity {event}", w.exe.display());
            assert_eq!(run[0]["command"], command.as_str());
            assert_eq!(run[0]["type"], "command");
            assert_eq!(run[0]["timeout"], 5);
            assert_eq!(run[0].as_object().unwrap().len(), 3);
        }
        let plugin: Value = serde_json::from_slice(&want.files[1]).unwrap();
        assert_eq!(plugin["name"], "wakeshot");
        assert_eq!(
            parse_marker(&want.marker, &ANTIGRAVITY),
            Some(vec![fnv1a(&want.files[0]), fnv1a(&want.files[1])])
        );
        let marker = String::from_utf8(want.marker.clone()).unwrap();
        assert!(
            marker.find("\"hooks\"").unwrap() < marker.find("\"plugin\"").unwrap(),
            "written in file order, the manifest last: {marker}"
        );
        assert_eq!(
            w.record_text().as_deref(),
            Some("{\"version\":1,\"antigravity\":\"added\"}\n")
        );
        assert!(w.has(r"Antigravity: hook folder added (.gemini\config\plugins\wakeshot)"));

        let before = tree(&w.temp.0);
        assert!(w.start().is_empty());
        assert_eq!(tree(&w.temp.0), before, "a second start writes nothing");

        // Uninstall takes the fixed set and leaves `plugins`, which other plugins share.
        fs::create_dir_all(w.gemini_plugins().join("other")).unwrap();
        fs::write(w.gemini_plugins().join(r"other\plugin.json"), b"{}").unwrap();
        let mut before = tree(&w.temp.0);
        before.retain(|path, _| !path.starts_with(r"profile\.gemini\config\plugins\wakeshot"));
        w.uninstall();
        assert_eq!(tree(&w.temp.0), before);
        assert!(w.has(r"Antigravity: .gemini\config\plugins\wakeshot removed"));
    }

    /// A stop during the add, before the manifest: `agy` may list a dot-named folder, but one with no `plugin.json` is
    /// no plugin; the next start removes it by the fixed names and adds the folder.
    #[test]
    fn antigravity_writes_its_manifest_last_and_a_stop_before_it_is_cleaned_up() {
        assert_eq!(
            ANTIGRAVITY.files.last().map(|f| f.name),
            Some("plugin.json")
        );
        let w = World::gemini("agy-staging");
        let want = wanted(&ANTIGRAVITY, &w.exe).unwrap();
        let staging = w.gemini_plugins().join(".wakeshot-new-4242");
        fs::create_dir_all(&staging).unwrap();
        fs::write(staging.join("hooks.json"), &want.files[0]).unwrap();
        assert_eq!(w.start(), ["Antigravity"]);
        assert_eq!(names(&w.gemini_plugins()), ANTIGRAVITY_SET);
        assert!(w.has(
            r"Antigravity: removed .gemini\config\plugins\.wakeshot-new-4242, which a stop left"
        ));
    }

    #[test]
    fn antigravity_counts_only_with_agy_and_its_config_folder() {
        // The IDE's copy alone is enough.
        let w = World::gemini("agy-ide");
        fs::remove_file(w.folders.local_app_data.join(r"agy\bin\agy.exe")).unwrap();
        let ide = w.folders.profile.join(r".gemini\bin\agy.exe");
        fs::create_dir_all(ide.parent().unwrap()).unwrap();
        fs::write(&ide, b"MZ").unwrap();
        assert_eq!(w.start(), ["Antigravity"]);

        // No `agy.exe`: Gemini CLI's `.gemini`, never touched.
        let w = World::gemini("agy-none");
        fs::remove_file(w.folders.local_app_data.join(r"agy\bin\agy.exe")).unwrap();
        let before = tree(&w.temp.0);
        assert!(w.start().is_empty());
        assert_eq!(tree(&w.temp.0), before);

        // A folder in place of the exe is no exe.
        fs::create_dir_all(w.folders.profile.join(r".gemini\bin\agy.exe")).unwrap();
        assert!(!agy_present(&w.folders, false));

        // No `config`: Wakeshot never makes it.
        let w = World::gemini("agy-no-config");
        fs::remove_dir(w.folders.profile.join(r".gemini\config")).unwrap();
        assert!(w.start().is_empty());
        assert!(!w.folders.profile.join(r".gemini\config").exists());

        // `.gemini` moved elsewhere: no hooks.
        let w = World::gemini("agy-moved");
        assert!(agy_present(&w.folders, false));
        assert!(!agy_present(&w.folders, true));
    }

    #[test]
    fn a_path_cmd_would_split_gets_no_antigravity_hooks() {
        assert_eq!(
            cmd_bare(
                Path::new(r"C:\Users\Արթուր-Ёлка_2\AppData\Local\wakeshot.exe"),
                false
            )
            .as_deref(),
            Some(r"C:\Users\Արթուր-Ёлка_2\AppData\Local\wakeshot.exe")
        );
        for bad in [
            r"C:\Users\John Smith\wakeshot.exe",
            r#"C:\Users\a"b\wakeshot.exe"#,
            r"C:\Users\O'Brien\wakeshot.exe",
            r"C:\Users\a%PATH%b\wakeshot.exe",
            r"C:\Users\a^b\wakeshot.exe",
            r"C:\Users\a&calc\wakeshot.exe",
            r"C:\Users\a|b\wakeshot.exe",
            r"C:\Users\a(b)\wakeshot.exe",
            r"C:\Users\a!b!\wakeshot.exe",
            r"C:\Users\a@b\wakeshot.exe",
            r"C:\Users\RUNNER~1\wakeshot.exe",
            r"C:\Users\a,b;c=d\wakeshot.exe",
            "C:\\Users\\a\u{2018}b\\wakeshot.exe",
            "C:\\Users\\a\u{FF06}b\\wakeshot.exe",
            "C:\\Users\\a\u{00A0}b\\wakeshot.exe",
            "C:\\Users\\a\tb\\wakeshot.exe",
            "",
        ] {
            assert_eq!(cmd_bare(Path::new(bad), false), None, "{bad:?}");
            assert!(antigravity_files(Path::new(bad)).is_none(), "{bad:?}");
        }

        // An 8.3 form may hold `~` inside a name, never a space or a `~` at the start.
        assert!(cmd_bare(Path::new(r"C:\Users\JOHNSM~1\wakeshot.exe"), true).is_some());
        for bad in [r"~\wakeshot.exe", r"C:\Users\John Smith\wakeshot.exe"] {
            assert_eq!(cmd_bare(Path::new(bad), true), None, "{bad:?}");
        }

        // At a start with a space in the exe's path: the 8.3 form where the volume makes short names, else nothing
        // and one line says why. Cursor, whose command quotes, gets its folder either way.
        let w = World::gemini("agy-space");
        fs::create_dir_all(w.folders.profile.join(".cursor")).unwrap();
        let spaced = w.temp.0.join(r"my apps\wakeshot.exe");
        fs::create_dir_all(spaced.parent().unwrap()).unwrap();
        fs::write(&spaced, b"MZ").unwrap();
        let short = short_path(&spaced).and_then(|short| cmd_bare(&short, true));
        println!("the 8.3 form of the test exe: {short:?}");
        let added = at_start(&w.folders, &w.record, &spaced, Ok(false), &w.log);
        if let Some(short) = short {
            assert_eq!(added, ["Cursor", "Antigravity"]);
            let hooks =
                fs::read_to_string(w.gemini_plugins().join(r"wakeshot\hooks.json")).unwrap();
            let command = json_string(&format!("{short} hook antigravity Stop"));
            assert!(hooks.contains(&command), "{hooks}");
            return;
        }
        assert_eq!(added, ["Cursor"]);
        assert!(!w.gemini_plugins().exists());
        assert!(w.has(
            "Antigravity: the hook exe's path holds a character the hook command cannot quote, so it gets no hooks"
        ));
        assert_eq!(
            w.record_text().as_deref(),
            Some("{\"version\":1,\"cursor\":\"added\"}\n")
        );
    }

    #[test]
    fn a_folder_antigravity_or_the_user_changed_is_never_touched() {
        type Edit = fn(&Path);
        let edits: [(&str, Edit); 4] = [
            ("an edited hooks.json", |dir| {
                fs::write(dir.join("hooks.json"), b"{}").unwrap();
            }),
            ("an added skills folder", |dir| {
                fs::create_dir(dir.join("skills")).unwrap();
            }),
            ("a hooks folder", |dir| {
                fs::create_dir(dir.join("hooks")).unwrap();
            }),
            // How older `agy` versions turned a plugin off.
            ("a disabled manifest", |dir| {
                fs::rename(dir.join("plugin.json"), dir.join("plugin.json.disabled")).unwrap();
            }),
        ];
        for (what, edit) in edits {
            let w = World::gemini("agy-changed");
            assert_eq!(w.start(), ["Antigravity"], "{what}");
            edit(&w.gemini_plugin());
            let before = tree(&w.gemini_plugin());
            assert!(w.start().is_empty(), "{what}");
            assert_eq!(tree(&w.gemini_plugin()), before, "{what}");
            assert!(w.has("was changed since Wakeshot wrote it"), "{what}");
            // Uninstall keeps what is not Wakeshot's under a dot name the agent does not load.
            w.uninstall();
            assert!(!w.gemini_plugin().exists(), "{what}");
            assert!(w.gemini_plugins().join(REMOVED).is_dir(), "{what}");
        }
    }

    #[test]
    fn an_elevated_uninstall_hands_antigravity_to_its_own_medium_copy() {
        let w = World::gemini("agy-elevated");
        fs::create_dir_all(w.folders.profile.join(".cursor")).unwrap();
        assert_eq!(w.start(), ["Cursor", "Antigravity"]);
        let before = tree(&w.temp.0);
        let mut asked = Vec::new();
        let mut copy = |exe: &Path, agents: &[&'static Agent]| {
            asked.push((
                exe.to_path_buf(),
                agents.iter().map(|a| a.key).collect::<Vec<_>>(),
            ));
            vec![CopyRun::Ended { outside_tree: true }; agents.len()]
        };
        on_uninstall(&w.folders, Some(&w.exe), Ok(true), &mut copy, &w.log);
        // One call for both, so every copy starts before any wait and the waits share one budget (review m1).
        assert_eq!(asked, [(w.exe.clone(), vec!["cursor", "antigravity"])]);
        assert!(w.has("Antigravity: hook folder removal handed to a Medium copy, which ended"));
        assert_eq!(tree(&w.temp.0), before);
        assert_eq!(find("antigravity").map(|a| a.label), Some("Antigravity"));
    }

    /// Orchestrator decision, 2026-11-06: the agent's own folder gone is the agent gone, so the record forgets it and a
    /// reinstall gets the hooks again; only Wakeshot's folder gone, the agent's still there, stays "removed".
    #[test]
    fn an_agent_whose_own_folder_went_gets_its_hooks_again_when_it_is_back() {
        let w = World::new("reinstalled");
        assert_eq!(w.start(), ["Cursor"]);
        fs::remove_dir_all(w.folders.profile.join(".cursor")).unwrap();
        assert!(w.start().is_empty());
        assert_eq!(w.record_text().as_deref(), Some("{\"version\":1}\n"));
        assert!(w.has(
            "Cursor: .cursor is gone, so Wakeshot forgets its hook folder there and adds it again when the agent is back"
        ));
        let lines = w.lines().len();
        assert!(w.start().is_empty());
        assert_eq!(w.lines().len(), lines, "said once");
        fs::create_dir_all(w.folders.profile.join(".cursor")).unwrap();
        assert_eq!(w.start(), ["Cursor"], "added again, with its notice");
        assert_eq!(names(&w.local()), FIXED_SET);

        // Only Wakeshot's folder gone: the user's no, kept.
        fs::remove_dir_all(w.plugin()).unwrap();
        assert!(w.start().is_empty());
        assert!(w.start().is_empty());
        assert!(!w.plugin().exists());
        assert_eq!(
            w.record_text().as_deref(),
            Some("{\"version\":1,\"cursor\":\"removed\"}\n")
        );
        // That no was for the Cursor that went: a new one starts as on a new PC.
        fs::remove_dir_all(w.folders.profile.join(".cursor")).unwrap();
        assert!(w.start().is_empty());
        fs::create_dir_all(w.folders.profile.join(".cursor")).unwrap();
        assert_eq!(w.start(), ["Cursor"]);

        // Antigravity: `.gemini\config` is its own folder; `agy.exe` gone alone forgets nothing.
        let w = World::gemini("reinstalled-agy");
        assert_eq!(w.start(), ["Antigravity"]);
        let agy = w.folders.local_app_data.join(r"agy\bin\agy.exe");
        fs::remove_file(&agy).unwrap();
        assert!(w.start().is_empty());
        assert_eq!(
            w.record_text().as_deref(),
            Some("{\"version\":1,\"antigravity\":\"added\"}\n")
        );
        fs::remove_dir_all(w.folders.profile.join(r".gemini\config")).unwrap();
        assert!(w.start().is_empty());
        assert_eq!(w.record_text().as_deref(), Some("{\"version\":1}\n"));
        assert!(w.has(r"Antigravity: .gemini\config is gone, so Wakeshot forgets"));
        fs::write(&agy, b"MZ").unwrap();
        fs::create_dir_all(w.folders.profile.join(r".gemini\config")).unwrap();
        assert_eq!(w.start(), ["Antigravity"]);
    }

    /// A junction at each part of Antigravity's path, to a tree with an older plugin folder: nothing through it is
    /// written or deleted.
    #[test]
    fn a_link_on_antigravitys_path_stops_every_write_and_delete() {
        let parents = [".gemini", "config", "plugins", FOLDER];
        for depth in 0..parents.len() {
            let part = parents[depth];
            let w = World::gemini("agy-link");
            let out = w.outside();
            let mut old = wanted(&ANTIGRAVITY, &w.exe).unwrap().files;
            old[0] = b"{\"wakeshot\":{}}\n".to_vec();
            let target = out.join(part);
            let plugin = parents[depth + 1..]
                .iter()
                .fold(target.clone(), |path, p| path.join(p));
            fs::create_dir_all(&plugin).unwrap();
            for (fixed, bytes) in ANTIGRAVITY.files.iter().zip(&old) {
                fs::write(file_path(&plugin, fixed), bytes).unwrap();
            }
            fs::write(
                plugin.join(MARKER),
                marker_bytes(&ANTIGRAVITY, &old, "0.0.1"),
            )
            .unwrap();
            let link = parents[..=depth]
                .iter()
                .fold(w.folders.profile.clone(), |path, p| path.join(p));
            let _ = fs::remove_dir_all(&link);
            fs::create_dir_all(link.parent().unwrap()).unwrap();
            junction(&link, &target);
            let before = tree(&out);
            assert!(w.start().is_empty(), "{part}");
            w.uninstall();
            assert_eq!(
                tree(&out),
                before,
                "{part}: nothing changed through the link"
            );
            assert!(
                w.has("is a link, a junction or not a plain folder"),
                "{part}: {:?}",
                w.lines()
            );
        }
    }

    #[test]
    fn a_record_that_does_not_read_forgets_nothing() {
        let w = World::new("record-unread");
        fs::remove_dir(w.folders.profile.join(".cursor")).unwrap();
        fs::create_dir_all(w.record.parent().unwrap()).unwrap();
        let record = r#"{"version":2,"cursor":"added"}"#;
        fs::write(&w.record, record).unwrap();
        assert!(w.start().is_empty());
        assert_eq!(fs::read_to_string(&w.record).unwrap(), record);
    }
}
