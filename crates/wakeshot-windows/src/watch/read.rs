//! Whole reads, tails, finds and name lists: the I/O behind each `WantedFile` kind, so the watch thread can feed any
//! source.
//!
//! Every open here shares read, write and delete, so a read never blocks an agent's own write, rename or delete
//! (Claude Code deletes its status file when the session ends), and no handle outlives the call that opened it.
//! [`names`], [`find`], [`recent_files`] and [`Finder::newest`] take names, types, sizes and times from the folder
//! listing alone and open no file; only [`Finder`]'s check that a cached path is still there opens it, for attributes
//! only (std's `fs::metadata`: no read or write access, all sharing).

use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom};
use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
use std::os::windows::io::AsRawHandle;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use wakeshot_core::process::FileTime;
use windows::Win32::Foundation::HANDLE;
use windows::Win32::Storage::FileSystem::{
    FILE_ID_INFO, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, FileIdInfo,
    GetFileInformationByHandleEx,
};

/// The largest file [`read_whole`] takes. A Claude status file is under 2 KB and a hold file is smaller, so a bigger
/// file is not one of the files it serves.
pub const WHOLE_CAP: u64 = 1 << 20;

/// The most new bytes one [`Tail::poll`] reads, so the first poll of a long transcript never holds it all in memory.
pub const POLL_CAP: u64 = 8 << 20;

/// The largest file a `Recent` want reads whole. A VS Code chat session holds whole answers and tool output and is
/// compacted only after 1024 lines, so it can pass [`WHOLE_CAP`] by far; its size in real use is not known (no
/// session that large was seen). Change it when a live session file is larger.
pub const RECENT_CAP: u64 = 64 << 20;

/// How many [`Finder`] results stay cached. Each live session adds one; past this many the cache starts over, which
/// costs one walk per session and keeps a tray that runs for weeks from growing.
const FINDER_CAP: usize = 256;

/// A file's size and last write time (a FILETIME), to tell a changed file from one that is not without reading it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stamp {
    pub len: u64,
    pub written: u64,
}

impl Stamp {
    fn of(meta: &fs::Metadata) -> Self {
        Self {
            len: meta.file_size(),
            written: meta.last_write_time(),
        }
    }
}

/// Reads a whole file of at most [`WHOLE_CAP`] bytes, if `accept` passes its file name.
///
/// The name is checked here, at the open, even though the caller filtered names before: so no later change to how a
/// path is built can open a file a source must never read (Claude's `.key` files). Errors: `InvalidInput` for a
/// refused name, `FileTooLarge` over the cap, `NotFound` when the file is gone, and the open's own error otherwise
/// (a writer that shares no read gives a sharing violation at once, never a wait).
pub fn read_whole(path: &Path, accept: impl Fn(&str) -> bool) -> io::Result<Vec<u8>> {
    read_whole_written(path, accept).map(|(bytes, _)| bytes)
}

/// [`read_whole`] plus the file's last write time, taken from the same handle, so the time belongs to the bytes read
/// (hold files: a `procStart`-less hold compares it with the process start).
pub fn read_whole_written(
    path: &Path,
    accept: impl Fn(&str) -> bool,
) -> io::Result<(Vec<u8>, FileTime)> {
    read_whole_stamped(path, accept, WHOLE_CAP)
        .map(|(bytes, stamp)| (bytes, FileTime(stamp.written)))
}

/// [`read_whole`] with its own cap, plus the file's [`Stamp`] from the same handle.
pub fn read_whole_stamped(
    path: &Path,
    accept: impl Fn(&str) -> bool,
    cap: u64,
) -> io::Result<(Vec<u8>, Stamp)> {
    check_name(path, &accept)?;
    let file = open_shared(path)?;
    let meta = file.metadata()?;
    if meta.len() > cap {
        return Err(too_large());
    }
    let mut bytes = Vec::with_capacity(usize::try_from(meta.len()).unwrap_or(0));
    // One byte past the cap tells a file that grew after the size check from one that fits exactly.
    file.take(cap + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > cap {
        return Err(too_large());
    }
    Ok((bytes, Stamp::of(&meta)))
}

fn check_name(path: &Path, accept: &impl Fn(&str) -> bool) -> io::Result<()> {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    // Win32 drops trailing dots and spaces from the last name and reads `name:stream` as a stream of `name`, so a name
    // with either could open a file other than the one `accept` saw.
    let plain = !name.is_empty() && !name.contains(':') && !name.ends_with(['.', ' ']);
    if plain && accept(name) {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "file name refused",
        ))
    }
}

fn too_large() -> io::Error {
    io::Error::new(io::ErrorKind::FileTooLarge, "file over the read cap")
}

