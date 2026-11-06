//! Read-only SQLite queries for `WantedFile::Query` (architecture section 7), through Windows' own `winsqlite3.dll`
//! loaded at run time (`LoadLibraryExW` with `LOAD_LIBRARY_SEARCH_SYSTEM32`, then `GetProcAddress`), so no SQLite is
//! built into Wakeshot. The watch thread calls [`query`] and feeds its answer as it is. A Windows without the DLL, or
//! with one older than [`MIN_VERSION`], answers `no_sqlite`, and the source reads that as "no signal".
//!
//! The agent owns the database, so a read must never change it or get in its way:
//!
//! - It runs only while the database, its `-wal` and its `-shm` exist: the agent has it open in WAL mode, or an agent
//!   that died left them. Then each read rebuilds the `-shm`'s index from the whole `-wal` (it is the first opener of
//!   the `-shm`), and writes nothing else. Otherwise it answers `Busy` (the watch thread feeds `Gone` once the `-wal`
//!   went).
//! - The connection goes through a VFS shim, [`VFS_NAME`], that opens the database and its `-wal` read only and never
//!   creates them. A plain read-only open of a WAL database whose agent just closed it creates both the `-wal` and the
//!   `-shm` and leaves them (`docs/research/opencode-signals.md`, probe). SQLite opens the `-shm` only after the
//!   `-wal`, so the shim stops both. The `-shm` is opened read-write, as by every WAL reader: its read marks are how a
//!   writer knows which pages a reader still needs.
//! - The shim refuses every delete (SQLite deletes the `-wal` of an empty database file), and the connection skips
//!   the checkpoint on close (`SQLITE_DBCONFIG_NO_CKPT_ON_CLOSE`, so the close takes no lock on the database either);
//!   with the `-wal` read only no checkpoint could run anyway. So a read never writes the database or the `-wal` and
//!   never deletes a file.
//! - Open `SQLITE_OPEN_READONLY | SQLITE_OPEN_NOMUTEX`, a [`BUSY_MS`] busy timeout, one statement, finalize and close
//!   on every call: nothing stays open between reads to hold back the agent's checkpoints.
//!
//! An answer names what failed with a fixed word, never SQLite's text, so nothing from the file reaches a log. A text
//! cell longer than [`MAX_CELL`] bytes ends the read before it is copied.
//!
//! Owner: `p4-opencode-source`; the signature by `p4-contracts-r2`.

use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::path::Path;
use std::ptr;
use std::sync::OnceLock;

use serde_json::Value;
use windows::Win32::System::LibraryLoader::{
    GetProcAddress, LOAD_LIBRARY_SEARCH_SYSTEM32, LoadLibraryExW,
};
use windows::core::{HSTRING, PCSTR};

/// One query's answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Answer {
    /// The bytes to feed as `Feed::Whole`, in the contract's JSON: `{"rows":[[...],...]}` with integer, text or null
    /// cells, or `{"error":"<kind>"}` with a fixed kind, never SQLite's own text: `no_sqlite`, `open`, `prepare`,
    /// `step`, `type` (a real or blob cell) or `size` (more than [`MAX_ROWS`] rows, or a text cell over [`MAX_CELL`]
    /// bytes).
    Json(Vec<u8>),
    /// `SQLITE_BUSY`, or the database closed before or during the read: no feed, so the source keeps the last
    /// answer.
    Busy,
}

/// Windows' own SQLite.
pub const DLL: &str = "winsqlite3.dll";

/// SQLite 3.22.0: the first that reads a WAL database read only (sqlite.org/wal.html, "Read-Only Databases").
pub const MIN_VERSION: c_int = 3_022_000;

/// How long a read waits for a lock the agent holds: long enough for its commit, short enough for the watch thread.
pub const BUSY_MS: c_int = 100;

/// A bound on one answer, so a strange file cannot fill memory. OpenCode's queries return a row per working session
/// or active schedule.
pub const MAX_ROWS: usize = 10_000;

/// The longest text cell taken, in bytes: the queries return ids and table names, and the source takes ids of at most
/// 128 characters. A longer one is refused before Wakeshot copies it.
pub const MAX_CELL: usize = 1_024;

/// The VFS shim's name.
pub const VFS_NAME: &CStr = c"wakeshot-readonly";

const SQLITE_OK: c_int = 0;
const SQLITE_BUSY: c_int = 5;
const SQLITE_CANTOPEN: c_int = 14;
const SQLITE_ROW: c_int = 100;
const SQLITE_DONE: c_int = 101;
const SQLITE_IOERR_DELETE: c_int = 10 | (10 << 8);

const DBCONFIG_NO_CKPT_ON_CLOSE: c_int = 1006;

const OPEN_READONLY: c_int = 0x0000_0001;
const OPEN_READWRITE: c_int = 0x0000_0002;
const OPEN_CREATE: c_int = 0x0000_0004;
const OPEN_MAIN_DB: c_int = 0x0000_0100;
const OPEN_NOMUTEX: c_int = 0x0000_8000;
const OPEN_WAL: c_int = 0x0008_0000;

const COLUMN_INTEGER: c_int = 1;
const COLUMN_TEXT: c_int = 3;
const COLUMN_NULL: c_int = 5;

/// `struct sqlite3`, `sqlite3_stmt` and `sqlite3_file`: SQLite's own, only ever behind a pointer.
#[repr(C)]
struct Connection {
    _private: [u8; 0],
}

#[repr(C)]
struct Statement {
    _private: [u8; 0],
}

#[repr(C)]
struct OsFile {
    _private: [u8; 0],
}

type VfsOpen =
    unsafe extern "system" fn(*mut Vfs, *const c_char, *mut OsFile, c_int, *mut c_int) -> c_int;
type VfsDelete = unsafe extern "system" fn(*mut Vfs, *const c_char, c_int) -> c_int;

/// `struct sqlite3_vfs`, version 3, as `winsqlite3.h` spells it. The methods after `xDelete` are only copied from the
/// default VFS and called by SQLite, never by Wakeshot, so they stay plain pointers.
#[repr(C)]
struct Vfs {
    version: c_int,
    os_file_size: c_int,
    max_path: c_int,
    next: *mut Vfs,
    name: *const c_char,
    app_data: *mut c_void,
    open: Option<VfsOpen>,
    delete: Option<VfsDelete>,
    /// `xAccess` to `xGetLastError` (version 1), `xCurrentTimeInt64` (2), the three system call methods (3).
    methods: [*const c_void; 14],
}