fn open_shared(path: &Path) -> io::Result<File> {
    // std shares all three by default today; spelled out so a std change cannot start blocking an agent's delete.
    OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ.0 | FILE_SHARE_WRITE.0 | FILE_SHARE_DELETE.0)
        .open(path)
}

/// Which file a path named when it was read. A file replaced under the same name (written to a temp name, then
/// renamed over) gets a new id; its creation time can stay the same (NTFS tunneling), so the id is what is compared.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FileKey {
    volume: u64,
    id: [u8; 16],
}

/// `None` on a file system without file ids (FAT): the tail then knows a replaced file only by its length.
fn file_key(file: &File) -> Option<FileKey> {
    let mut info = FILE_ID_INFO::default();
    // SAFETY: the handle stays open while `file` is borrowed; the pointer and the size describe one live local of the
    // type `FileIdInfo` fills.
    unsafe {
        GetFileInformationByHandleEx(
            HANDLE(file.as_raw_handle()),
            FileIdInfo,
            std::ptr::from_mut(&mut info).cast(),
            size_of::<FILE_ID_INFO>() as u32,
        )
    }
    .ok()?;
    Some(FileKey {
        volume: info.VolumeSerialNumber,
        id: info.FileId.Identifier,
    })
}

/// Follows an append-only file (a transcript) and hands over complete lines only.
///
/// A new `Tail` starts at byte 0, so after a Wakeshot restart a source replays the file from the start
/// (`docs/research/claude-code-signals.md`, detection rule 5). The handle is opened and closed on each poll.
#[derive(Debug)]
pub struct Tail {
    path: PathBuf,
    offset: u64,
    partial: Vec<u8>,
    key: Option<FileKey>,
    seen: bool,
}

/// One [`Tail::poll`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TailRead {
    /// Complete lines as the file has them, each with its `\n` (and a `\r` before it if the file has one); empty
    /// when nothing new is complete. A last line without its `\n` stays held back for a later poll.
    pub lines: Vec<u8>,
    /// The file got shorter or is a new file under the same name: `lines` start again from byte 0, so the source must
    /// forget what earlier polls gave.
    pub restarted: bool,
    /// This poll stopped at [`POLL_CAP`]: poll again for the rest.
    pub more: bool,
}

impl Tail {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            offset: 0,
            partial: Vec::new(),
            key: None,
            seen: false,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Reads what the file gained since the last poll. `NotFound` when the file is gone; the tail keeps its place, so
    /// the same file coming back goes on, and a new file under the name restarts.
    pub fn poll(&mut self) -> io::Result<TailRead> {
        let mut file = open_shared(&self.path)?;
        let key = file_key(&file);
        let len = file.metadata()?.len();
        let restarted = self.seen && (key != self.key || len < self.offset);
        if restarted {
            self.offset = 0;
            self.partial.clear();
        }
        self.seen = true;
        self.key = key;
        let mut read = TailRead {
            restarted,
            ..TailRead::default()
        };
        if len == self.offset {
            return Ok(read);
        }
        file.seek(SeekFrom::Start(self.offset))?;
        let start = self.partial.len();
        let got = match (&mut file).take(POLL_CAP).read_to_end(&mut self.partial) {
            Ok(got) => got as u64,
            Err(e) => {
                // A failed read may have appended some bytes; the offset did not move, so the next poll reads them.
                self.partial.truncate(start);
                return Err(e);
            }
        };
        self.offset += got;
        if let Some(last) = self.partial[start..].iter().rposition(|&b| b == b'\n') {
            let rest = self.partial.split_off(start + last + 1);
            read.lines = std::mem::replace(&mut self.partial, rest);
        }
        read.more = got == POLL_CAP;
        Ok(read)
    }
}

/// What a walk found, and whether a folder on its way could not be listed: the first such error's code (`Some(None)`
/// for one without a code). A missing folder is no error.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Walked<T> {
    pub found: T,
    pub unlisted: Option<Option<i32>>,
}

impl<T> Walked<T> {
    /// Keeps the first listing error but a missing folder's.
    fn note(&mut self, e: &io::Error) {
        if e.kind() != io::ErrorKind::NotFound && self.unlisted.is_none() {
            self.unlisted = Some(e.raw_os_error());
        }
    }
}

/// The files `root\<depth folder levels>\*<name_suffix>`, newest folders first, names compared without case. Folder
/// links (symlinks, junctions) are not followed, so a link loop cannot trap the walk. An unreadable folder counts as
/// empty, and its error goes to `unlisted`.
pub fn find(root: &Path, depth: u8, name_suffix: &str) -> Walked<Vec<PathBuf>> {
    let mut walked = Walked::default();
    walk(root, depth, &name_suffix.to_lowercase(), &mut walked);
    walked
}

fn walk(dir: &Path, depth: u8, suffix: &str, out: &mut Walked<Vec<PathBuf>>) {
    let mut entries = match list(dir) {
        Ok(entries) => entries,
        Err(e) => {
            out.note(&e);
            return;
        }
    };
    entries.sort_by(|a, b| {
        b.modified
            .cmp(&a.modified)
            .then_with(|| a.name.cmp(&b.name))
    });
    for entry in entries {
        if depth == 0 {
            if entry.is_file && entry.name.to_lowercase().ends_with(suffix) {
                out.found.push(entry.path);
            }
        } else if entry.is_dir {
            walk(&entry.path, depth - 1, suffix, out);
        }
    }
}

#[derive(Debug)]
struct Entry {
    path: PathBuf,
    name: String,
    is_file: bool,
    is_dir: bool,
    modified: SystemTime,
}

/// Every file at `depth` folder levels under `dir`, from the listings alone.
fn walk_files(dir: &Path, depth: u8, out: &mut Walked<Vec<Entry>>) {
    let entries = match list(dir) {
        Ok(entries) => entries,
        Err(e) => {
            out.note(&e);
            return;
        }
    };
    for entry in entries {
        if depth == 0 {
            if entry.is_file {
                out.found.push(entry);
            }
        } else if entry.is_dir {
            walk_files(&entry.path, depth - 1, out);
        }
    }
}

/// The folder's entries from the listing alone: on Windows, `DirEntry::file_type` and `DirEntry::metadata` come from
/// the folder enumeration, so no file is opened.
fn list(dir: &Path) -> io::Result<Vec<Entry>> {
    let mut entries = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        let modified = entry
            .metadata()
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);
        entries.push(Entry {
            path: entry.path(),
            name: entry.file_name().to_string_lossy().into_owned(),
            is_file: kind.is_file(),
            is_dir: kind.is_dir(),
            modified,
        });
    }
    Ok(entries)
}

/// [`find`] with a cache, so the tick does not walk the same folders every 30 s: a want walks again only while its
/// file is not found or once a cached path is gone. A new match elsewhere is not seen while the cached ones last,
/// which suits names that are unique per session (a Claude session id).
///
/// [`newest`](Self::newest) must see a new match at once (a reverted Codex thread's new file), so it walks on each
/// call, but once per root and depth until [`forget_walks`](Self::forget_walks): every want of one call shares it.
#[derive(Debug, Default)]
pub struct Finder {
    found: HashMap<(PathBuf, u8, String), Vec<PathBuf>>,
    walks: HashMap<(PathBuf, u8), Walked<Vec<Entry>>>,
}

impl Finder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Of the files `root\<depth folder levels>\<name>` with `matches(name)`, the newest by modified time (from the
    /// listing), the greater name on a tie. Folder links are not followed; an unreadable folder counts as empty, and
    /// its error goes to `unlisted`.
    pub fn newest(
        &mut self,
        root: &Path,
        depth: u8,
        matches: impl Fn(&str) -> bool,
    ) -> Walked<Option<PathBuf>> {
        let walk = self
            .walks
            .entry((root.to_path_buf(), depth))
            .or_insert_with(|| {
                let mut walk = Walked::default();
                walk_files(root, depth, &mut walk);
                walk
            });
        let found = walk
            .found
            .iter()
            .filter(|e| matches(&e.name))
            .max_by(|a, b| {
                a.modified
                    .cmp(&b.modified)
                    .then_with(|| a.name.cmp(&b.name))
            })
            .map(|e| e.path.clone());
        Walked {
            found,
            unlisted: walk.unlisted,
        }
    }

    /// Drops the walks [`newest`](Self::newest) keeps, so the next call lists the folders again.
    pub fn forget_walks(&mut self) {
        self.walks.clear();
    }

    /// [`find`], or the cached paths with no walk and so no `unlisted`.
    pub fn find(&mut self, root: &Path, depth: u8, name_suffix: &str) -> Walked<Vec<PathBuf>> {
        let key = (root.to_path_buf(), depth, name_suffix.to_lowercase());
        if let Some(paths) = self.found.get(&key)
            && !paths.is_empty()
            && paths.iter().all(|p| p.is_file())
        {
            return Walked {
                found: paths.clone(),
                unlisted: None,
            };
        }
        let walked = find(root, depth, name_suffix);
        if self.found.len() >= FINDER_CAP && !self.found.contains_key(&key) {
            self.found.clear();
        }
        self.found.insert(key, walked.found.clone());
        walked
    }
}

/// The files `dir\<depth folder levels>\<name>` whose name `accept` passes, sorted by path. Names only: no file is
/// opened, so a lock file another process holds is never disturbed. A missing `dir` lists nothing; a subfolder that
/// goes away during the walk is skipped. Any other error is returned, so the caller keeps its last list instead of
/// reading every name as gone.
pub fn names(dir: &Path, depth: u8, accept: impl Fn(&str) -> bool) -> io::Result<Vec<PathBuf>> {
    let mut found = Vec::new();
    match collect_names(dir, depth, &accept, &mut found) {
        // Only the top folder's own listing can end in `NotFound`: a subfolder's is skipped inside.
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        other => other?,
    }
    found.sort();
    Ok(found)
}