/// The shim: its `Vfs` first, so SQLite's pointer to it is a pointer to the whole.
#[repr(C)]
struct Shim {
    vfs: Vfs,
    base: *mut Vfs,
}

/// The exports used, as `winsqlite3.h` declares them.
type OpenV2 =
    unsafe extern "system" fn(*const c_char, *mut *mut Connection, c_int, *const c_char) -> c_int;
type PrepareV2 = unsafe extern "system" fn(
    *mut Connection,
    *const c_char,
    c_int,
    *mut *mut Statement,
    *mut *const c_char,
) -> c_int;
type OfConnection = unsafe extern "system" fn(*mut Connection) -> c_int;
type BusyTimeout = unsafe extern "system" fn(*mut Connection, c_int) -> c_int;
/// `sqlite3_db_config` is the one `SQLITE_CDECL` export taken, and it has variable arguments.
type DbConfig = unsafe extern "C" fn(*mut Connection, c_int, ...) -> c_int;
type OfStatement = unsafe extern "system" fn(*mut Statement) -> c_int;
type OfColumn = unsafe extern "system" fn(*mut Statement, c_int) -> c_int;
type ColumnInt64 = unsafe extern "system" fn(*mut Statement, c_int) -> i64;
type ColumnText = unsafe extern "system" fn(*mut Statement, c_int) -> *const u8;
type LibVersion = unsafe extern "system" fn() -> c_int;
type VfsFind = unsafe extern "system" fn(*const c_char) -> *mut Vfs;
type VfsRegister = unsafe extern "system" fn(*mut Vfs, c_int) -> c_int;

/// The calls a read needs, found once in the DLL. The DLL is never freed, so they stay valid.
struct Api {
    open_v2: OpenV2,
    close: OfConnection,
    busy_timeout: BusyTimeout,
    db_config: DbConfig,
    prepare_v2: PrepareV2,
    step: OfStatement,
    finalize: OfStatement,
    column_count: OfStatement,
    column_type: OfColumn,
    column_int64: ColumnInt64,
    column_text: ColumnText,
    column_bytes: OfColumn,
}

/// Runs `sql` read only on the database `db`.
pub fn query(db: &Path, sql: &str) -> Answer {
    read(api(), db, sql)
}

/// The real DLL, loaded on the first query.
fn api() -> Option<&'static Api> {
    static API: OnceLock<Option<Api>> = OnceLock::new();
    API.get_or_init(|| load(DLL)).as_ref()
}

fn read(api: Option<&Api>, db: &Path, sql: &str) -> Answer {
    let Some(api) = api else {
        return error("no_sqlite");
    };
    if !open_by_agent(db) {
        return Answer::Busy;
    }
    match run(api, db, sql) {
        Ok(rows) => Answer::Json(
            serde_json::to_vec(&serde_json::json!({ "rows": rows }))
                .unwrap_or_else(|_| br#"{"error":"size"}"#.to_vec()),
        ),
        Err(Fail::Busy) => Answer::Busy,
        // The agent closed the database under the read, so the shim refused the `-wal`.
        Err(Fail::Kind(_)) if !open_by_agent(db) => Answer::Busy,
        Err(Fail::Kind(kind)) => error(kind),
    }
}

fn error(kind: &str) -> Answer {
    Answer::Json(format!(r#"{{"error":"{kind}"}}"#).into_bytes())
}

/// The database and both its WAL files exist: the agent has it open in WAL mode, so a read creates nothing.
fn open_by_agent(db: &Path) -> bool {
    let with = |suffix: &str| {
        let mut path = db.as_os_str().to_owned();
        path.push(suffix);
        Path::new(&path).is_file()
    };
    db.is_file() && with("-wal") && with("-shm")
}

#[derive(Debug, PartialEq, Eq)]
enum Fail {
    Busy,
    Kind(&'static str),
}

fn fail(rc: c_int, kind: &'static str) -> Fail {
    // Extended codes such as `SQLITE_BUSY_SNAPSHOT` keep the primary code in the low byte.
    if rc & 0xff == SQLITE_BUSY {
        Fail::Busy
    } else {
        Fail::Kind(kind)
    }
}

/// Closes the connection on every path; SQLite takes a null pointer as a no-op, and a failed open still gives a
/// handle to close.
struct Open<'a> {
    api: &'a Api,
    ptr: *mut Connection,
}

impl Drop for Open<'_> {
    fn drop(&mut self) {
        // SAFETY: `ptr` came from `sqlite3_open_v2` (or is null), and every statement on it is finalized first.
        unsafe { (self.api.close)(self.ptr) };
    }
}

struct Prepared<'a> {
    api: &'a Api,
    ptr: *mut Statement,
}

impl Drop for Prepared<'_> {
    fn drop(&mut self) {
        // SAFETY: `ptr` came from `sqlite3_prepare_v2` (or is null, a no-op).
        unsafe { (self.api.finalize)(self.ptr) };
    }
}

/// The rows of `sql`, through the shim.
fn run(api: &Api, db: &Path, sql: &str) -> Result<Vec<Value>, Fail> {
    run_on(api, db, sql, VFS_NAME.as_ptr())
}

/// A read-only connection on `vfs`, with the busy timeout and no checkpoint on close.
fn open_on<'a>(api: &'a Api, db: &Path, vfs: *const c_char) -> Result<Open<'a>, Fail> {
    let path = db
        .to_str()
        .and_then(|path| CString::new(path).ok())
        .ok_or(Fail::Kind("open"))?;
    let mut conn = Open {
        api,
        ptr: ptr::null_mut(),
    };
    // SAFETY: `path` and `vfs` are NUL-terminated and outlive the call; `conn.ptr` receives the handle.
    let rc = unsafe {
        (api.open_v2)(
            path.as_ptr(),
            &mut conn.ptr,
            OPEN_READONLY | OPEN_NOMUTEX,
            vfs,
        )
    };
    if rc != SQLITE_OK {
        return Err(fail(rc, "open"));
    }
    // SAFETY: an open connection.
    unsafe { (api.busy_timeout)(conn.ptr, BUSY_MS) };
    // SAFETY: an open connection; this option takes an int and an int pointer, which may be null.
    let rc = unsafe {
        (api.db_config)(
            conn.ptr,
            DBCONFIG_NO_CKPT_ON_CLOSE,
            1 as c_int,
            ptr::null_mut::<c_int>(),
        )
    };
    if rc != SQLITE_OK {
        return Err(Fail::Kind("open"));
    }
    Ok(conn)
}