fn collect_names(
    dir: &Path,
    depth: u8,
    accept: &impl Fn(&str) -> bool,
    found: &mut Vec<PathBuf>,
) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if depth == 0 {
            if kind.is_file()
                && let Some(name) = entry.file_name().to_str()
                && accept(name)
            {
                found.push(entry.path());
            }
        } else if kind.is_dir() {
            match collect_names(&entry.path(), depth - 1, accept, found) {
                Err(e) if e.kind() == io::ErrorKind::NotFound => {}
                other => other?,
            }
        }
    }
    Ok(())
}

/// What [`recent_files`] found.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Recent {
    /// The files and their stamps, sorted by path.
    pub files: Vec<(PathBuf, Stamp)>,
    /// Folders under the root that could not be listed, with the error code: their files are not known, so they are
    /// not gone either.
    pub failed: Vec<(PathBuf, Option<i32>)>,
}

/// The files `root\<depth folder levels>\<sub>\<name>` (no `sub` level when `None`) whose name `accept` passes, with
/// their stamps, from the listings alone: no file is opened. A missing `root`, a missing folder on the way and a
/// subfolder that goes during the walk list nothing. A `root` that cannot be listed is an error, so the caller keeps
/// what it knew; a deeper folder that cannot be listed goes to `failed`.
pub fn recent_files(
    root: &Path,
    depth: u8,
    sub: Option<&str>,
    accept: impl Fn(&str) -> bool,
) -> io::Result<Recent> {
    let mut out = Recent::default();
    let top = if depth == 0 {
        sub.map_or_else(|| root.to_path_buf(), |sub| root.join(sub))
    } else {
        root.to_path_buf()
    };
    match recent_level(&top, depth, sub, &accept, &mut out) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Recent::default()),
        other => other?,
    }
    out.files.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(out)
}

/// An error only for `dir`'s own listing; a deeper folder that fails goes to `failed`.
fn recent_level(
    dir: &Path,
    depth: u8,
    sub: Option<&str>,
    accept: &impl Fn(&str) -> bool,
    out: &mut Recent,
) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if depth == 0 {
            if kind.is_file()
                && let Some(name) = entry.file_name().to_str()
                && accept(name)
            {
                out.files
                    .push((entry.path(), Stamp::of(&entry.metadata()?)));
            }
            continue;
        }
        if !kind.is_dir() {
            continue;
        }
        let next = match (depth, sub) {
            (1, Some(sub)) => entry.path().join(sub),
            _ => entry.path(),
        };
        match recent_level(&next, depth - 1, sub, accept, out) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => out.failed.push((next, e.raw_os_error())),
        }
    }
    Ok(())
}

/// A fresh folder under the temp folder, removed on drop. Shared with the other watch tests in this crate.
#[cfg(test)]
pub(crate) struct TempDir(PathBuf);

#[cfg(test)]
impl TempDir {
    pub(crate) fn new(tag: &str) -> Self {
        use std::sync::atomic::{AtomicU32, Ordering};
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let nanos = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        let path = std::env::temp_dir().join(format!(
            "wakeshot-watch-{tag}-{}-{n}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    pub(crate) fn path(&self) -> &Path {
        &self.0
    }

    /// Writes `bytes` to `rel` under the folder, creating its folders, and returns the full path.
    pub(crate) fn write(&self, rel: &str, bytes: &[u8]) -> PathBuf {
        let path = self.0.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, bytes).unwrap();
        path
    }
}

#[cfg(test)]
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;
    use std::time::Duration;

    use windows::Win32::Foundation::{ERROR_LOCK_VIOLATION, ERROR_SHARING_VIOLATION};
    use windows::Win32::Storage::FileSystem::{
        FILE_FLAG_BACKUP_SEMANTICS, FILE_WRITE_ATTRIBUTES, LOCKFILE_EXCLUSIVE_LOCK,
        LOCKFILE_FAIL_IMMEDIATELY, LockFileEx, UnlockFileEx,
    };
    use windows::Win32::System::IO::OVERLAPPED;

    use super::*;

    const SHARE_ALL: u32 = FILE_SHARE_READ.0 | FILE_SHARE_WRITE.0 | FILE_SHARE_DELETE.0;

    /// The Claude status file rule: `<digits>.json` and nothing else.
    fn status_name(name: &str) -> bool {
        name.strip_suffix(".json")
            .is_some_and(|stem| !stem.is_empty() && stem.bytes().all(|b| b.is_ascii_digit()))
    }

    fn any_json(name: &str) -> bool {
        name.ends_with(".json")
    }