fn run_on(api: &Api, db: &Path, sql: &str, vfs: *const c_char) -> Result<Vec<Value>, Fail> {
    let len = c_int::try_from(sql.len()).map_err(|_| Fail::Kind("prepare"))?;
    let conn = open_on(api, db, vfs)?;
    let mut stmt = Prepared {
        api,
        ptr: ptr::null_mut(),
    };
    // SAFETY: `sql` is `len` bytes and outlives the call; the tail pointer is not wanted.
    let rc = unsafe {
        (api.prepare_v2)(
            conn.ptr,
            sql.as_ptr().cast(),
            len,
            &mut stmt.ptr,
            ptr::null_mut(),
        )
    };
    if rc != SQLITE_OK {
        return Err(fail(rc, "prepare"));
    }
    if stmt.ptr.is_null() {
        return Err(Fail::Kind("prepare"));
    }
    // SAFETY: a prepared statement.
    let columns = unsafe { (api.column_count)(stmt.ptr) };
    let mut rows = Vec::new();
    loop {
        // SAFETY: a prepared statement, stepped on this thread only.
        match unsafe { (api.step)(stmt.ptr) } {
            SQLITE_ROW => {
                if rows.len() == MAX_ROWS {
                    return Err(Fail::Kind("size"));
                }
                let row = (0..columns)
                    .map(|i| cell(api, stmt.ptr, i))
                    .collect::<Result<Vec<Value>, Fail>>()?;
                rows.push(Value::Array(row));
            }
            SQLITE_DONE => break,
            rc => return Err(fail(rc, "step")),
        }
    }
    drop(stmt);
    drop(conn);
    Ok(rows)
}

fn cell(api: &Api, stmt: *mut Statement, i: c_int) -> Result<Value, Fail> {
    // SAFETY: `stmt` has a row and `i` is below its column count.
    match unsafe { (api.column_type)(stmt, i) } {
        COLUMN_NULL => Ok(Value::Null),
        // SAFETY: as above.
        COLUMN_INTEGER => Ok(Value::from(unsafe { (api.column_int64)(stmt, i) })),
        COLUMN_TEXT => {
            // The length in UTF-8 (SQLite converts a UTF-16 value first), so a long cell is refused uncopied.
            // SAFETY: as above.
            let long =
                usize::try_from(unsafe { (api.column_bytes)(stmt, i) }).unwrap_or(usize::MAX);
            if long > MAX_CELL {
                return Err(Fail::Kind("size"));
            }
            // The text, then its length again: SQLite's documented order, so the length is of the bytes given.
            // SAFETY: as above; the bytes stay valid until the next step, and are copied before it.
            let text = unsafe { (api.column_text)(stmt, i) };
            // SAFETY: as above.
            let len = usize::try_from(unsafe { (api.column_bytes)(stmt, i) }).unwrap_or(0);
            if text.is_null() {
                return Ok(Value::String(String::new()));
            }
            // SAFETY: SQLite gives `len` bytes at `text`.
            let bytes = unsafe { std::slice::from_raw_parts(text, len) };
            Ok(Value::String(String::from_utf8_lossy(bytes).into_owned()))
        }
        _ => Err(Fail::Kind("type")),
    }
}

/// Loads `dll` from System32, checks its version and registers the shim; `None` when any of it fails.
fn load(dll: &str) -> Option<Api> {
    let name = HSTRING::from(dll);
    // SAFETY: searched in System32 only. It is never freed, so the pointers below stay valid.
    let module = unsafe { LoadLibraryExW(&name, None, LOAD_LIBRARY_SEARCH_SYSTEM32) }.ok()?;
    macro_rules! export {
        ($name:literal as $kind:ty) => {{
            // SAFETY: the module is loaded and the name is NUL-terminated.
            let found = unsafe { GetProcAddress(module, PCSTR(concat!($name, "\0").as_ptr())) }?;
            // SAFETY: the export has the signature `winsqlite3.h` gives it, which `$kind` spells; on x64 and ARM64
            // `__stdcall` is the one calling convention.
            unsafe { std::mem::transmute::<unsafe extern "system" fn() -> isize, $kind>(found) }
        }};
    }
    let version = export!("sqlite3_libversion_number" as LibVersion);
    // SAFETY: no arguments.
    if unsafe { version() } < MIN_VERSION {
        return None;
    }
    let vfs_find = export!("sqlite3_vfs_find" as VfsFind);
    let vfs_register = export!("sqlite3_vfs_register" as VfsRegister);
    if !register_shim(vfs_find, vfs_register) {
        return None;
    }
    Some(Api {
        open_v2: export!("sqlite3_open_v2" as OpenV2),
        close: export!("sqlite3_close" as OfConnection),
        busy_timeout: export!("sqlite3_busy_timeout" as BusyTimeout),
        db_config: export!("sqlite3_db_config" as DbConfig),
        prepare_v2: export!("sqlite3_prepare_v2" as PrepareV2),
        step: export!("sqlite3_step" as OfStatement),
        finalize: export!("sqlite3_finalize" as OfStatement),
        column_count: export!("sqlite3_column_count" as OfStatement),
        column_type: export!("sqlite3_column_type" as OfColumn),
        column_int64: export!("sqlite3_column_int64" as ColumnInt64),
        column_text: export!("sqlite3_column_text" as ColumnText),
        column_bytes: export!("sqlite3_column_bytes" as OfColumn),
    })
}

/// Registers the shim over the default VFS once per process; it stays registered, as the connections use it by name.
fn register_shim(vfs_find: VfsFind, vfs_register: VfsRegister) -> bool {
    // SAFETY: a NUL-terminated name; SQLite initializes itself on this call.
    if !unsafe { vfs_find(VFS_NAME.as_ptr()) }.is_null() {
        return true;
    }
    // SAFETY: null asks for the default VFS.
    let base = unsafe { vfs_find(ptr::null()) };
    // SAFETY: a VFS SQLite gave out stays valid while registered, and the default one is never unregistered.
    if base.is_null() || unsafe { (*base).version } < 3 {
        return false;
    }
    // SAFETY: a version 3 VFS has every field `Vfs` spells.
    let mut vfs = unsafe { ptr::read(base) };
    vfs.version = 3;
    vfs.next = ptr::null_mut();
    vfs.name = VFS_NAME.as_ptr();
    vfs.open = Some(open_read_only);
    vfs.delete = Some(refuse_delete);
    let shim = Box::leak(Box::new(Shim { vfs, base }));
    // SAFETY: the shim lives for the process, as a registered VFS must.
    unsafe { vfs_register(&mut shim.vfs, 0) == SQLITE_OK }
}