    fn os_code(e: &io::Error) -> Option<u32> {
        e.raw_os_error().and_then(|c| u32::try_from(c).ok())
    }

    /// A writer that shares read, write and delete, the most an agent's writer can allow a reader.
    fn sharing_writer(path: &Path) -> File {
        OpenOptions::new()
            .write(true)
            .share_mode(SHARE_ALL)
            .open(path)
            .unwrap()
    }

    fn lock(file: &File) -> windows::core::Result<()> {
        let mut overlapped = OVERLAPPED::default();
        // SAFETY: the handle stays open while `file` is borrowed; the OVERLAPPED is a live local and the handle is
        // synchronous, so the call is done with it when it returns.
        unsafe {
            LockFileEx(
                HANDLE(file.as_raw_handle()),
                LOCKFILE_EXCLUSIVE_LOCK | LOCKFILE_FAIL_IMMEDIATELY,
                None,
                u32::MAX,
                u32::MAX,
                &mut overlapped,
            )
        }
    }

    fn unlock(file: &File) -> windows::core::Result<()> {
        let mut overlapped = OVERLAPPED::default();
        // SAFETY: as in `lock`.
        unsafe {
            UnlockFileEx(
                HANDLE(file.as_raw_handle()),
                None,
                u32::MAX,
                u32::MAX,
                &mut overlapped,
            )
        }
    }

    fn set_modified(path: &Path, when: SystemTime) {
        let file = OpenOptions::new()
            .access_mode(FILE_WRITE_ATTRIBUTES.0)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS.0)
            .open(path)
            .unwrap();
        file.set_modified(when).unwrap();
    }

    fn days_ago(days: u64) -> SystemTime {
        SystemTime::now() - Duration::from_secs(days * 86_400)
    }

    #[test]
    fn whole_read_shares_with_a_writer_that_writes_renames_and_deletes() {
        let dir = TempDir::new("whole-share");
        let path = dir.write("20636.json", b"{\"status\":\"busy\"}");

        // A writer holds the file open while Wakeshot reads.
        let mut writer = sharing_writer(&path);
        assert_eq!(
            read_whole(&path, status_name).unwrap(),
            b"{\"status\":\"busy\"}"
        );

        // And while Wakeshot's own handle is open, the writer still writes, renames over and deletes.
        let ours = open_shared(&path).unwrap();
        writer.set_len(0).unwrap();
        writer.write_all(b"{\"status\":\"idle\"}").unwrap();
        drop(writer);
        let next = dir.write("20636.json.tmp", b"{\"status\":\"waiting\"}");
        fs::rename(&next, &path).unwrap();
        drop(ours);
        assert_eq!(
            read_whole(&path, status_name).unwrap(),
            b"{\"status\":\"waiting\"}"
        );

        let ours = open_shared(&path).unwrap();
        fs::remove_file(&path).unwrap();
        drop(ours);
        let gone = read_whole(&path, status_name).unwrap_err();
        assert_eq!(gone.kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn whole_read_fails_at_once_while_a_writer_shares_no_read() {
        let dir = TempDir::new("whole-exclusive");
        let path = dir.write("1.json", b"{}");
        let _writer = OpenOptions::new()
            .write(true)
            .share_mode(0)
            .open(&path)
            .unwrap();
        let err = read_whole(&path, status_name).unwrap_err();
        assert_eq!(os_code(&err), Some(ERROR_SHARING_VIOLATION.0));
    }

    #[test]
    fn name_guard_refuses_a_key_file_even_with_a_crafted_path() {
        let dir = TempDir::new("whole-guard");
        let status = dir.write("20636.json", b"{\"pid\":20636}");
        let key = dir.write("20636.685bf0d758162e18.key", b"secret");
        // Held with no sharing, so any open of the key file, even one that reads nothing, fails with a sharing
        // violation: an `InvalidInput` below means the guard refused before the open.
        let _held = OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(&key)
            .unwrap();
        let d = dir.path().display();
        let crafted = [
            key.clone(),
            PathBuf::from(format!(r"{d}\20636.json\..\20636.685bf0d758162e18.key")),
            PathBuf::from(format!(r"{d}\20636.685bf0d758162e18.key\.")),
            PathBuf::from(format!(r"{d}/20636.685bf0d758162e18.key")),
        ];
        for path in &crafted {
            let err = read_whole(path, status_name).unwrap_err();
            assert_eq!(
                err.kind(),
                io::ErrorKind::InvalidInput,
                "{}",
                path.display()
            );
        }
        // A filter that only refuses `.key` endings: Win32 would drop the trailing dot or space, or open a stream of
        // the key file, so the guard refuses those names itself.
        let not_key = |name: &str| !name.ends_with(".key");
        for tail in [".key.", ".key ", ".key::$DATA", ".key:x.json"] {
            let path = PathBuf::from(format!(r"{d}\20636.685bf0d758162e18{tail}"));
            let err = read_whole(&path, not_key).unwrap_err();
            assert_eq!(
                err.kind(),
                io::ErrorKind::InvalidInput,
                "{}",
                path.display()
            );
        }
        let err = read_whole(&PathBuf::from(format!(r"{d}\20636.json\..")), any_json).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);

        assert_eq!(
            read_whole(&status, status_name).unwrap(),
            b"{\"pid\":20636}"
        );
        // The name the filter sees is the one Windows opens: the key folder name before `..` is not read.
        let through = PathBuf::from(format!(r"{d}\20636.685bf0d758162e18.key\..\20636.json"));
        assert_eq!(
            read_whole(&through, status_name).unwrap(),
            b"{\"pid\":20636}"
        );
    }

    #[test]
    fn whole_read_stops_at_the_cap() {
        let dir = TempDir::new("whole-cap");
        let cap = usize::try_from(WHOLE_CAP).unwrap();
        let fits = dir.write("1.json", &vec![b' '; cap]);
        assert_eq!(read_whole(&fits, status_name).unwrap().len(), cap);
        let over = dir.write("2.json", &vec![b' '; cap + 1]);
        assert_eq!(
            read_whole(&over, status_name).unwrap_err().kind(),
            io::ErrorKind::FileTooLarge
        );
        let empty = dir.write("3.json", b"");
        assert!(read_whole(&empty, status_name).unwrap().is_empty());
    }

    #[test]
    fn whole_read_gives_the_write_time_of_the_bytes_read() {
        let dir = TempDir::new("whole-written");
        let path = dir.write("build.json", b"{\"version\":1,\"pid\":4}");
        let (bytes, written) = read_whole_written(&path, any_json).unwrap();
        assert_eq!(bytes, b"{\"version\":1,\"pid\":4}");
        assert_eq!(
            written,
            FileTime(fs::metadata(&path).unwrap().last_write_time())
        );
        assert!(written.0 > 0);
    }

    #[test]
    fn tail_hands_over_complete_lines_only_for_lf_and_crlf() {
        let dir = TempDir::new("tail-lines");
        let path = dir.write("t.jsonl", b"");
        let mut tail = Tail::new(path.clone());
        assert_eq!(tail.poll().unwrap(), TailRead::default());

        let mut w = OpenOptions::new().append(true).open(&path).unwrap();
        w.write_all(b"{\"a\":1}\r\n{\"b\"").unwrap();
        assert_eq!(tail.poll().unwrap().lines, b"{\"a\":1}\r\n");
        w.write_all(b":2}\n{\"c\":3}\r").unwrap();
        // The `\r` alone does not end a line: it stays held back with its line.
        assert_eq!(tail.poll().unwrap().lines, b"{\"b\":2}\n");
        w.write_all(b"\n").unwrap();
        assert_eq!(tail.poll().unwrap().lines, b"{\"c\":3}\r\n");
        assert_eq!(tail.poll().unwrap(), TailRead::default());
        w.write_all(b"{\"d\":4}\n{\"e\":5}\n{\"f\"").unwrap();
        let read = tail.poll().unwrap();
        assert_eq!(read.lines, b"{\"d\":4}\n{\"e\":5}\n");
        assert!(!read.restarted && !read.more);
    }

    #[test]
    fn tail_first_poll_reads_from_the_start() {
        let dir = TempDir::new("tail-start");
        let path = dir.write("t.jsonl", b"{\"old\":1}\n{\"old\":2}\n");
        let mut tail = Tail::new(path);
        let read = tail.poll().unwrap();
        assert_eq!(read.lines, b"{\"old\":1}\n{\"old\":2}\n");
        assert!(!read.restarted, "a first read is not a restart");
    }

    #[test]
    fn tail_restarts_when_the_file_gets_shorter() {
        let dir = TempDir::new("tail-short");
        let path = dir.write("t.jsonl", b"{\"a\":1}\n{\"b\":2}\n{\"c\"");
        let mut tail = Tail::new(path.clone());
        assert_eq!(tail.poll().unwrap().lines, b"{\"a\":1}\n{\"b\":2}\n");

        // Truncated in place: same file id, fewer bytes.
        let mut w = sharing_writer(&path);
        w.set_len(0).unwrap();
        w.write_all(b"{\"z\":9}\n").unwrap();
        drop(w);
        let read = tail.poll().unwrap();
        assert!(read.restarted);
        assert_eq!(
            read.lines, b"{\"z\":9}\n",
            "the held-back partial line is dropped"
        );
        assert!(!tail.poll().unwrap().restarted);
    }

    #[test]
    fn tail_restarts_when_the_file_is_replaced() {
        let dir = TempDir::new("tail-replace");
        let path = dir.write("t.jsonl", b"{\"a\":1}\n");
        let mut tail = Tail::new(path.clone());
        assert_eq!(tail.poll().unwrap().lines, b"{\"a\":1}\n");

        // Longer than the old file, so only the file id shows the change.
        let next = dir.write("t.jsonl.tmp", b"{\"new\":1}\n{\"new\":2}\n");
        fs::rename(&next, &path).unwrap();
        let read = tail.poll().unwrap();
        assert!(read.restarted);
        assert_eq!(read.lines, b"{\"new\":1}\n{\"new\":2}\n");
    }

    #[test]
    fn tail_waits_for_a_missing_file_and_restarts_on_a_new_one() {
        let dir = TempDir::new("tail-missing");
        let path = dir.path().join("t.jsonl");
        let mut tail = Tail::new(path.clone());
        assert_eq!(tail.poll().unwrap_err().kind(), io::ErrorKind::NotFound);

        fs::write(&path, b"{\"a\":1}\n").unwrap();
        let read = tail.poll().unwrap();
        assert_eq!(read.lines, b"{\"a\":1}\n");
        assert!(!read.restarted, "nothing was read before");

        fs::remove_file(&path).unwrap();
        assert_eq!(tail.poll().unwrap_err().kind(), io::ErrorKind::NotFound);
        fs::write(&path, b"{\"b\":2}\n{\"c\":3}\n").unwrap();
        let read = tail.poll().unwrap();
        assert!(read.restarted, "a new file under the name starts over");
        assert_eq!(read.lines, b"{\"b\":2}\n{\"c\":3}\n");
    }

    #[test]
    fn tail_reads_a_10_mb_file_in_capped_polls() {
        let dir = TempDir::new("tail-big");
        let mut content = Vec::new();
        let mut n = 0u32;
        while content.len() < 10 << 20 {
            writeln!(content, "{{\"n\":{n},\"pad\":\"{}\"}}", "x".repeat(200)).unwrap();
            n += 1;
        }
        let path = dir.write("t.jsonl", &content);
        let mut tail = Tail::new(path);
        let mut got = Vec::new();
        let mut polls = 0;
        loop {
            let read = tail.poll().unwrap();
            polls += 1;
            assert!(read.lines.is_empty() || read.lines.ends_with(b"\n"));
            assert!(read.lines.len() as u64 <= POLL_CAP + 300);
            got.extend_from_slice(&read.lines);
            if !read.more {
                break;
            }
        }
        assert_eq!(polls, 2);
        assert_eq!(got.len(), content.len());
        assert!(got == content);
    }

    #[test]
    fn find_takes_exactly_one_level_with_decoys() {
        let dir = TempDir::new("find-1");
        let sid = "8e1f6c2a-0b7d-4c55-9a43-1f2e3d4c5b6a.jsonl";
        let old = dir.write(&format!("C--old-project/{sid}"), b"");
        let new = dir.write(&format!("C--new-project/{}", sid.to_uppercase()), b"");
        dir.write(sid, b"");
        dir.write(&format!("C--a/sub/{sid}"), b"");
        dir.write(&format!("C--b/{sid}.bak"), b"");
        dir.write(&format!("C--c/other-{sid}x"), b"");
        fs::create_dir_all(dir.path().join("C--d").join(sid)).unwrap();
        set_modified(&dir.path().join("C--old-project"), days_ago(9));
        set_modified(&dir.path().join("C--new-project"), days_ago(1));

        assert_eq!(find(dir.path(), 1, sid).found, vec![new, old]);
        assert_eq!(
            find(&dir.path().join("missing"), 1, sid),
            Walked::default(),
            "a missing folder is no error"
        );
    }

    #[test]
    fn a_walk_tells_a_folder_it_could_not_list() {
        let dir = TempDir::new("find-unlisted");
        let file = dir.write("not-a-folder", b"");
        let walked = find(&file, 1, "s.jsonl");
        assert!(walked.found.is_empty());
        assert!(matches!(walked.unlisted, Some(Some(_))), "{walked:?}");

        let mut finder = Finder::new();
        let newest = finder.newest(&file, 3, |_| true);
        assert_eq!(newest.found, None);
        assert_eq!(newest.unlisted, walked.unlisted);
        assert_eq!(
            finder.newest(&dir.path().join("missing"), 3, |_| true),
            Walked::default()
        );
    }

    #[test]
    fn find_takes_exactly_three_levels_with_decoys() {
        let dir = TempDir::new("find-3");
        let id = "019a4b2c-7d8e-7f01-a2b3-c4d5e6f70819.jsonl";
        let older = dir.write(&format!("2026/10/31/rollout-2026-10-31T09-00-00-{id}"), b"");
        let newer = dir.write(&format!("2026/11/03/rollout-2026-11-03T08-00-00-{id}"), b"");
        dir.write(&format!("2026/11/rollout-{id}"), b"");
        dir.write(&format!("2026/11/03/x/rollout-{id}"), b"");
        dir.write("2026/11/03/rollout-2026-11-03T08-00-00-another.jsonl", b"");
        set_modified(&dir.path().join("2026/10/31"), days_ago(3));
        set_modified(&dir.path().join("2026/11/03"), days_ago(0));
        set_modified(&dir.path().join("2026/10"), days_ago(3));
        set_modified(&dir.path().join("2026/11"), days_ago(0));

        assert_eq!(find(dir.path(), 3, id).found, vec![newer, older]);
        assert!(
            find(dir.path(), 2, id).found.len() == 1,
            "depth 2 sees only the decoy one level up"
        );
    }

    #[test]
    fn finder_walks_again_only_when_not_found_or_gone() {
        let dir = TempDir::new("finder");
        let mut finder = Finder::new();
        assert!(finder.find(dir.path(), 1, "s.jsonl").found.is_empty());

        let first = dir.write("a/s.jsonl", b"");
        assert_eq!(
            finder.find(dir.path(), 1, "s.jsonl").found,
            vec![first.clone()],
            "not found walks again"
        );

        // A second match appears; the cached answer still stands, so the walk is skipped.
        let second = dir.write("b/s.jsonl", b"");
        assert_eq!(
            finder.find(dir.path(), 1, "S.JSONL").found,
            vec![first.clone()]
        );

        fs::remove_file(&first).unwrap();
        assert_eq!(
            finder.find(dir.path(), 1, "s.jsonl").found,
            vec![second],
            "a gone path walks again"
        );
    }

    #[test]
    fn finder_cache_starts_over_past_its_cap() {
        let dir = TempDir::new("finder-cap");
        let path = dir.write("a/s.jsonl", b"");
        let mut finder = Finder::new();
        for n in 0..FINDER_CAP + 10 {
            finder.find(dir.path(), 1, &format!("{n}.jsonl"));
        }
        assert!(finder.found.len() <= FINDER_CAP);
        assert_eq!(finder.find(dir.path(), 1, "s.jsonl").found, vec![path]);
    }

    #[test]
    fn names_lists_locked_files_without_disturbing_the_lock() {
        let dir = TempDir::new("names-lock");
        let locked = dir.write("thread-1.lock", b"lock");
        let exclusive = dir.write("thread-2.lock", b"lock");
        dir.write("thread-3.json", b"{}");
        fs::create_dir_all(dir.path().join("folder.lock")).unwrap();

        let holder = OpenOptions::new()
            .read(true)
            .write(true)
            .share_mode(FILE_SHARE_READ.0 | FILE_SHARE_WRITE.0)
            .open(&locked)
            .unwrap();
        lock(&holder).unwrap();
        let alone = OpenOptions::new()
            .read(true)
            .write(true)
            .share_mode(0)
            .open(&exclusive)
            .unwrap();

        let listed = names(dir.path(), 0, |n| n.ends_with(".lock")).unwrap();
        assert_eq!(listed, vec![locked.clone(), exclusive.clone()]);

        // The holder still has its lock and its exclusive open.
        let probe = OpenOptions::new()
            .read(true)
            .share_mode(SHARE_ALL)
            .open(&locked)
            .unwrap();
        let taken = lock(&probe).unwrap_err();
        assert_eq!(taken.code(), ERROR_LOCK_VIOLATION.to_hresult());
        let err = OpenOptions::new()
            .read(true)
            .share_mode(SHARE_ALL)
            .open(&exclusive)
            .unwrap_err();
        assert_eq!(os_code(&err), Some(ERROR_SHARING_VIOLATION.0));
        unlock(&holder).unwrap();
        drop(alone);
    }

    #[test]
    fn names_at_depth_one_lists_subfolder_files_only() {
        let dir = TempDir::new("names-1");
        dir.write("top.lock", b"");
        let a = dir.write("t1/a.lock", b"");
        let b = dir.write("t2/b.lock", b"");
        dir.write("t2/b.txt", b"");
        dir.write("t3/deeper/c.lock", b"");
        let listed = names(dir.path(), 1, |n| n.ends_with(".lock")).unwrap();
        assert_eq!(listed, vec![a, b]);
    }

    #[test]
    fn names_of_a_missing_folder_is_empty_and_a_file_is_an_error() {
        let dir = TempDir::new("names-missing");
        assert!(
            names(&dir.path().join("none"), 0, |_| true)
                .unwrap()
                .is_empty()
        );
        assert!(
            names(&dir.path().join("none"), 1, |_| true)
                .unwrap()
                .is_empty()
        );
        let file = dir.write("plain.txt", b"");
        assert!(names(&file, 0, |_| true).is_err());
    }
}