/// The shim's `xOpen`: the database and its `-wal` read only and never created; anything else (SQLite's temp files)
/// as asked.
unsafe extern "system" fn open_read_only(
    vfs: *mut Vfs,
    name: *const c_char,
    file: *mut OsFile,
    flags: c_int,
    out_flags: *mut c_int,
) -> c_int {
    // SAFETY: SQLite calls this only through the shim's own `Vfs`, the first field of a leaked `Shim`.
    let base = unsafe { (*vfs.cast::<Shim>()).base };
    let flags = if flags & (OPEN_MAIN_DB | OPEN_WAL) != 0 {
        (flags & !(OPEN_READWRITE | OPEN_CREATE)) | OPEN_READONLY
    } else {
        flags
    };
    // SAFETY: `base` is the default VFS, valid for the process. It gets itself as its VFS, so the file it opens and
    // the `-shm` it maps later are its own.
    match unsafe { (*base).open } {
        Some(open) => unsafe { open(base, name, file, flags, out_flags) },
        None => SQLITE_CANTOPEN,
    }
}

/// The shim's `xDelete`: the agent's files are never Wakeshot's to remove, so the read fails instead.
unsafe extern "system" fn refuse_delete(
    _vfs: *mut Vfs,
    _name: *const c_char,
    _sync: c_int,
) -> c_int {
    SQLITE_IOERR_DELETE
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::fs;
    use std::io::{BufRead, BufReader, Write};
    use std::path::PathBuf;
    use std::process::{Child, Stdio};
    use std::sync::Arc;
    use std::time::Duration;

    use wakeshot_core::command::Command;
    use wakeshot_core::core::Core;
    use wakeshot_core::event::Event;
    use wakeshot_core::files::Config;
    use wakeshot_core::process::{Alive, FileTime, ProcessRef};
    use wakeshot_core::sources::opencode::{
        IMAGES, LABEL, OpenCodeSource, PROBLEM_KEY, SCHEDULES, TABLES, UNREADABLE, WORKING,
    };
    use wakeshot_core::sources::{
        Activity, AgentSource, Feed, FileId, FormatHealth, Liveness as Seen, SessionFacts,
        SessionKey, SourceId,
    };
    use wakeshot_core::time::Now;

    use super::*;
    use crate::log::MemoryLog;
    use crate::process::{FakeLiveness, ProcessEntry};
    use crate::watch::agents::Agents;
    use crate::watch::read::TempDir;

    fn real() -> &'static Api {
        api().expect("winsqlite3.dll loads on this laptop")
    }

    /// A writer on the default VFS, as the agent is, which keeps the database open in WAL mode while it lives.
    struct Writer {
        conn: *mut Connection,
    }

    impl Writer {
        fn open(db: &Path) -> Self {
            let api = real();
            let path = CString::new(db.to_str().unwrap()).unwrap();
            let mut conn = ptr::null_mut();
            // SAFETY: a NUL-terminated path; null picks the default VFS.
            let rc = unsafe {
                (api.open_v2)(
                    path.as_ptr(),
                    &mut conn,
                    OPEN_READWRITE | OPEN_CREATE,
                    ptr::null(),
                )
            };
            assert_eq!(rc, SQLITE_OK);
            let writer = Self { conn };
            assert_eq!(writer.rows("PRAGMA journal_mode = WAL"), [vec![s("wal")]]);
            writer
        }

        /// Runs one statement to its end; its rows, integers and text only.
        fn rows(&self, sql: &str) -> Vec<Vec<Value>> {
            let api = real();
            let text = CString::new(sql).unwrap();
            let mut stmt = Prepared {
                api,
                ptr: ptr::null_mut(),
            };
            // SAFETY: an open connection and a NUL-terminated statement.
            let rc = unsafe {
                (api.prepare_v2)(self.conn, text.as_ptr(), -1, &mut stmt.ptr, ptr::null_mut())
            };
            assert_eq!(rc, SQLITE_OK, "prepare {sql}");
            // SAFETY: a prepared statement.
            let columns = unsafe { (api.column_count)(stmt.ptr) };
            let mut rows = Vec::new();
            loop {
                // SAFETY: as above.
                match unsafe { (api.step)(stmt.ptr) } {
                    SQLITE_ROW => rows.push(
                        (0..columns)
                            .map(|i| cell(api, stmt.ptr, i).unwrap())
                            .collect(),
                    ),
                    SQLITE_DONE => return rows,
                    rc => panic!("step {sql}: {rc}"),
                }
            }
        }

        fn exec(&self, sql: &str) {
            self.rows(sql);
        }
    }

    impl Drop for Writer {
        fn drop(&mut self) {
            // SAFETY: every statement was finalized.
            let rc = unsafe { (real().close)(self.conn) };
            assert_eq!(rc, SQLITE_OK);
        }
    }

    fn s(text: &str) -> Value {
        Value::String(text.to_owned())
    }

    fn with(db: &Path, suffix: &str) -> PathBuf {
        let mut path = db.as_os_str().to_owned();
        path.push(suffix);
        PathBuf::from(path)
    }

    /// The rows of an answer, sorted: the queries have no `ORDER BY`.
    fn rows_of(answer: &Answer) -> BTreeSet<String> {
        let Answer::Json(bytes) = answer else {
            panic!("busy: {answer:?}");
        };
        let value: Value = serde_json::from_slice(bytes).unwrap();
        value["rows"]
            .as_array()
            .unwrap_or_else(|| panic!("no rows: {value}"))
            .iter()
            .map(Value::to_string)
            .collect()
    }

    fn set(rows: &[&str]) -> BTreeSet<String> {
        rows.iter().map(|r| (*r).to_owned()).collect()
    }

    fn json(text: &str) -> Answer {
        Answer::Json(text.as_bytes().to_vec())
    }

    /// `session_v2` and `schedule` as `@opencode/cli` 2.2.5 makes them (`core/src/database/schema.gen.ts`), and an
    /// `account` table with a token, as the real file has.
    const SCHEMA: &[&str] = &[
        "CREATE TABLE `session_v2` (`id` text PRIMARY KEY, `project_id` text NOT NULL, `workspace_id` text, \
         `parent_id` text, `fork_session_id` text, `fork_boundary` text, `slug` text NOT NULL, \
         `directory` text NOT NULL, `path` text, `title` text, `version` text NOT NULL, `share_url` text, \
         `summary_additions` integer, `summary_deletions` integer, `summary_files` integer, `summary_diffs` text, \
         `metadata` text, `cost` real DEFAULT 0 NOT NULL, `tokens_input` integer DEFAULT 0 NOT NULL, \
         `tokens_output` integer DEFAULT 0 NOT NULL, `tokens_reasoning` integer DEFAULT 0 NOT NULL, \
         `tokens_cache_read` integer DEFAULT 0 NOT NULL, `tokens_cache_write` integer DEFAULT 0 NOT NULL, \
         `revert` text, `permission` text, `agent` text, `model` text, `time_created` integer NOT NULL, \
         `time_updated` integer NOT NULL, `time_busy` integer, `time_idle` integer, `time_viewed` integer, \
         `idle_outcome` text, `time_compacting` integer, `time_archived` integer, `time_suspended` integer, \
         `resume_attempts` integer DEFAULT 0 NOT NULL)",
        "CREATE TABLE `schedule` (`id` text PRIMARY KEY, `session_id` text, `name` text, `action` text NOT NULL, \
         `check` text, `when` text NOT NULL, `tz` text NOT NULL, `status` text DEFAULT 'active' NOT NULL, \
         `next_due_at` integer, `ends_at` integer, `remaining_runs` integer, `run_count` integer DEFAULT 0 NOT NULL, \
         `checks` integer DEFAULT 0 NOT NULL, `failures` integer DEFAULT 0 NOT NULL, `last_fired_at` integer, \
         `last_message_id` text, `last_outcome` text, `retry_at` integer, `state` text, \
         `time_created` integer NOT NULL, `time_updated` integer NOT NULL, \
         CONSTRAINT `fk_schedule_session_id_session_v2_id_fk` FOREIGN KEY (`session_id`) \
         REFERENCES `session_v2`(`id`) ON DELETE CASCADE)",
        "CREATE TABLE `account` (`id` text PRIMARY KEY, `access_token` text, `refresh_token` text)",
        "INSERT INTO account VALUES ('acc_1', 'token-never-read', 'refresh-never-read')",
    ];

    fn session(id: &str, busy: Option<i64>, idle: Option<i64>, suspended: Option<i64>) -> String {
        let n = |v: Option<i64>| v.map_or("NULL".to_owned(), |v| v.to_string());
        format!(
            "INSERT INTO session_v2 (id, project_id, slug, directory, title, version, time_created, time_updated, \
             time_busy, time_idle, time_suspended) VALUES ('{id}', 'prj_1', 'slug', 'C:\\work', 'a prompt-like title', \
             '2.2.5', 1, 1, {}, {}, {})",
            n(busy),
            n(idle),
            n(suspended)
        )
    }

    fn schedule(
        id: &str,
        session: Option<&str>,
        status: &str,
        next: Option<i64>,
        retry: Option<i64>,
    ) -> String {
        let n = |v: Option<i64>| v.map_or("NULL".to_owned(), |v| v.to_string());
        let session = session.map_or("NULL".to_owned(), |s| format!("'{s}'"));
        format!(
            "INSERT INTO schedule (id, session_id, action, \"when\", tz, status, next_due_at, retry_at, time_created, \
             time_updated) VALUES ('{id}', {session}, 'prompt text', 'every 1h', 'UTC', '{status}', {}, {}, 1, 1)",
            n(next),
            n(retry)
        )
    }

    /// A database shaped as OpenCode 2.2.5's, open in a writer.
    fn opencode_db(dir: &TempDir) -> (PathBuf, Writer) {
        let db = dir.path().join("opencode.db");
        let writer = Writer::open(&db);
        for sql in SCHEMA {
            writer.exec(sql);
        }
        (db, writer)
    }

    #[test]
    fn the_dll_loads_with_its_shim_and_a_missing_one_gives_no_sqlite() {
        assert!(api().is_some(), "winsqlite3.dll {MIN_VERSION} or later");
        assert!(load("wakeshot-no-such-sqlite.dll").is_none());
        let dir = TempDir::new("sqlite-no-dll");
        let (db, _writer) = opencode_db(&dir);
        assert_eq!(
            read(None, &db, TABLES.sql),
            json(r#"{"error":"no_sqlite"}"#)
        );
    }

    #[test]
    fn the_opencode_queries_read_working_sessions_and_due_schedules() {
        let dir = TempDir::new("sqlite-opencode");
        let (db, writer) = opencode_db(&dir);
        for sql in [
            session("ses_working", Some(2_000), Some(1_000), None),
            session("ses_first", Some(3_000), None, None),
            session("ses_idle", Some(1_000), Some(2_000), None),
            session("ses_never", None, None, None),
            // A turn cut by a shutdown: `time_busy` stays newer and `time_suspended` stays set.
            session("ses_claimed", Some(5_000), Some(4_000), Some(5_000)),
            schedule("sch_a", Some("ses_idle"), "active", Some(10_000), None),
            schedule(
                "sch_b",
                Some("ses_working"),
                "active",
                Some(10_000),
                Some(15_000),
            ),
            schedule(
                "sch_c",
                Some("ses_working"),
                "active",
                Some(20_000),
                Some(15_000),
            ),
            schedule("sch_d", None, "active", Some(30_000), None),
            schedule("sch_e", Some("ses_idle"), "paused", Some(40_000), None),
            schedule("sch_f", Some("ses_idle"), "done", None, None),
            schedule("sch_g", Some("ses_idle"), "active", None, None),
        ] {
            writer.exec(&sql);
        }

        assert_eq!(
            rows_of(&query(&db, TABLES.sql)),
            set(&[r#"["schedule"]"#, r#"["session_v2"]"#])
        );
        assert_eq!(
            rows_of(&query(&db, WORKING.sql)),
            set(&[
                r#"["ses_claimed",5000]"#,
                r#"["ses_first",3000]"#,
                r#"["ses_working",2000]"#
            ])
        );
        assert_eq!(
            rows_of(&query(&db, SCHEDULES.sql)),
            set(&[
                r#"["sch_a","ses_idle",10000]"#,
                r#"["sch_b","ses_working",15000]"#,
                r#"["sch_c","ses_working",20000]"#,
                r#"["sch_d",null,30000]"#
            ])
        );

        // The answers fed as the watch thread feeds them.
        let mut source = OpenCodeSource::new(db.clone());
        for sql in [TABLES, WORKING, SCHEDULES] {
            let Answer::Json(bytes) = query(&db, sql.sql) else {
                panic!("busy");
            };
            source.feed(&FileId::query(&db, sql.name), Feed::Whole(bytes));
        }
        let seen: Vec<(String, Activity, Option<i64>, Vec<i64>)> = source
            .sessions()
            .into_iter()
            .map(|f| {
                let Seen::ImageSince {
                    images,
                    since_wall_ms,
                } = f.liveness
                else {
                    panic!("{f:?}");
                };
                assert_eq!(
                    (images, f.label, f.format),
                    (IMAGES, LABEL, FormatHealth::Ok)
                );
                (f.key.id, f.activity, since_wall_ms, f.timers)
            })
            .collect();
        assert_eq!(
            seen,
            [
                (
                    "schedule:sch_d".to_owned(),
                    Activity::Idle,
                    None,
                    vec![30_000]
                ),
                (
                    "ses_claimed".to_owned(),
                    Activity::Working,
                    Some(5_000),
                    vec![]
                ),
                (
                    "ses_first".to_owned(),
                    Activity::Working,
                    Some(3_000),
                    vec![]
                ),
                ("ses_idle".to_owned(), Activity::Idle, None, vec![10_000]),
                (
                    "ses_working".to_owned(),
                    Activity::Working,
                    Some(2_000),
                    vec![15_000, 20_000]
                ),
            ]
        );
    }

    #[test]
    fn a_read_runs_only_while_the_agent_has_the_database_open_and_creates_no_file() {
        let dir = TempDir::new("sqlite-closed");
        let (db, writer) = opencode_db(&dir);
        writer.exec(&session("ses_first", Some(3_000), None, None));
        drop(writer);
        let (wal, shm) = (with(&db, "-wal"), with(&db, "-shm"));
        assert!(
            !wal.exists() && !shm.exists(),
            "the last close removed both"
        );
        let before = fs::read(&db).unwrap();

        assert_eq!(query(&db, WORKING.sql), Answer::Busy);
        assert!(!wal.exists() && !shm.exists());

        // As if the agent closed it between the check and the open: the shim refuses to make the `-wal`.
        let refused = run(real(), &db, WORKING.sql);
        assert!(matches!(refused, Err(Fail::Kind(_))), "{refused:?}");
        assert!(!wal.exists() && !shm.exists(), "no -wal or -shm made");
        assert_eq!(fs::read(&db).unwrap(), before, "the database is unchanged");

        fs::write(&wal, b"").unwrap();
        assert_eq!(
            query(&db, WORKING.sql),
            Answer::Busy,
            "no -shm: not open by the agent"
        );
        assert!(!shm.exists());
        fs::remove_file(&wal).unwrap();
        assert_eq!(query(&with(&db, "-missing"), WORKING.sql), Answer::Busy);

        // Why the shim: a plain read-only open makes both files and leaves them.
        let plain = run_on(real(), &db, WORKING.sql, ptr::null());
        assert_eq!(plain.map(|rows| rows.len()), Ok(1));
        assert!(wal.exists() && shm.exists(), "the default VFS made them");
    }

    #[test]
    fn a_read_takes_committed_rows_and_never_waits_for_an_open_write() {
        let dir = TempDir::new("sqlite-writer");
        let (db, writer) = opencode_db(&dir);
        writer.exec(&session("ses_first", Some(3_000), None, None));
        assert_eq!(
            rows_of(&query(&db, WORKING.sql)),
            set(&[r#"["ses_first",3000]"#])
        );

        writer.exec("BEGIN IMMEDIATE");
        writer.exec(&session("ses_second", Some(4_000), None, None));
        // The write lock stays held, so a read that waited on it would end `Busy` and `rows_of` would fail.
        assert_eq!(
            rows_of(&query(&db, WORKING.sql)),
            set(&[r#"["ses_first",3000]"#]),
            "the last committed state"
        );
        writer.exec("COMMIT");
        assert_eq!(
            rows_of(&query(&db, WORKING.sql)),
            set(&[r#"["ses_first",3000]"#, r#"["ses_second",4000]"#])
        );
    }

    #[test]
    fn a_writer_checkpoints_the_whole_wal_after_a_read() {
        let dir = TempDir::new("sqlite-checkpoint");
        let (db, writer) = opencode_db(&dir);
        for n in 0..50 {
            writer.exec(&session(&format!("ses_{n}"), Some(n + 1), None, None));
        }
        assert!(fs::metadata(with(&db, "-wal")).unwrap().len() > 0);
        assert_eq!(rows_of(&query(&db, WORKING.sql)).len(), 50);
        assert_eq!(
            writer.rows("PRAGMA wal_checkpoint(TRUNCATE)"),
            [vec![Value::from(0), Value::from(0), Value::from(0)]],
            "not busy, nothing left in the WAL"
        );
        assert_eq!(fs::metadata(with(&db, "-wal")).unwrap().len(), 0);
        assert_eq!(
            rows_of(&query(&db, WORKING.sql)).len(),
            50,
            "read from the file now"
        );
    }

    #[test]
    fn the_reader_cannot_write() {
        let dir = TempDir::new("sqlite-readonly");
        let (db, writer) = opencode_db(&dir);
        writer.exec(&session("ses_first", Some(3_000), None, None));
        assert_eq!(
            query(&db, "DELETE FROM session_v2"),
            json(r#"{"error":"step"}"#)
        );
        assert_eq!(
            query(&db, "CREATE TABLE wakeshot (x)"),
            json(r#"{"error":"step"}"#)
        );
        assert_eq!(
            writer.rows("SELECT count(*) FROM session_v2"),
            [vec![Value::from(1)]]
        );
        assert_eq!(
            writer.rows("SELECT count(*) FROM sqlite_master WHERE name = 'wakeshot'"),
            [vec![Value::from(0)]]
        );
    }

    #[test]
    fn an_answer_names_a_failure_with_a_fixed_word_only() {
        let dir = TempDir::new("sqlite-errors");
        let (db, _writer) = opencode_db(&dir);
        assert_eq!(
            query(&db, "SELECT secret_column FROM session_v2"),
            json(r#"{"error":"prepare"}"#),
            "no 'no such column' text"
        );
        assert_eq!(query(&db, ""), json(r#"{"error":"prepare"}"#));
        assert_eq!(query(&db, "SELECT 1.5"), json(r#"{"error":"type"}"#));
        assert_eq!(query(&db, "SELECT x'00'"), json(r#"{"error":"type"}"#));
        let many = format!(
            "WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n LIMIT {}) SELECT i FROM n",
            MAX_ROWS + 1
        );
        assert_eq!(query(&db, &many), json(r#"{"error":"size"}"#));
        let enough = many.replace(&(MAX_ROWS + 1).to_string(), &MAX_ROWS.to_string());
        assert_eq!(rows_of(&query(&db, &enough)).len(), MAX_ROWS);
        // `hex` of n zero bytes is 2n characters.
        let cell = |chars: usize| format!("SELECT hex(zeroblob({}))", chars / 2);
        assert_eq!(query(&db, &cell(MAX_CELL + 2)), json(r#"{"error":"size"}"#));
        assert_eq!(
            query(&db, &cell(MAX_CELL)),
            json(&format!(r#"{{"rows":[["{}"]]}}"#, "0".repeat(MAX_CELL)))
        );
        assert_eq!(
            query(&db, "SELECT 'a\"b' || char(10) || 'ü', NULL, -7"),
            json("{\"rows\":[[\"a\\\"b\\nü\",null,-7]]}"),
            "text escaped as JSON"
        );
    }

    /// Why the core's table test reads a literal after `FROM` as a table.
    #[test]
    fn sqlite_takes_a_literal_where_only_a_table_name_fits() {
        let dir = TempDir::new("sqlite-literal");
        let (db, writer) = opencode_db(&dir);
        writer.exec(&session("ses_first", Some(3_000), None, None));
        assert_eq!(
            rows_of(&query(&db, "SELECT id FROM 'session_v2'")),
            set(&[r#"["ses_first"]"#])
        );
    }

    #[test]
    fn an_empty_database_keeps_its_wal() {
        let dir = TempDir::new("sqlite-empty");
        let db = dir.write("opencode.db", b"");
        let wal = dir.write("opencode.db-wal", b"frames");
        dir.write("opencode.db-shm", b"");
        // SQLite deletes the `-wal` of an empty database file; the shim refuses, so the read fails instead.
        assert_eq!(query(&db, WORKING.sql), json(r#"{"error":"prepare"}"#));
        assert_eq!(fs::read(&wal).unwrap(), b"frames");
        assert_eq!(fs::read(&db).unwrap(), b"");
    }

    #[test]
    fn the_reader_skips_the_checkpoint_on_close() {
        let dir = TempDir::new("sqlite-no-checkpoint");
        let (db, _writer) = opencode_db(&dir);
        let conn = open_on(real(), &db, VFS_NAME.as_ptr()).unwrap();
        let mut on = -1;
        // SAFETY: an open connection; -1 reads the setting into `on`.
        let rc = unsafe {
            (real().db_config)(conn.ptr, DBCONFIG_NO_CKPT_ON_CLOSE, -1 as c_int, &mut on)
        };
        assert_eq!((rc, on), (SQLITE_OK, 1));
    }

    // Across processes: SQLite shares one `-shm` mapping per process, so only a writer in another process makes the
    // reader open the `-shm` itself, and only a killed one leaves its files behind.

    const WRITER_ARG: &str = "wakeshot-test-sqlite-writer=";
    /// A child that outlives its test (a crashed run) closes on its own after this.
    const WRITER_MAX: Duration = Duration::from_secs(120);

    /// The child role. Started as `<this exe> sqlite::tests::writer_role --exact --ignored
    /// wakeshot-test-sqlite-writer=<db>` it opens `<db>` as OpenCode does, commits one session, says `open` on stdout
    /// and closes the database when stdin closes; started any other way it ends at once. The last argument is a test
    /// filter that matches nothing, so the harness takes it.
    #[test]
    #[ignore = "a child role for the cross-process tests"]
    fn writer_role() {
        let Some(db) = std::env::args().find_map(|a| a.strip_prefix(WRITER_ARG).map(PathBuf::from))
        else {
            return;
        };
        let writer = Writer::open(&db);
        writer.exec(&session("ses_child", Some(9_000), None, None));
        // Straight to the handle: the harness captures only the print macros.
        let mut out = std::io::stdout().lock();
        out.write_all(b"open\n").unwrap();
        out.flush().unwrap();
        drop(out);
        let (done, wait) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = std::io::stdin().read_line(&mut String::new());
            let _ = done.send(());
        });
        let _ = wait.recv_timeout(WRITER_MAX);
        drop(writer);
    }

    /// A writer child, killed and reaped on drop, so a failed assert leaves no process.
    struct OtherWriter(Child);

    impl OtherWriter {
        fn start(db: &Path) -> Self {
            let mut arg = std::ffi::OsString::from(WRITER_ARG);
            arg.push(db);
            let mut child = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "sqlite::tests::writer_role",
                    "--exact",
                    "--ignored",
                    "--test-threads=1",
                    "--quiet",
                ])
                .arg(arg)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
                .unwrap();
            let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
            let writer = Self(child);
            assert!(
                lines.any(|line| line.is_ok_and(|l| l.trim() == "open")),
                "the child never opened the database"
            );
            writer
        }

        /// Closes its stdin, so it closes the database and ends by itself. Its exit code is not checked: its harness
        /// may fail to print its summary to the stdout pipe dropped in `start`.
        fn close(&mut self) {
            drop(self.0.stdin.take());
            self.0.wait().unwrap();
        }

        /// Ends it by its own handle, as a crash would: the database is never closed.
        fn kill(&mut self) {
            self.0.kill().unwrap();
            self.0.wait().unwrap();
        }
    }

    impl Drop for OtherWriter {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }

    /// A file's bytes and write time.
    fn stamp(path: &Path) -> (Vec<u8>, std::time::SystemTime) {
        (
            fs::read(path).unwrap(),
            fs::metadata(path).unwrap().modified().unwrap(),
        )
    }

    fn names(dir: &TempDir) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        names
    }

    /// A database with one session, closed, so the child is its only writer.
    fn closed_db(dir: &TempDir) -> PathBuf {
        let (db, writer) = opencode_db(dir);
        writer.exec(&session("ses_first", Some(3_000), None, None));
        db
    }

    const BOTH: [&str; 2] = [r#"["ses_child",9000]"#, r#"["ses_first",3000]"#];

    #[test]
    fn a_reader_beside_a_writer_in_another_process_and_after_it_died_writes_only_the_shm() {
        let dir = TempDir::new("sqlite-cross-kill");
        let db = closed_db(&dir);
        let (wal, shm) = (with(&db, "-wal"), with(&db, "-shm"));
        let mut child = OtherWriter::start(&db);
        let before = (stamp(&db), stamp(&wal));
        assert_eq!(rows_of(&query(&db, WORKING.sql)), set(&BOTH));
        assert_eq!(
            (stamp(&db), stamp(&wal)),
            before,
            "the reader wrote neither"
        );

        // A crash: both files stay, and a read rebuilds the `-shm` from the `-wal`, which holds the child's commit.
        child.kill();
        assert!(wal.exists() && shm.exists());
        let left = (stamp(&db), stamp(&wal));
        for _ in 0..2 {
            assert_eq!(rows_of(&query(&db, WORKING.sql)), set(&BOTH));
        }
        assert_eq!((stamp(&db), stamp(&wal)), left, "only the -shm changes");
        assert_eq!(
            names(&dir),
            ["opencode.db", "opencode.db-shm", "opencode.db-wal"]
        );
    }

    #[test]
    fn an_agent_in_another_process_that_closes_takes_its_files_and_a_read_then_makes_none() {
        let dir = TempDir::new("sqlite-cross-close");
        let db = closed_db(&dir);
        let mut child = OtherWriter::start(&db);
        assert_eq!(rows_of(&query(&db, WORKING.sql)), set(&BOTH));
        child.close();
        assert_eq!(names(&dir), ["opencode.db"], "its last close removed both");
        assert_eq!(query(&db, WORKING.sql), Answer::Busy);
        assert_eq!(names(&dir), ["opencode.db"]);
    }

    /// A process of OpenCode's image that started at the epoch, so every row time is later than it.
    fn opencode_process() -> FakeLiveness {
        let live = FakeLiveness::default();
        live.processes.lock().unwrap().push(ProcessEntry {
            pid: 4242,
            parent: 0,
            start: FileTime(116_444_736_000_000_000),
            image: "opencode.exe".to_owned(),
        });
        live
    }

    #[test]
    fn nothing_read_from_the_database_reaches_a_log() {
        const MARK: &str = "MARK3R";
        let dir = TempDir::new("sqlite-log");
        let db = dir.path().join("opencode.db");
        let writer = Writer::open(&db);
        // An older or changed `session_v2` without `time_idle`: the query fails with SQLite's "no such column" text.
        writer.exec("CREATE TABLE session_v2 (id text PRIMARY KEY, time_busy integer, title text)");
        writer.exec(&format!(
            "INSERT INTO session_v2 VALUES ('ses_{MARK}', 1793750412345, '{MARK} prompt')"
        ));
        writer.exec(
            "CREATE TABLE schedule (id text PRIMARY KEY, session_id text, status text, next_due_at integer, \
             retry_at integer)",
        );
        // A good row and one whose id is not an id.
        writer.exec(&format!(
            "INSERT INTO schedule VALUES ('sch_{MARK}', NULL, 'active', 1793750467890, NULL)"
        ));
        writer.exec(&format!(
            "INSERT INTO schedule VALUES ('{MARK} not an id', NULL, 'active', 1793750467891, NULL)"
        ));

        let log = Arc::new(MemoryLog::default());
        let source = OpenCodeSource::new(db.clone());
        let mut agents = Agents::new(vec![Box::new(source)], log.clone());
        agents.on_tick();
        let now = Now {
            wall_ms: 1_793_750_500_000,
            mono_ms: 3_600_000,
        };
        let snapshots = agents.snapshots(&opencode_process(), now);
        let [Event::Sources { source, sessions }] = snapshots.events.as_slice() else {
            panic!("{:?}", snapshots.events);
        };

        // The watch thread's log, and the core's log once it takes the snapshot, checked first.
        let watch_lines: Vec<String> = log
            .lines
            .lock()
            .unwrap()
            .iter()
            .map(|l| l.2.clone())
            .collect();
        let mut core = Core::new(Config::default());
        let tray = ProcessRef {
            pid: 1,
            start: FileTime(1),
        };
        let mut commands = core.handle(
            now,
            Event::Started {
                config: Config::default(),
                journal: None,
                journal_unread: false,
                owner: true,
                on_console: true,
                tray,
                first_run: false,
            },
        );
        commands.extend(core.handle(now, snapshots.events[0].clone()));
        commands.extend(core.handle(now, Event::Tick));
        let core_lines: Vec<String> = commands
            .into_iter()
            .filter_map(|c| match c {
                Command::Log(text) => Some(text),
                _ => None,
            })
            .collect();
        for text in watch_lines.iter().chain(&core_lines) {
            for leak in [
                MARK,
                "prompt",
                "no such column",
                "time_idle",
                "12345",
                "67890",
            ] {
                assert!(!text.contains(leak), "{leak:?} in {text:?}");
            }
        }
        assert_eq!(watch_lines, ["OpenCode: format problem in opencode.db"]);
        assert!(
            core_lines
                .iter()
                .any(|l| l.contains("cannot read opencode.db")),
            "{core_lines:?}"
        );

        assert_eq!(*source, SourceId::OpenCode);
        let problem = SessionFacts {
            key: SessionKey {
                source: SourceId::OpenCode,
                id: PROBLEM_KEY.to_owned(),
            },
            liveness: Seen::ImageSince {
                images: IMAGES,
                since_wall_ms: None,
            },
            activity: Activity::Idle,
            timers: vec![],
            open_tasks: 0,
            label: LABEL,
            format: FormatHealth::Unreadable(UNREADABLE),
        };
        assert_eq!(
            sessions.last(),
            Some(&(problem, Alive::Yes)),
            "the read problem is reported"
        );
        assert_eq!(sessions.len(), 2, "the good schedule and the problem");
        drop(writer);
    }
}
