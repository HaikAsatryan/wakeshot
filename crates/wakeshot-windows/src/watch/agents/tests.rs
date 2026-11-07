use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::windows::fs::OpenOptionsExt;
use std::sync::{Mutex, MutexGuard, PoisonError};

use wakeshot_core::process::FileTime;
use wakeshot_core::sources::{Activity, SessionKey};
use wakeshot_core::testing::{FakeClock, FakeSource};
use wakeshot_core::time::Clock;
use windows::Win32::Foundation::{CloseHandle, HANDLE, HLOCAL, LUID, LocalFree};
use windows::Win32::Security::Authorization::{
    ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
};
use windows::Win32::Security::{
    AdjustTokenPrivileges, DACL_SECURITY_INFORMATION, ImpersonateSelf, LUID_AND_ATTRIBUTES,
    LookupPrivilegeValueW, PSECURITY_DESCRIPTOR, RevertToSelf, SE_BACKUP_NAME,
    SecurityImpersonation, SetFileSecurityW, TOKEN_ADJUST_PRIVILEGES, TOKEN_PRIVILEGES,
    TOKEN_PRIVILEGES_ATTRIBUTES,
};
use windows::Win32::System::Threading::{GetCurrentThread, OpenThreadToken};
use windows::core::{HSTRING, PCWSTR, w};

use super::*;
use crate::log::MemoryLog;
use crate::process::FakeLiveness;
use crate::watch::read::TempDir;

const START: Now = Now {
    wall_ms: 1_793_750_400_000,
    mono_ms: 5_000,
};

type Hook = Box<dyn FnMut(&mut FakeSource, &FileId) + Send>;

struct Inner {
    fake: FakeSource,
    after_feed: Option<Hook>,
    asked: usize,
}

/// A `FakeSource` the test still reaches after `Agents` took it, with a hook that runs after each feed, the way a
/// parser's wants change once a status file names its transcript.
#[derive(Clone)]
struct Shared(Arc<Mutex<Inner>>);

impl Shared {
    fn new(id: SourceId) -> Self {
        Self(Arc::new(Mutex::new(Inner {
            fake: FakeSource::new(id),
            after_feed: None,
            asked: 0,
        })))
    }

    fn inner(&self) -> MutexGuard<'_, Inner> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn set(&self, f: impl FnOnce(&mut FakeSource)) {
        f(&mut self.inner().fake);
    }

    /// The feeds since the last call.
    fn fed(&self) -> Vec<(FileId, Feed)> {
        std::mem::take(&mut self.inner().fake.fed)
    }

    fn boxed(&self) -> Box<dyn AgentSource + Send> {
        Box::new(self.clone())
    }
}

impl AgentSource for Shared {
    fn id(&self) -> SourceId {
        self.inner().fake.id
    }

    fn wanted(&self) -> Vec<WantedFile> {
        let mut inner = self.inner();
        inner.asked += 1;
        inner.fake.wanted()
    }

    fn feed(&mut self, file: &FileId, data: Feed) -> Fed {
        let mut inner = self.inner();
        let Inner {
            fake, after_feed, ..
        } = &mut *inner;
        let fed = fake.feed(file, data);
        if let Some(hook) = after_feed {
            hook(fake, file);
        }
        fed
    }

    fn sessions(&self) -> Vec<SessionFacts> {
        self.inner().fake.sessions()
    }
}

fn agents(sources: &[&Shared]) -> (Agents, Arc<MemoryLog>) {
    let log = Arc::new(MemoryLog::default());
    let boxed = sources.iter().map(|s| s.boxed()).collect();
    (Agents::new(boxed, log.clone()), log)
}

fn lines(log: &MemoryLog) -> Vec<String> {
    log.lines
        .lock()
        .unwrap()
        .iter()
        .map(|(level, area, text)| {
            assert_eq!((*level, *area), (Level::Warn, AREA));
            text.clone()
        })
        .collect()
}

/// The Claude status file rule: `<digits>.json` and nothing else.
fn status_name(name: &str) -> bool {
    name.strip_suffix(".json")
        .is_some_and(|stem| !stem.is_empty() && stem.bytes().all(|b| b.is_ascii_digit()))
}

fn lock_name(name: &str) -> bool {
    name.ends_with(".lock")
}

fn folder(dir: &Path) -> WantedFile {
    WantedFile::Folder {
        dir: dir.to_path_buf(),
        accept: status_name,
    }
}

fn find_tail(root: &Path, suffix: &str) -> WantedFile {
    WantedFile::FindTail {
        root: root.to_path_buf(),
        depth: 1,
        name_suffix: suffix.to_owned(),
    }
}

fn whole(path: &Path, bytes: &[u8]) -> (FileId, Feed) {
    (FileId(path.to_path_buf()), Feed::Whole(bytes.to_vec()))
}

fn appended(path: &Path, bytes: &[u8]) -> (FileId, Feed) {
    (FileId(path.to_path_buf()), Feed::Appended(bytes.to_vec()))
}

fn gone(path: &Path) -> (FileId, Feed) {
    (FileId(path.to_path_buf()), Feed::Gone)
}

fn present(path: &Path) -> (FileId, Feed) {
    (FileId(path.to_path_buf()), Feed::Present)
}

fn process(pid: u32) -> ProcessRef {
    ProcessRef {
        pid,
        start: FileTime(134_381_211_391_959_336 + u64::from(pid)),
    }
}

fn session(pid: u32) -> SessionFacts {
    let p = process(pid);
    SessionFacts {
        key: SessionKey {
            source: SourceId::ClaudeCode,
            id: pid.to_string(),
        },
        liveness: sources::Liveness::Process {
            pid: p.pid,
            start: p.start,
        },
        activity: Activity::Working,
        timers: vec![],
        open_tasks: 0,
        label: "Claude Code",
        format: FormatHealth::Ok,
    }
}

fn append(path: &Path, bytes: &[u8]) {
    let mut file = OpenOptions::new().append(true).open(path).unwrap();
    file.write_all(bytes).unwrap();
}

#[test]
fn folders_lists_each_watched_folder_once_with_subtree_for_depth() {
    let root = TempDir::new("agents-folders");
    let [d1, d2, d3] = ["d1", "d2", "d3"].map(|d| root.path().join(d));
    let a = Shared::new(SourceId::ClaudeCode);
    a.set(|f| {
        f.wanted = vec![
            folder(&d1),
            WantedFile::Names {
                dir: d2.clone(),
                depth: 1,
                accept: lock_name,
            },
            WantedFile::Tail(root.path().join("t.jsonl")),
            find_tail(&root.path().join("projects"), "s.jsonl"),
        ];
    });
    let b = Shared::new(SourceId::Codex);
    b.set(|f| {
        f.wanted = vec![
            folder(&d1),
            WantedFile::Names {
                dir: d1.clone(),
                depth: 0,
                accept: lock_name,
            },
            WantedFile::Names {
                dir: d3.clone(),
                depth: 0,
                accept: lock_name,
            },
        ];
    });
    let (agents, _) = agents(&[&a, &b]);
    let watch = |dir: &Path, subtree| WatchFolder {
        dir: dir.to_path_buf(),
        subtree,
    };
    assert_eq!(
        agents.folders(),
        [watch(&d1, false), watch(&d2, true), watch(&d3, false)]
    );
}

#[test]
fn tick_feeds_each_want_kind_and_never_a_refused_name() {
    let root = TempDir::new("agents-kinds");
    let sessions = root.path().join("sessions");
    let status = root.write("sessions/20636.json", b"{\"pid\":20636}");
    root.write("sessions/20636.685bf0d758162e18.key", b"secret");
    root.write("sessions/notes.txt", b"other");
    let locks = root.path().join("locks");
    let lock = root.write("locks/t1/a.lock", b"");
    let tail = root.write("t.jsonl", b"{\"a\":1}\n{\"b\"");
    let found = root.write("projects/C--work/s1.jsonl", b"{\"s\":1}\n");
    let source = Shared::new(SourceId::ClaudeCode);
    source.set(|f| {
        f.wanted = vec![
            folder(&sessions),
            WantedFile::Names {
                dir: locks.clone(),
                depth: 1,
                accept: lock_name,
            },
            WantedFile::Tail(tail.clone()),
            find_tail(&root.path().join("projects"), "s1.jsonl"),
        ];
    });
    let (mut agents, _) = agents(&[&source]);

    assert!(agents.on_tick().is_empty());
    assert_eq!(
        source.fed(),
        [
            whole(&status, b"{\"pid\":20636}"),
            present(&lock),
            appended(&tail, b"{\"a\":1}\n"),
            appended(&found, b"{\"s\":1}\n"),
        ]
    );
}

#[test]
fn folder_feeds_a_file_again_only_when_it_changed_and_gone_when_deleted() {
    let root = TempDir::new("agents-folder");
    let one = root.write("1.json", b"{\"v\":1}");
    let two = root.write("2.json", b"{\"v\":2}");
    let source = Shared::new(SourceId::ClaudeCode);
    source.set(|f| f.wanted = vec![folder(root.path())]);
    let (mut agents, _) = agents(&[&source]);
    agents.on_tick();
    assert_eq!(
        source.fed(),
        [whole(&one, b"{\"v\":1}"), whole(&two, b"{\"v\":2}")]
    );

    agents.on_tick();
    assert!(source.fed().is_empty(), "unchanged files are not fed again");

    fs::write(&one, b"{\"v\":11}").unwrap();
    fs::remove_file(&two).unwrap();
    agents.on_tick();
    assert_eq!(source.fed(), [gone(&two), whole(&one, b"{\"v\":11}")]);

    fs::remove_file(&one).unwrap();
    agents.on_change(root.path(), Some(&[PathBuf::from("1.json")]));
    assert_eq!(source.fed(), [gone(&one)]);
    agents.on_change(root.path(), Some(&[PathBuf::from("1.json")]));
    assert!(source.fed().is_empty(), "a file is gone once");
}

#[test]
fn change_reads_only_the_named_files_and_runs_only_that_folders_sources() {
    let root = TempDir::new("agents-change");
    let da = root.path().join("a");
    let one = root.write("a/1.json", b"1");
    let two = root.write("a/2.json", b"2");
    let tail_a = root.write("a.jsonl", b"");
    let db = root.path().join("b");
    let tail_b = root.write("b.jsonl", b"");
    let a = Shared::new(SourceId::ClaudeCode);
    a.set(|f| f.wanted = vec![folder(&da), WantedFile::Tail(tail_a.clone())]);
    let b = Shared::new(SourceId::Codex);
    b.set(|f| f.wanted = vec![folder(&db), WantedFile::Tail(tail_b.clone())]);
    let (mut agents, _) = agents(&[&a, &b]);
    agents.on_tick();
    a.fed();

    fs::write(&one, b"11").unwrap();
    fs::write(&two, b"22").unwrap();
    append(&tail_a, b"x\n");
    append(&tail_b, b"y\n");
    let names = [
        PathBuf::from("1.json"),
        PathBuf::from("1.json"),
        PathBuf::from(r"sub\2.json"),
        PathBuf::from(r"..\a\2.json"),
    ];
    agents.on_change(&da, Some(&names));
    assert_eq!(
        a.fed(),
        [whole(&one, b"11"), appended(&tail_a, b"x\n")],
        "only the plain name is read, once; the source's tail is polled"
    );
    assert!(
        b.fed().is_empty(),
        "a source that does not watch the folder is not run"
    );

    agents.on_change(&da, None);
    assert_eq!(
        a.fed(),
        [whole(&two, b"22")],
        "no names: the folder is read in full"
    );
}

#[test]
fn retry_reads_again_once_and_feeds_even_unchanged_bytes() {
    let root = TempDir::new("agents-retry");
    let file = root.write("1.json", b"{\"half\":");
    let source = Shared::new(SourceId::ClaudeCode);
    source.set(|f| {
        f.wanted = vec![folder(root.path())];
        f.retry = vec![FileId(file.clone())];
    });
    let (mut agents, _) = agents(&[&source]);

    let retries = agents.on_tick();
    assert_eq!(
        retries.iter().map(Retry::file).collect::<Vec<_>>(),
        [FileId(file.clone())]
    );
    assert_eq!(source.fed(), [whole(&file, b"{\"half\":")]);

    assert!(agents.retry(&retries).is_empty());
    assert_eq!(source.fed(), [whole(&file, b"{\"half\":")]);
    agents.on_tick();
    assert!(source.fed().is_empty(), "taken bytes are not fed again");
}

#[test]
fn a_retry_answered_with_retry_once_again_is_not_retried_again() {
    let root = TempDir::new("agents-retry-twice");
    let file = root.write("1.json", b"{");
    let source = Shared::new(SourceId::ClaudeCode);
    let id = FileId(file.clone());
    source.set(|f| {
        f.wanted = vec![folder(root.path())];
        f.retry = vec![id.clone(), id.clone()];
    });
    let (mut agents, _) = agents(&[&source]);
    let retries = agents.on_tick();
    assert_eq!(retries.len(), 1);
    assert!(agents.retry(&retries).is_empty(), "retried once only");
    assert_eq!(source.fed().len(), 2);
    agents.on_tick();
    assert!(source.fed().is_empty(), "the source had its retry");

    // A retry that never came: the bytes were not taken, so the next read feeds them again.
    fs::write(&file, b"{{").unwrap();
    source.set(|f| f.retry = vec![id.clone()]);
    assert_eq!(agents.on_tick().len(), 1);
    assert_eq!(source.fed(), [whole(&file, b"{{")]);
    assert!(agents.on_tick().is_empty());
    assert_eq!(source.fed(), [whole(&file, b"{{")]);
    agents.on_tick();
    assert!(source.fed().is_empty());
}

#[test]
fn a_read_that_fails_is_retried_logged_and_never_fed_as_gone() {
    let root = TempDir::new("agents-unread");
    let file = root.write("1.json", b"{}");
    let source = Shared::new(SourceId::ClaudeCode);
    source.set(|f| f.wanted = vec![folder(root.path())]);
    let (mut agents, log) = agents(&[&source]);
    let live = FakeLiveness::default();

    let writer = OpenOptions::new()
        .write(true)
        .share_mode(0)
        .open(&file)
        .unwrap();
    let retries = agents.on_tick();
    assert_eq!(retries.len(), 1);
    assert!(source.fed().is_empty());
    agents.snapshots(&live, START);
    assert_eq!(
        lines(&log),
        ["Claude Code: a file it reads could not be read (error 32)"]
    );

    drop(writer);
    assert!(agents.retry(&retries).is_empty());
    assert_eq!(source.fed(), [whole(&file, b"{}")]);

    let _writer = OpenOptions::new()
        .write(true)
        .share_mode(0)
        .open(&file)
        .unwrap();
    agents.on_tick();
    assert!(
        source.fed().is_empty(),
        "a file that cannot be read is not gone"
    );
}

#[test]
fn names_feeds_a_file_appearing_and_going() {
    let root = TempDir::new("agents-names");
    let a = root.write("a.lock", b"");
    root.write("x.json", b"{}");
    let source = Shared::new(SourceId::Codex);
    source.set(|f| {
        f.wanted = vec![WantedFile::Names {
            dir: root.path().to_path_buf(),
            depth: 0,
            accept: lock_name,
        }];
    });
    let (mut agents, _) = agents(&[&source]);
    agents.on_tick();
    assert_eq!(source.fed(), [present(&a)]);
    agents.on_tick();
    assert!(
        source.fed().is_empty(),
        "a name still there is not fed again"
    );

    let b = root.write("b.lock", b"");
    fs::remove_file(&a).unwrap();
    agents.on_change(root.path(), Some(&[PathBuf::from("a.lock")]));
    assert_eq!(source.fed(), [gone(&a), present(&b)]);
}

/// Denies listing a folder to everyone, on the folder only, so the files in it stay readable. Dropped, the folder
/// lists again.
struct NoListing(PathBuf);

impl NoListing {
    fn on(dir: &Path) -> Self {
        set_dacl(dir, w!("D:(D;;0x1;;;WD)(A;OICI;FA;;;WD)")).unwrap();
        Self(dir.to_path_buf())
    }
}

impl Drop for NoListing {
    fn drop(&mut self) {
        // No assert: a panic here, during a failed test's unwind, would abort the run.
        let _ = set_dacl(&self.0, w!("D:(A;OICI;FA;;;WD)"));
    }
}

/// This test thread runs without `SeBackupPrivilege` until dropped. An elevated Git Bash (the MSYS2 runtime) starts
/// every child with that privilege enabled, and a folder opened for listing with backup semantics, as `read_dir`
/// opens it, then lists past a deny ACE; the tray never runs so. So the thread takes a copy of the process token
/// with the privilege off, and the deny holds in any shell (research, "A deny ACE and the backup privilege").
struct NoBackup;

impl NoBackup {
    fn on() -> Self {
        // SAFETY: gives this thread its own copy of the process token; `RevertToSelf` in the drop ends it.
        unsafe { ImpersonateSelf(SecurityImpersonation) }.unwrap();
        let guard = Self;
        let mut token = HANDLE::default();
        // SAFETY: the current-thread pseudo handle needs no close, and `token` is a live out pointer.
        unsafe {
            OpenThreadToken(
                GetCurrentThread(),
                TOKEN_ADJUST_PRIVILEGES,
                true,
                &mut token,
            )
        }
        .unwrap();
        let mut luid = LUID::default();
        // SAFETY: a null system name means this PC; the name is a static string and `luid` a live out pointer.
        unsafe { LookupPrivilegeValueW(PCWSTR::null(), SE_BACKUP_NAME, &mut luid) }.unwrap();
        let off = TOKEN_PRIVILEGES {
            PrivilegeCount: 1,
            Privileges: [LUID_AND_ATTRIBUTES {
                Luid: luid,
                Attributes: TOKEN_PRIVILEGES_ATTRIBUTES(0),
            }],
        };
        // SAFETY: the token is open with TOKEN_ADJUST_PRIVILEGES and `off` outlives the call; no previous state is
        // asked for. A token without the privilege (an unelevated shell) answers success too.
        let adjusted = unsafe { AdjustTokenPrivileges(token, false, Some(&off), 0, None, None) };
        // SAFETY: a handle the call above opened, closed once.
        unsafe { CloseHandle(token) }.unwrap();
        adjusted.unwrap();
        guard
    }
}

impl Drop for NoBackup {
    fn drop(&mut self) {
        // No assert: a panic here, during a failed test's unwind, would abort the run.
        // SAFETY: ends this thread's impersonation; nothing else on the thread relies on it.
        let _ = unsafe { RevertToSelf() };
    }
}

/// `SetFileSecurityW` changes the folder only: the files in it keep the access they had.
fn set_dacl(path: &Path, sddl: PCWSTR) -> windows::core::Result<()> {
    let mut sd = PSECURITY_DESCRIPTOR::default();
    // SAFETY: a constant string; `sd` is freed below with LocalFree, as the call documents.
    unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(sddl, SDDL_REVISION_1, &mut sd, None)
    }?;
    let name = HSTRING::from(path);
    // SAFETY: the name and the descriptor live through the call.
    let set = unsafe { SetFileSecurityW(&name, DACL_SECURITY_INFORMATION, sd) };
    // SAFETY: allocated by the conversion above, freed once.
    unsafe {
        let _ = LocalFree(Some(HLOCAL(sd.0)));
    }
    set.ok()
}

/// With its listing denied, a folder's known files are still read, so a status file that changes or goes reaches
/// its source; a new one waits until the folder lists again.
#[test]
fn a_folder_that_cannot_be_listed_reads_its_known_files() {
    let root = TempDir::new("agents-no-list");
    let dir = root.path().join("sessions");
    let one = root.write("sessions/1.json", b"{\"v\":1}");
    let two = root.write("sessions/2.json", b"{\"v\":2}");
    let source = Shared::new(SourceId::ClaudeCode);
    source.set(|f| f.wanted = vec![folder(&dir)]);
    let (mut agents, log) = agents(&[&source]);
    agents.on_tick();
    assert_eq!(source.fed().len(), 2);

    let _no_backup = NoBackup::on();
    let denied = NoListing::on(&dir);
    let listing = names(&dir, 0, |_| true).map_err(|e| e.raw_os_error());
    assert_eq!(listing, Err(Some(5)), "the listing is denied");
    assert_eq!(
        fs::read(&one).unwrap(),
        b"{\"v\":1}",
        "the files stay readable"
    );
    fs::write(&one, b"{\"v\":11}").unwrap();
    fs::remove_file(&two).unwrap();
    let three = root.write("sessions/3.json", b"{\"v\":3}");
    agents.on_tick();
    assert_eq!(source.fed(), [whole(&one, b"{\"v\":11}"), gone(&two)]);
    agents.snapshots(&FakeLiveness::default(), START);
    assert_eq!(
        lines(&log),
        ["Claude Code: a folder it reads could not be listed (error 5)"]
    );

    drop(denied);
    agents.on_tick();
    assert_eq!(source.fed(), [whole(&three, b"{\"v\":3}")]);
}

#[test]
fn a_folder_replaced_by_a_file_feeds_its_known_files_gone() {
    let root = TempDir::new("agents-unlisted");
    let dir = root.path().join("sessions");
    let file = root.write("sessions/1.json", b"{}");
    let source = Shared::new(SourceId::ClaudeCode);
    source.set(|f| f.wanted = vec![folder(&dir)]);
    let (mut agents, log) = agents(&[&source]);
    agents.on_tick();
    assert_eq!(source.fed(), [whole(&file, b"{}")]);

    // A file where the folder was: the listing fails, and the known file is not found.
    fs::remove_dir_all(&dir).unwrap();
    fs::write(&dir, b"").unwrap();
    agents.on_tick();
    assert_eq!(source.fed(), [gone(&file)]);
    agents.snapshots(&FakeLiveness::default(), START);
    let logged = lines(&log);
    assert_eq!(logged.len(), 1);
    assert!(
        logged[0].starts_with("Claude Code: a folder it reads could not be listed (error "),
        "{logged:?}"
    );

    fs::remove_file(&dir).unwrap();
    agents.on_tick();
    assert!(source.fed().is_empty(), "a file is gone once");
}

#[test]
fn tail_feeds_only_new_complete_lines_on_ticks_and_changes() {
    let root = TempDir::new("agents-tail");
    let dir = root.path().join("sessions");
    fs::create_dir_all(&dir).unwrap();
    let tail = root.write("t.jsonl", b"{\"a\":1}\n{\"b\":2}\n");
    let source = Shared::new(SourceId::ClaudeCode);
    source.set(|f| f.wanted = vec![folder(&dir), WantedFile::Tail(tail.clone())]);
    let (mut agents, _) = agents(&[&source]);
    agents.on_tick();
    assert_eq!(source.fed(), [appended(&tail, b"{\"a\":1}\n{\"b\":2}\n")]);

    append(&tail, b"{\"c\":3}\n{\"d\"");
    agents.on_tick();
    assert_eq!(source.fed(), [appended(&tail, b"{\"c\":3}\n")]);
    agents.on_tick();
    assert!(source.fed().is_empty());

    append(&tail, b":4}\n");
    agents.on_change(&dir, Some(&[]));
    assert_eq!(source.fed(), [appended(&tail, b"{\"d\":4}\n")]);
}

#[test]
fn tail_restart_feeds_gone_then_the_lines_from_the_start() {
    let root = TempDir::new("agents-restart");
    let tail = root.write("t.jsonl", b"{\"a\":1}\n");
    let source = Shared::new(SourceId::ClaudeCode);
    source.set(|f| f.wanted = vec![WantedFile::Tail(tail.clone())]);
    let (mut agents, _) = agents(&[&source]);
    agents.on_tick();
    assert_eq!(source.fed(), [appended(&tail, b"{\"a\":1}\n")]);

    let next = root.write("t.jsonl.tmp", b"{\"n\":1}\n{\"n\":2}\n");
    fs::rename(&next, &tail).unwrap();
    agents.on_tick();
    assert_eq!(
        source.fed(),
        [gone(&tail), appended(&tail, b"{\"n\":1}\n{\"n\":2}\n")]
    );

    // Gone, then back: fed as gone once, then from the start without a second `Gone`.
    fs::remove_file(&tail).unwrap();
    agents.on_tick();
    assert_eq!(source.fed(), [gone(&tail)]);
    agents.on_tick();
    assert!(source.fed().is_empty());
    fs::write(&tail, b"{\"z\":1}\n").unwrap();
    agents.on_tick();
    assert_eq!(source.fed(), [appended(&tail, b"{\"z\":1}\n")]);
}

#[test]
fn a_tail_that_leaves_the_wants_starts_from_the_start_when_it_comes_back() {
    let root = TempDir::new("agents-leave");
    let tail = root.write("t.jsonl", b"{\"a\":1}\n");
    let source = Shared::new(SourceId::ClaudeCode);
    source.set(|f| f.wanted = vec![WantedFile::Tail(tail.clone())]);
    let (mut agents, _) = agents(&[&source]);
    agents.on_tick();
    assert_eq!(source.fed().len(), 1);

    source.set(|f| f.wanted.clear());
    agents.on_tick();
    assert!(source.fed().is_empty());
    source.set(|f| f.wanted = vec![WantedFile::Tail(tail.clone())]);
    agents.on_tick();
    assert_eq!(source.fed(), [appended(&tail, b"{\"a\":1}\n")]);
}

#[test]
fn find_tail_keeps_its_file_and_feeds_gone_when_it_goes() {
    let root = TempDir::new("agents-find");
    let projects = root.path().join("projects");
    let first = root.write("projects/p/s.jsonl", b"{\"a\":1}\n");
    let source = Shared::new(SourceId::ClaudeCode);
    source.set(|f| f.wanted = vec![find_tail(&projects, "s.jsonl")]);
    let (mut agents, _) = agents(&[&source]);
    agents.on_tick();
    assert_eq!(source.fed(), [appended(&first, b"{\"a\":1}\n")]);

    // A second match does not take over while the first is still there.
    let second = root.write("projects/q/s.jsonl", b"{\"z\":1}\n");
    append(&first, b"{\"b\":2}\n");
    agents.on_tick();
    assert_eq!(source.fed(), [appended(&first, b"{\"b\":2}\n")]);

    fs::remove_file(&first).unwrap();
    agents.on_tick();
    assert_eq!(
        source.fed(),
        [gone(&first), appended(&second, b"{\"z\":1}\n")]
    );
}

#[test]
fn wanted_is_asked_again_after_each_feed() {
    let root = TempDir::new("agents-asked");
    let sessions = root.path().join("sessions");
    let status = root.write("sessions/1.json", b"{\"sessionId\":\"s1\"}");
    let projects = root.path().join("projects");
    let transcript = root.write("projects/p/s1.jsonl", b"{\"t\":1}\n");
    let source = Shared::new(SourceId::ClaudeCode);
    source.set(|f| f.wanted = vec![folder(&sessions)]);
    let hook_projects = projects.clone();
    source.inner().after_feed = Some(Box::new(move |fake, file| {
        if file.0.extension().is_some_and(|e| e == "json") {
            fake.wanted.push(find_tail(&hook_projects, "s1.jsonl"));
        }
    }));
    let (mut agents, _) = agents(&[&source]);
    let asked = source.inner().asked;

    agents.on_change(&sessions, Some(&[PathBuf::from("1.json")]));
    assert_eq!(
        source.fed(),
        [
            whole(&status, b"{\"sessionId\":\"s1\"}"),
            appended(&transcript, b"{\"t\":1}\n"),
        ],
        "the tail the status file named is polled in the same call"
    );
    // Once at the start of the call, then once after each of the two feeds.
    assert_eq!(source.inner().asked - asked, 3);
}

/// A resume: the crashed process's leftover status file is fed `Gone` before the new process's file names the same
/// transcript. The source dropped the transcript in between, so it must get it again from byte 0.
#[test]
fn a_tail_that_leaves_and_comes_back_within_one_call_starts_from_the_start() {
    let root = TempDir::new("agents-resume");
    let sessions = root.path().join("sessions");
    let old = root.write("sessions/100.json", b"{\"sessionId\":\"s1\"}");
    let projects = root.path().join("projects");
    let transcript = root.write("projects/p/s1.jsonl", b"{\"t\":1}\n");
    let source = Shared::new(SourceId::ClaudeCode);
    source.set(|f| f.wanted = vec![folder(&sessions)]);
    let hook_projects = projects.clone();
    source.inner().after_feed = Some(Box::new(move |fake, file| {
        let is_status = file.0.extension().is_some_and(|e| e == "json");
        let gone = matches!(fake.fed.last(), Some((_, Feed::Gone)));
        if is_status && gone {
            fake.wanted
                .retain(|w| !matches!(w, WantedFile::FindTail { .. }));
        } else if is_status && fake.wanted.len() == 1 {
            fake.wanted.push(find_tail(&hook_projects, "s1.jsonl"));
        }
    }));
    let (mut agents, _) = agents(&[&source]);
    agents.on_tick();
    assert_eq!(
        source.fed(),
        [
            whole(&old, b"{\"sessionId\":\"s1\"}"),
            appended(&transcript, b"{\"t\":1}\n"),
        ]
    );

    append(&transcript, b"{\"t\":2}\n");
    fs::remove_file(&old).unwrap();
    let new = root.write("sessions/200.json", b"{\"sessionId\":\"s1\"}");
    agents.on_change(&sessions, None);
    assert_eq!(
        source.fed(),
        [
            gone(&old),
            whole(&new, b"{\"sessionId\":\"s1\"}"),
            appended(&transcript, b"{\"t\":1}\n{\"t\":2}\n"),
        ]
    );
}

#[test]
fn process_liveness_is_yes_gone_or_unknown_and_live_ones_are_returned() {
    let source = Shared::new(SourceId::ClaudeCode);
    source.set(|f| f.sessions = vec![session(1), session(2), session(3)]);
    let (mut agents, log) = agents(&[&source]);
    let live = FakeLiveness::default();
    live.alive
        .lock()
        .unwrap()
        .extend([(process(1), Alive::Yes), (process(2), Alive::Unknown)]);

    let snapshots = agents.snapshots(&live, START);
    assert_eq!(
        snapshots,
        Snapshots {
            events: vec![Event::Sources {
                source: SourceId::ClaudeCode,
                sessions: vec![
                    (session(1), Alive::Yes),
                    (session(2), Alive::Unknown),
                    (session(3), Alive::Gone),
                ],
            }],
            live: vec![process(1)],
        }
    );
    assert_eq!(
        *live.checked.lock().unwrap(),
        [process(1), process(2), process(3)]
    );
    assert_eq!(
        lines(&log),
        ["Claude Code: a session's process could not be checked, so its liveness is unknown"]
    );
}

/// Lock file holders the test sets, one answer for every path, and every path asked about.
#[derive(Clone, Default)]
struct Owners(Arc<Mutex<OwnersInner>>);

#[derive(Default)]
struct OwnersInner {
    answer: Option<Result<Vec<ProcessRef>, crate::WinError>>,
    asked: Vec<PathBuf>,
}

impl Owners {
    fn inner(&self) -> MutexGuard<'_, OwnersInner> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn answer(&self, answer: Result<Vec<ProcessRef>, crate::WinError>) {
        self.inner().answer = Some(answer);
    }

    /// How many times it was asked since the last call.
    fn asked(&self) -> usize {
        std::mem::take(&mut self.inner().asked).len()
    }
}

impl LockOwners for Owners {
    fn holders(&self, path: &Path) -> Result<Vec<ProcessRef>, crate::WinError> {
        let mut inner = self.inner();
        inner.asked.push(path.to_path_buf());
        inner.answer.clone().unwrap_or(Ok(Vec::new()))
    }
}

const RM_FAILED: crate::WinError = crate::WinError {
    call: "RmGetList",
    code: 5,
};

fn codex_session(lock_file: &Path) -> SessionFacts {
    SessionFacts {
        key: SessionKey {
            source: SourceId::Codex,
            id: "0199aaaa-0000-7000-8000-000000000001".to_owned(),
        },
        liveness: sources::Liveness::AnyProcess {
            image: "codex.exe",
            lock_file: lock_file.to_path_buf(),
        },
        activity: Activity::Working,
        timers: vec![],
        open_tasks: 0,
        label: "Codex",
        format: FormatHealth::Ok,
    }
}

fn entry(process: ProcessRef, image: &str) -> ProcessEntry {
    ProcessEntry {
        pid: process.pid,
        parent: 0,
        start: process.start,
        image: image.to_owned(),
    }
}

/// A Copilot session as `CopilotSource` reports it: its liveness names the held in-use file.
fn copilot_session(lock_file: &Path) -> SessionFacts {
    SessionFacts {
        key: SessionKey {
            source: SourceId::Copilot,
            id: "0199c0de-0000-4000-8000-000000000001".to_owned(),
        },
        liveness: sources::Liveness::AnyProcess {
            image: "copilot.exe",
            lock_file: lock_file.to_path_buf(),
        },
        label: "Copilot",
        ..codex_session(lock_file)
    }
}

/// `Agents` over one Codex source whose one session names `lock_file`, with fake holders.
fn lock_agents(lock_file: &Path) -> (Agents, Arc<MemoryLog>, Shared, Owners) {
    lock_agents_for(codex_session(lock_file))
}

/// `Agents` over one source of `session`'s kind with that one session, with fake holders.
fn lock_agents_for(session: SessionFacts) -> (Agents, Arc<MemoryLog>, Shared, Owners) {
    let source = Shared::new(session.key.source);
    source.set(|f| f.sessions = vec![session]);
    let owners = Owners::default();
    let log = Arc::new(MemoryLog::default());
    let agents = Agents::with_owners(vec![source.boxed()], log.clone(), Box::new(owners.clone()));
    (agents, log, source, owners)
}

fn only_alive(snapshots: &Snapshots) -> Alive {
    match snapshots.events.as_slice() {
        [Event::Sources { sessions, .. }] => {
            assert_eq!(sessions.len(), 1);
            sessions[0].1
        }
        other => panic!("one event with one session: {other:?}"),
    }
}

#[test]
fn a_lock_file_session_lives_while_a_holder_running_its_image_lives() {
    let lock = PathBuf::from(r"C:\Users\x\.codex\thread-writer-locks\t1.lock");
    let (mut agents, log, source, owners) = lock_agents(&lock);
    let live = FakeLiveness::default();
    owners.answer(Ok(vec![process(7)]));
    live.processes
        .lock()
        .unwrap()
        .push(entry(process(7), "Codex.EXE"));
    live.alive.lock().unwrap().push((process(7), Alive::Yes));

    let first = agents.snapshots(&live, START);
    assert_eq!(
        first,
        Snapshots {
            events: vec![Event::Sources {
                source: SourceId::Codex,
                sessions: vec![(codex_session(&lock), Alive::Yes)],
            }],
            live: vec![process(7)],
        },
        "the holder is waited on like any session process"
    );
    assert_eq!(owners.asked(), 1);
    assert!(lines(&log).is_empty());

    // Known now: only its pid and start are checked.
    let again = agents.snapshots(&live, START);
    assert_eq!((again.events.len(), again.live), (0, vec![process(7)]));
    assert_eq!(owners.asked(), 0, "Restart Manager is not asked again");
    assert_eq!(*live.checked.lock().unwrap(), [process(7)]);

    // The holder ended: asked again; nobody holds it now.
    live.alive.lock().unwrap().clear();
    owners.answer(Ok(vec![]));
    let ended = agents.snapshots(&live, START);
    assert_eq!(only_alive(&ended), Alive::Gone);
    assert!(ended.live.is_empty());
    assert_eq!(owners.asked(), 1);

    // "Nobody" is kept too, ticks included, until the session's facts change (a log line came).
    agents.snapshots(&live, START);
    agents.on_tick();
    agents.snapshots(&live, START);
    assert_eq!(owners.asked(), 0, "a tick does not ask again");
    source.set(|f| f.sessions[0].activity = Activity::Idle);
    agents.snapshots(&live, START);
    assert_eq!(owners.asked(), 1);
    agents.snapshots(&live, START);
    assert_eq!(owners.asked(), 0);
}

/// A crash leaves lock files nobody holds for days (two Copilot sessions on the test laptop), and one Restart
/// Manager question costs 14 to 22 ms of CPU: a tick must not ask about them again.
#[test]
fn a_lock_file_nobody_holds_is_asked_about_once_however_many_ticks_come() {
    let dir = TempDir::new("agents-lock-dead");
    let lock = dir.write("inuse.8588.hold", b"");
    let (mut agents, _, source, owners) = lock_agents_for(copilot_session(&lock));
    source.set(|f| {
        f.wanted = vec![WantedFile::Names {
            dir: dir.path().to_path_buf(),
            depth: 0,
            accept: |name| name.ends_with(".hold"),
        }];
    });
    let live = FakeLiveness::default();
    agents.on_tick();
    assert_eq!(only_alive(&agents.snapshots(&live, START)), Alive::Gone);
    assert_eq!(owners.asked(), 1);

    for _ in 0..20 {
        agents.on_tick();
        agents.snapshots(&live, START);
    }
    assert_eq!(owners.asked(), 0);

    // A crashed session's file taken up again with no notice: its next log line changes the facts.
    owners.answer(Ok(vec![process(8588)]));
    live.processes
        .lock()
        .unwrap()
        .push(entry(process(8588), "copilot.exe"));
    live.alive.lock().unwrap().push((process(8588), Alive::Yes));
    source.set(|f| f.sessions[0].activity = Activity::Idle);
    agents.on_tick();
    assert_eq!(only_alive(&agents.snapshots(&live, START)), Alive::Yes);
    assert_eq!(owners.asked(), 1);
}

#[test]
fn a_holder_of_another_image_or_start_does_not_count() {
    let lock = PathBuf::from(r"C:\Users\x\.codex\thread-writer-locks\t1.lock");
    let (mut agents, _, _source, owners) = lock_agents(&lock);
    let live = FakeLiveness::default();
    let scanner = process(8);
    let reused = ProcessRef {
        pid: 9,
        start: FileTime(1),
    };
    owners.answer(Ok(vec![scanner, process(9), process(7)]));
    live.processes.lock().unwrap().extend([
        entry(scanner, "MsMpEng.exe"),
        entry(reused, "codex.exe"),
        entry(process(7), "codex.exe"),
    ]);
    live.alive.lock().unwrap().push((process(7), Alive::Yes));

    let snapshots = agents.snapshots(&live, START);
    assert_eq!(only_alive(&snapshots), Alive::Yes);
    assert_eq!(snapshots.live, [process(7)]);

    // Only the scanner holds it after the codex process ended: gone.
    live.alive.lock().unwrap().clear();
    owners.answer(Ok(vec![scanner]));
    let snapshots = agents.snapshots(&live, START);
    assert_eq!(only_alive(&snapshots), Alive::Gone);
    assert!(snapshots.live.is_empty());
}

/// The same short id as the core's lines (`core::holds`), FNV-1a 32 values from the published algorithm.
#[test]
fn short_ids_are_a_pid_as_it_is_or_8_hex_digits() {
    assert_eq!(short_id("20636"), "20636");
    assert_eq!(short_id("a"), "e40c292c");
    assert_eq!(short_id("foobar"), "bf9cf968");
    assert_eq!(short_id("12345678901"), "3605f02f", "too long for a pid");
    assert_eq!(short_id("ses_1"), "4a856782");
}

/// A working session that counts as ended says why, once: an agent whose exe got a new name would otherwise never
/// hold without a word.
#[test]
fn a_working_lock_file_session_that_counts_as_ended_says_why_once() {
    let lock = PathBuf::from(r"C:\Users\x\.codex\thread-writer-locks\t1.lock");
    let (mut agents, log, source, owners) = lock_agents(&lock);
    let live = FakeLiveness::default();
    let (scanner, renamed, gone) = (process(8), process(9), process(10));
    owners.answer(Ok(vec![renamed, scanner, gone]));
    live.processes.lock().unwrap().extend([
        entry(scanner, "MsMpEng.exe"),
        entry(renamed, "codex-next.exe"),
    ]);
    assert_eq!(only_alive(&agents.snapshots(&live, START)), Alive::Gone);
    for _ in 0..3 {
        agents.on_tick();
        agents.snapshots(&live, START);
    }
    assert_eq!(
        lines(&log),
        [
            "Codex CLI: session 1ad7a9dc shows work but counts as ended: no codex.exe holds its lock file, only \
                 MsMpEng.exe, codex-next.exe"
        ]
    );

    // Idle, its liveness decides nothing, so nothing more is said.
    log.lines.lock().unwrap().clear();
    source.set(|f| f.sessions[0].activity = Activity::Idle);
    assert_eq!(only_alive(&agents.snapshots(&live, START)), Alive::Gone);
    assert!(lines(&log).is_empty());
}

#[test]
fn a_working_session_whose_named_lock_nobody_holds_says_so() {
    let lock = PathBuf::from(r"C:\Users\x\.copilot\session-state\s1\inuse.8588.hold");
    let (mut agents, log, _source, _owners) = lock_agents_for(copilot_session(&lock));
    assert_eq!(
        only_alive(&agents.snapshots(&FakeLiveness::default(), START)),
        Alive::Gone
    );
    assert_eq!(
        lines(&log),
        [format!(
            "Copilot CLI: session {} shows work but counts as ended: neither copilot.exe nor the process its \
                 name gives holds its lock file",
            short_id("0199c0de-0000-4000-8000-000000000001")
        )]
    );
}

#[test]
fn a_change_notice_or_a_listing_of_the_lock_file_asks_again() {
    let dir = TempDir::new("agents-lock-owners");
    let lock = dir.write("t1.lock", b"");
    let (mut agents, _, source, owners) = lock_agents(&lock);
    source.set(|f| {
        f.wanted = vec![WantedFile::Names {
            dir: dir.path().to_path_buf(),
            depth: 0,
            accept: lock_name,
        }];
    });
    let live = FakeLiveness::default();
    owners.answer(Ok(vec![process(7)]));
    live.processes
        .lock()
        .unwrap()
        .push(entry(process(7), "codex.exe"));
    live.alive.lock().unwrap().push((process(7), Alive::Yes));
    agents.on_tick();
    assert_eq!(source.fed(), [present(&lock)]);
    agents.snapshots(&live, START);
    assert_eq!(owners.asked(), 1);

    // A tick that lists the same names keeps what is known; so does a notice about another file.
    agents.on_tick();
    agents.on_change(dir.path(), Some(&[PathBuf::from("t2.lock")]));
    agents.snapshots(&live, START);
    assert_eq!(owners.asked(), 0);

    // A notice that names it, or one that lost its names: the file may have been made again.
    agents.on_change(dir.path(), Some(&[PathBuf::from("t1.lock")]));
    agents.snapshots(&live, START);
    assert_eq!(owners.asked(), 1);
    agents.on_change(dir.path(), None);
    agents.snapshots(&live, START);
    assert_eq!(owners.asked(), 1);

    // A listing that feeds it gone (the fake source still lists the session).
    fs::remove_file(&lock).unwrap();
    agents.on_tick();
    assert_eq!(source.fed(), [gone(&lock)]);
    agents.snapshots(&live, START);
    assert_eq!(owners.asked(), 1);
}

#[test]
fn a_restart_manager_error_counts_any_live_process_running_the_image() {
    let lock = PathBuf::from(r"C:\Users\x\.codex\thread-writer-locks\t1.lock");
    let (mut agents, log, _source, owners) = lock_agents(&lock);
    let live = FakeLiveness::default();
    owners.answer(Err(RM_FAILED));
    live.processes
        .lock()
        .unwrap()
        .push(entry(process(9), "codex.exe"));

    let snapshots = agents.snapshots(&live, START);
    assert_eq!(only_alive(&snapshots), Alive::Yes);
    assert!(
        snapshots.live.is_empty(),
        "no one process is this session's"
    );
    assert_eq!(
        lines(&log),
        [
            "Codex CLI: Restart Manager could not tell which process holds a lock file (error 5), so any running \
                 codex.exe counts"
        ]
    );

    // Nothing is kept from a failed answer.
    live.processes
        .lock()
        .unwrap()
        .push(entry(process(10), "other.exe"));
    live.processes.lock().unwrap().remove(0);
    let snapshots = agents.snapshots(&live, START);
    assert_eq!(only_alive(&snapshots), Alive::Gone);
    assert_eq!(owners.asked(), 2);

    *live.list_error.lock().unwrap() = Some(RM_FAILED);
    let snapshots = agents.snapshots(&live, START);
    assert_eq!(only_alive(&snapshots), Alive::Unknown);
}

#[test]
fn a_failed_process_list_or_holder_check_is_unknown() {
    let lock = PathBuf::from(r"C:\Users\x\.codex\thread-writer-locks\t1.lock");
    let (mut agents, log, _source, owners) = lock_agents(&lock);
    let live = FakeLiveness::default();
    owners.answer(Ok(vec![process(7)]));
    *live.list_error.lock().unwrap() = Some(RM_FAILED);

    let snapshots = agents.snapshots(&live, START);
    assert_eq!(only_alive(&snapshots), Alive::Unknown);
    assert!(snapshots.live.is_empty());
    assert_eq!(
        lines(&log),
        ["Codex CLI: a session's process could not be checked, so its liveness is unknown"]
    );
    agents.snapshots(&live, START);
    assert_eq!(owners.asked(), 2, "an unknown answer is not kept");

    // Known, then its check fails: unknown, and the holder stays known.
    *live.list_error.lock().unwrap() = None;
    live.processes
        .lock()
        .unwrap()
        .push(entry(process(7), "codex.exe"));
    live.alive.lock().unwrap().push((process(7), Alive::Yes));
    assert_eq!(only_alive(&agents.snapshots(&live, START)), Alive::Yes);
    live.alive.lock().unwrap()[0].1 = Alive::Unknown;
    let snapshots = agents.snapshots(&live, START);
    assert_eq!(only_alive(&snapshots), Alive::Unknown);
    assert!(snapshots.live.is_empty());
    live.alive.lock().unwrap()[0].1 = Alive::Yes;
    assert_eq!(agents.snapshots(&live, START).events.len(), 1);
    assert_eq!(owners.asked(), 1, "asked once in the three calls");
}

// Copilot's in-use files (seen live 2026-11-06): the process that has a session open holds
// `inuse.<its pid>.hold`; for VS Code's Copilot harness that is `copilot-runtime.exe`, not `copilot.exe`.

#[test]
fn a_holder_whose_pid_the_lock_name_gives_counts_whatever_its_image() {
    let lock = PathBuf::from(r"C:\Users\x\.copilot\session-state\s1\inuse.2728.hold");
    let (mut agents, log, source, owners) = lock_agents_for(copilot_session(&lock));
    let live = FakeLiveness::default();
    let runtime = process(2728);
    let scanner = process(8);
    owners.answer(Ok(vec![scanner, runtime]));
    live.processes.lock().unwrap().extend([
        entry(scanner, "MsMpEng.exe"),
        entry(runtime, "copilot-runtime.exe"),
    ]);
    live.alive.lock().unwrap().push((runtime, Alive::Yes));

    let snapshots = agents.snapshots(&live, START);
    assert_eq!(only_alive(&snapshots), Alive::Yes);
    assert_eq!(
        snapshots.live,
        [runtime],
        "the scanner is not this session's"
    );
    assert!(lines(&log).is_empty());

    // The runtime ended and only the scanner holds it: gone.
    live.alive.lock().unwrap().clear();
    owners.answer(Ok(vec![scanner]));
    let snapshots = agents.snapshots(&live, START);
    assert_eq!(only_alive(&snapshots), Alive::Gone);
    assert!(snapshots.live.is_empty());

    // A pid the name gives that started at another time is another process: still gone, until the list agrees.
    // Each step changes the session's facts, as a log line does, so Restart Manager is asked again.
    source.set(|f| f.sessions[0].activity = Activity::Idle);
    owners.answer(Ok(vec![runtime]));
    live.processes.lock().unwrap()[1] = entry(
        ProcessRef {
            pid: 2728,
            start: FileTime(1),
        },
        "copilot-runtime.exe",
    );
    let snapshots = agents.snapshots(&live, START);
    assert_eq!(only_alive(&snapshots), Alive::Gone);
    assert!(snapshots.live.is_empty());
    source.set(|f| f.sessions[0].activity = Activity::Working);
    live.processes.lock().unwrap()[1] = entry(runtime, "copilot-runtime.exe");
    assert_eq!(only_alive(&agents.snapshots(&live, START)), Alive::Yes);
}

#[test]
fn a_restart_manager_error_counts_only_the_process_the_lock_name_gives() {
    let lock = PathBuf::from(r"C:\Users\x\.copilot\session-state\s1\inuse.52300.hold");
    let (mut agents, log, _source, owners) = lock_agents_for(copilot_session(&lock));
    let live = FakeLiveness::default();
    owners.answer(Err(RM_FAILED));
    // VS Code's `copilot` shim and another session's CLI run; the process the name gives does not.
    live.processes.lock().unwrap().extend([
        entry(process(47868), "copilot.exe"),
        entry(process(9), "copilot.exe"),
    ]);

    let snapshots = agents.snapshots(&live, START);
    assert_eq!(only_alive(&snapshots), Alive::Gone);
    assert!(snapshots.live.is_empty());
    assert_eq!(
        lines(&log),
        [
            "Copilot CLI: Restart Manager could not tell which process holds a lock file (error 5), so the \
                 process its name gives counts"
        ]
    );

    // It runs, whatever its image: waited on like a holder.
    live.processes
        .lock()
        .unwrap()
        .push(entry(process(52300), "copilot-runtime.exe"));
    let snapshots = agents.snapshots(&live, START);
    assert_eq!(only_alive(&snapshots), Alive::Yes);
    assert_eq!(snapshots.live, [process(52300)]);
    assert_eq!(owners.asked(), 2, "nothing is kept from a failed answer");

    *live.list_error.lock().unwrap() = Some(RM_FAILED);
    let snapshots = agents.snapshots(&live, START);
    assert_eq!(only_alive(&snapshots), Alive::Unknown);
}

#[test]
fn a_lock_name_gives_a_pid_only_as_a_whole_number_part() {
    for (name, pid) in [
        ("inuse.52300.hold", Some(52300)),
        ("INUSE.2728.LOCK", Some(2728)),
        ("inuse.9.x.hold", Some(9)),
        ("0199aaaa-0000-7000-8000-000000000001.lock", None),
        ("t1.lock", None),
        ("inuse..hold", None),
        ("inuse.0.hold", None),
        ("inuse.-1.hold", None),
        ("inuse.4294967296.hold", None),
        ("inuse.12345678901.hold", None),
    ] {
        let path = Path::new(r"C:\Users\x\.copilot\session-state\s1").join(name);
        assert_eq!(named_pid(&path), pid, "{name}");
    }
}

/// The Copilot source on a made-up Copilot home, read through the real listing, tail and Restart Manager, with the
/// in-use files as Copilot 1.0.106 makes them: this process keeps `inuse.<its pid>.hold` open and has written
/// `inuse.<its pid>.lock`. Its image is not `copilot.exe`, so only the pid in the name makes it the holder.
#[test]
fn copilot_from_a_held_in_use_file_and_its_events() {
    use wakeshot_core::sources::copilot::CopilotSource;
    use wakeshot_core::sources::cron::FixedOffset;

    const ID: &str = "0199c0de-0000-4000-8000-000000000001";
    let home = TempDir::new("agents-copilot");
    let me = crate::process::current().unwrap();
    let hold = home.write(&format!(r"session-state\{ID}\inuse.{}.hold", me.pid), b"");
    home.write(
        &format!(r"session-state\{ID}\inuse.{}.lock", me.pid),
        me.pid.to_string().as_bytes(),
    );
    let held = fs::File::open(&hold).unwrap();
    home.write(
        &format!(r"session-state\{ID}\events.jsonl"),
        concat!(
            r#"{"id":"a","timestamp":"2026-11-06T10:00:00.000Z","parentId":null,"type":"user.message","data":{"content":""}}"#,
            "\n"
        )
        .as_bytes(),
    );
    let log = Arc::new(MemoryLog::default());
    let source = CopilotSource::new(home.path().to_path_buf(), Box::new(FixedOffset(240)));
    let mut agents = Agents::new(vec![Box::new(source)], log.clone());
    let live = FakeLiveness::default();
    live.processes
        .lock()
        .unwrap()
        .push(entry(me, "copilot-runtime.exe"));
    live.alive.lock().unwrap().push((me, Alive::Yes));

    agents.on_tick();
    // The tail joins once its session is listed, so a second round reads the log.
    agents.on_tick();
    assert_eq!(
        agents.snapshots(&live, START),
        Snapshots {
            events: vec![Event::Sources {
                source: SourceId::Copilot,
                sessions: vec![(
                    SessionFacts {
                        activity: Activity::Working,
                        ..copilot_session(&hold)
                    },
                    Alive::Yes
                )],
            }],
            live: vec![me],
        }
    );

    // A clean exit closes the `.hold` and deletes both files.
    drop(held);
    for entry in fs::read_dir(hold.parent().unwrap()).unwrap() {
        let path = entry.unwrap().path();
        if path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("inuse.")
        {
            fs::remove_file(path).unwrap();
        }
    }
    agents.on_tick();
    assert_eq!(
        agents.snapshots(&live, START).events,
        [Event::Sources {
            source: SourceId::Copilot,
            sessions: vec![],
        }]
    );
    assert!(lines(&log).is_empty());
}

/// The Codex source on a made-up Codex home, read through the real listing, finder, tail and Restart Manager; this
/// process holds the lock file the way Codex does and passes for `codex.exe` in the fake process list.
#[test]
fn codex_from_a_held_lock_file_and_its_rollout() {
    use wakeshot_core::sources::codex::CodexSource;

    const ID: &str = "0199aaaa-0000-7000-8000-000000000001";
    let home = TempDir::new("agents-codex");
    let lock = home.write(&format!(r"thread-writer-locks\{ID}.lock"), b"");
    home.write(r"thread-writer-locks\.coordination.lock", b"");
    let held = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&lock)
        .unwrap();
    held.try_lock().unwrap();
    let start = concat!(
        r#"{"timestamp":"2026-11-06T10:00:00.000Z","ordinal":0,"type":"session_meta","payload":{"id":"0199aaaa-0000-7000-8000-000000000001"}}"#,
        "\n",
        r#"{"timestamp":"2026-11-06T10:00:05.000Z","ordinal":1,"type":"event_msg","payload":{"type":"task_started","turn_id":"t1"}}"#,
        "\n",
    );
    let rollout = home.write(
        &format!(r"sessions\2026\11\06\rollout-2026-11-06T12-00-00-{ID}.jsonl"),
        start.as_bytes(),
    );
    let log = Arc::new(MemoryLog::default());
    let mut agents = Agents::new(
        vec![Box::new(CodexSource::new(home.path().to_path_buf()))],
        log.clone(),
    );
    let me = crate::process::current().unwrap();
    let live = FakeLiveness::default();
    live.processes.lock().unwrap().push(entry(me, "codex.exe"));
    live.alive.lock().unwrap().push((me, Alive::Yes));
    let locks_dir = home.path().join("thread-writer-locks");
    assert_eq!(
        agents.folders(),
        [WatchFolder {
            dir: locks_dir.clone(),
            subtree: false,
        }]
    );

    let facts = |activity| SessionFacts {
        activity,
        ..codex_session(&lock)
    };
    agents.on_tick();
    let first = agents.snapshots(&live, START);
    assert_eq!(
        first,
        Snapshots {
            events: vec![Event::Sources {
                source: SourceId::Codex,
                sessions: vec![(facts(Activity::Working), Alive::Yes)],
            }],
            live: vec![me],
        }
    );

    append(
        &rollout,
        concat!(
            r#"{"timestamp":"2026-11-06T10:01:00.000Z","ordinal":2,"type":"event_msg","payload":{"type":"task_complete","turn_id":"t1"}}"#,
            "\n"
        )
        .as_bytes(),
    );
    agents.on_tick();
    assert_eq!(
        agents.snapshots(&live, START).events,
        [Event::Sources {
            source: SourceId::Codex,
            sessions: vec![(facts(Activity::Idle), Alive::Yes)],
        }]
    );

    // A revert in the TUI: the thread goes on in a new file, read from its start, and the old one is left.
    set_written(&rollout, Duration::from_secs(60));
    home.write(
        &format!(
            r"sessions\2026\11\06\rollout-2026-11-06T12-30-00-{ID}_0199aaaa-0000-7000-8000-0000000000ff.jsonl"
        ),
        start.as_bytes(),
    );
    agents.on_tick();
    assert_eq!(
        agents.snapshots(&live, START).events,
        [Event::Sources {
            source: SourceId::Codex,
            sessions: vec![(facts(Activity::Working), Alive::Yes)],
        }]
    );

    // Listing, finding and asking left the lock as Codex took it: a second taker is still refused.
    let second = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&lock)
        .unwrap();
    assert!(matches!(
        second.try_lock(),
        Err(std::fs::TryLockError::WouldBlock)
    ));
    drop(second);

    // Codex closes the lock file, then deletes it.
    drop(held);
    fs::remove_file(&lock).unwrap();
    agents.on_change(&locks_dir, Some(&[PathBuf::from(format!("{ID}.lock"))]));
    assert_eq!(
        agents.snapshots(&live, START).events,
        [Event::Sources {
            source: SourceId::Codex,
            sessions: vec![],
        }]
    );
    assert!(lines(&log).is_empty(), "{:?}", lines(&log));
}

/// Seen live: Codex appends to its open rollout and each append gives a change notice in milliseconds, but with
/// only the lock folder watched a turn's start and end waited for the 30 s tick. The source's `sessions`
/// listing gets the tree watched, so a notice there reads the rollout at once.
#[test]
fn a_codex_rollout_write_is_read_on_its_notice_without_a_tick() {
    use wakeshot_core::sources::codex::CodexSource;

    const ID: &str = "0199aaaa-0000-7000-8000-000000000001";
    let home = TempDir::new("agents-codex-notice");
    let lock = home.write(&format!(r"thread-writer-locks\{ID}.lock"), b"");
    let held = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&lock)
        .unwrap();
    held.try_lock().unwrap();
    let name = format!("rollout-2026-11-06T12-00-00-{ID}.jsonl");
    let rollout = home.write(
        &format!(r"sessions\2026\11\06\{name}"),
        concat!(
            r#"{"timestamp":"2026-11-06T10:00:00.000Z","ordinal":0,"type":"session_meta","payload":{}}"#,
            "\n"
        )
        .as_bytes(),
    );
    let log = Arc::new(MemoryLog::default());
    let mut agents = Agents::new(
        vec![Box::new(CodexSource::new(home.path().to_path_buf()))],
        log.clone(),
    );
    let me = crate::process::current().unwrap();
    let live = FakeLiveness::default();
    live.processes.lock().unwrap().push(entry(me, "codex.exe"));
    live.alive.lock().unwrap().push((me, Alive::Yes));
    let posted = |activity| {
        vec![Event::Sources {
            source: SourceId::Codex,
            sessions: vec![(
                SessionFacts {
                    activity,
                    ..codex_session(&lock)
                },
                Alive::Yes,
            )],
        }]
    };

    agents.on_tick();
    let sessions_dir = home.path().join("sessions");
    assert_eq!(
        agents.folders(),
        [
            WatchFolder {
                dir: sessions_dir.clone(),
                subtree: true,
            },
            WatchFolder {
                dir: home.path().join("thread-writer-locks"),
                subtree: false,
            },
        ]
    );
    assert_eq!(
        agents.snapshots(&live, START).events,
        posted(Activity::Idle)
    );

    append(
        &rollout,
        concat!(
            r#"{"timestamp":"2026-11-06T10:00:05.000Z","ordinal":1,"type":"event_msg","payload":{"type":"task_started","turn_id":"t1"}}"#,
            "\n"
        )
        .as_bytes(),
    );
    let changed = [PathBuf::from(format!(r"2026\11\06\{name}"))];
    assert!(agents.on_change(&sessions_dir, Some(&changed)).is_empty());
    assert_eq!(
        agents.snapshots(&live, START).events,
        posted(Activity::Working)
    );

    // A notice that lost its names reads the same.
    append(
        &rollout,
        concat!(
            r#"{"timestamp":"2026-11-06T10:01:00.000Z","ordinal":2,"type":"event_msg","payload":{"type":"task_complete","turn_id":"t1"}}"#,
            "\n"
        )
        .as_bytes(),
    );
    agents.on_change(&sessions_dir, None);
    assert_eq!(
        agents.snapshots(&live, START).events,
        posted(Activity::Idle)
    );
    assert!(lines(&log).is_empty(), "{:?}", lines(&log));

    // No thread: the tree is no longer watched.
    drop(held);
    fs::remove_file(&lock).unwrap();
    agents.on_tick();
    assert_eq!(
        agents.folders(),
        [WatchFolder {
            dir: home.path().join("thread-writer-locks"),
            subtree: false,
        }]
    );
}

#[test]
fn no_event_when_nothing_changed() {
    let a = Shared::new(SourceId::ClaudeCode);
    a.set(|f| f.sessions = vec![session(1)]);
    let b = Shared::new(SourceId::Codex);
    let (mut agents, _) = agents(&[&a, &b]);
    let live = FakeLiveness::default();
    live.alive.lock().unwrap().push((process(1), Alive::Yes));

    let first = agents.snapshots(&live, START);
    assert_eq!(
        first.events,
        [
            Event::Sources {
                source: SourceId::ClaudeCode,
                sessions: vec![(session(1), Alive::Yes)],
            },
            Event::Sources {
                source: SourceId::Codex,
                sessions: vec![],
            },
        ],
        "every source on the first call, even one without sessions"
    );
    let again = agents.snapshots(&live, START);
    assert_eq!(again.events, []);
    assert_eq!(again.live, [process(1)], "live processes come every time");

    // The process ended: only that source's snapshot changed.
    live.alive.lock().unwrap().clear();
    let ended = agents.snapshots(&live, START);
    assert_eq!(
        ended.events,
        [Event::Sources {
            source: SourceId::ClaudeCode,
            sessions: vec![(session(1), Alive::Gone)],
        }]
    );
    assert!(ended.live.is_empty());

    agents.forget_posted();
    assert_eq!(agents.snapshots(&live, START).events.len(), 2);
}

#[test]
fn format_problems_log_at_most_once_a_minute_per_source() {
    let clock = FakeClock::new(START);
    let claude = Shared::new(SourceId::ClaudeCode);
    let mut unreadable = session(1);
    unreadable.format = FormatHealth::Unreadable("status file");
    let mut unknown = session(2);
    unknown.format = FormatHealth::UnknownValue("status", "thinking".into());
    claude.set(|f| f.sessions = vec![unreadable.clone()]);
    let codex = Shared::new(SourceId::Codex);
    let (mut agents, log) = agents(&[&claude, &codex]);
    let live = FakeLiveness::default();
    let take = |log: &MemoryLog| std::mem::take(&mut *log.lines.lock().unwrap()).len();

    agents.snapshots(&live, clock.now());
    assert_eq!(lines(&log), ["Claude Code: format problem in status file"]);
    take(&log);

    // A new problem 10 s later waits for the minute; another source logs at once.
    clock.run(10_000);
    claude.set(|f| f.sessions.push(unknown.clone()));
    let mut codex_unknown = session(3);
    codex_unknown.format = FormatHealth::UnknownValue("type", "x".into());
    codex.set(|f| f.sessions = vec![codex_unknown]);
    agents.snapshots(&live, clock.now());
    assert_eq!(lines(&log), ["Codex CLI: unknown value in type"]);
    take(&log);
    clock.run(49_000);
    agents.snapshots(&live, clock.now());
    assert_eq!(take(&log), 0, "59 s after the last line");

    clock.run(1_000);
    agents.snapshots(&live, clock.now());
    assert_eq!(lines(&log), ["Claude Code: unknown value in status"]);
    take(&log);

    // Still there an hour later: already logged.
    clock.run(3_600_000);
    agents.snapshots(&live, clock.now());
    assert_eq!(take(&log), 0);

    // Gone, then back: logged again.
    claude.set(|f| f.sessions = vec![session(1)]);
    agents.snapshots(&live, clock.now());
    claude.set(|f| f.sessions = vec![unreadable]);
    clock.run(60_000);
    agents.snapshots(&live, clock.now());
    assert_eq!(lines(&log), ["Claude Code: format problem in status file"]);
    assert!(
        !log.lines
            .lock()
            .unwrap()
            .iter()
            .any(|(_, _, text)| text.contains("thinking")),
        "a file's value never reaches the log"
    );
}

// `FindNewest`, `Recent`, `Query` and `ImageSince`.

const DAY: Duration = Duration::from_secs(24 * 60 * 60);

/// Sets a file's last write time `ago` back from now.
fn set_written(path: &Path, ago: Duration) {
    let file = OpenOptions::new().write(true).open(path).unwrap();
    file.set_modified(SystemTime::now() - ago).unwrap();
}

/// The made-up rule of the `FindNewest` tests: `<key>.jsonl` or `<key>_<tag>.jsonl`.
fn of_key(name: &str, key: &str) -> bool {
    name.strip_prefix(key)
        .is_some_and(|rest| rest == ".jsonl" || (rest.starts_with('_') && rest.ends_with(".jsonl")))
}

fn newest(root: &Path, key: &str) -> WantedFile {
    WantedFile::FindNewest {
        root: root.to_path_buf(),
        depth: 3,
        key: key.to_owned(),
        matches: of_key,
    }
}

fn chat_name(name: &str) -> bool {
    name.ends_with(".jsonl")
}

fn recent(root: &Path, depth: u8, sub: Option<&'static str>) -> WantedFile {
    WantedFile::Recent {
        root: root.to_path_buf(),
        depth,
        sub,
        accept: chat_name,
    }
}

#[test]
fn find_newest_follows_the_newest_match_and_feeds_the_old_one_gone_first() {
    let root = TempDir::new("agents-newest");
    let old = root.write(r"2026\11\05\t1.jsonl", b"a\n");
    set_written(&old, Duration::from_secs(3600));
    // Another key's file, the newest of all, is never read.
    root.write(r"2026\11\06\t2.jsonl", b"x\n");
    let source = Shared::new(SourceId::Codex);
    source.set(|f| f.wanted = vec![newest(root.path(), "t1")]);
    let (mut agents, _) = agents(&[&source]);
    assert_eq!(agents.folders(), [], "a tail is not watched");
    agents.on_tick();
    assert_eq!(source.fed(), [appended(&old, b"a\n")]);

    // A newer match: the old file goes, then the new one comes from its start.
    let new = root.write(r"2026\11\06\t1_r2.jsonl", b"b\nc\n");
    agents.on_tick();
    assert_eq!(source.fed(), [gone(&old), appended(&new, b"b\nc\n")]);
    append(&new, b"d\n");
    agents.on_tick();
    assert_eq!(source.fed(), [appended(&new, b"d\n")]);

    // A match made later but written earlier is not the newest.
    let early = root.write(r"2026\11\04\t1_r0.jsonl", b"e\n");
    set_written(&early, Duration::from_secs(7200));
    agents.on_tick();
    assert!(source.fed().is_empty());

    // The newest one went: back to the next newest, from its start.
    fs::remove_file(&new).unwrap();
    agents.on_tick();
    assert_eq!(source.fed(), [gone(&new), appended(&old, b"a\n")]);
}

/// A transcript or rollout in a folder that cannot be listed is never found: said, instead of a session that
/// silently never reads its file.
#[test]
fn a_find_whose_folder_cannot_be_listed_says_so_while_nothing_is_found() {
    let root = TempDir::new("agents-find-no-list");
    let transcript = root.write(r"projects\C--work\s1.jsonl", b"{\"s\":1}\n");
    let rollout = root.write(r"sessions\2026\11\06\t1.jsonl", b"a\n");
    let claude = Shared::new(SourceId::ClaudeCode);
    claude.set(|f| f.wanted = vec![find_tail(&root.path().join("projects"), "s1.jsonl")]);
    let codex = Shared::new(SourceId::Codex);
    codex.set(|f| f.wanted = vec![newest(&root.path().join("sessions"), "t1")]);
    let (mut agents, log) = agents(&[&claude, &codex]);

    let no_backup = NoBackup::on();
    let denied = [
        NoListing::on(&root.path().join(r"projects\C--work")),
        NoListing::on(&root.path().join(r"sessions\2026\11\06")),
    ];
    agents.on_tick();
    assert!(claude.fed().is_empty() && codex.fed().is_empty());
    agents.snapshots(&FakeLiveness::default(), START);
    assert_eq!(
        lines(&log),
        [
            "Claude Code: a folder it reads could not be listed (error 5)",
            "Codex CLI: a folder it reads could not be listed (error 5)"
        ]
    );

    drop(denied);
    drop(no_backup);
    agents.on_tick();
    assert_eq!(claude.fed(), [appended(&transcript, b"{\"s\":1}\n")]);
    assert_eq!(codex.fed(), [appended(&rollout, b"a\n")]);
}

#[test]
fn folders_watch_a_recent_root_and_a_query_database_folder() {
    let root = TempDir::new("agents-new-folders");
    let [ws, global, data] = ["ws", "global", "data"].map(|d| root.path().join(d));
    let source = Shared::new(SourceId::VsCode);
    source.set(|f| {
        f.wanted = vec![
            recent(&ws, 1, Some("chatSessions")),
            recent(&global.join("emptyWindowChatSessions"), 0, None),
            WantedFile::Query {
                db: data.join("opencode.db"),
                name: "tables",
                sql: "SELECT 1",
            },
            newest(root.path(), "t1"),
        ];
    });
    let (agents, _) = agents(&[&source]);
    let watch = |dir: &Path, subtree| WatchFolder {
        dir: dir.to_path_buf(),
        subtree,
    };
    assert_eq!(
        agents.folders(),
        [
            watch(&data, false),
            watch(&global.join("emptyWindowChatSessions"), false),
            watch(&ws, true),
        ]
    );
}

#[test]
fn recent_reads_only_recent_files_at_start_and_never_opens_an_old_one() {
    let root = TempDir::new("agents-recent-start");
    let old = root.write(r"ws1\chatSessions\old.jsonl", b"{\"old\":1}\n");
    set_written(&old, 3 * DAY);
    let new = root.write(r"ws1\chatSessions\new.jsonl", b"{\"n\":1}\n");
    let b = root.write(r"ws2\chatSessions\b.jsonl", b"{\"b\":1}\n");
    root.write(r"ws1\other\x.jsonl", b"{}\n");
    root.write(r"ws1\chatSessions\c.json", b"{}\n");
    root.write(r"ws1\chatSessions\deep\y.jsonl", b"{}\n");
    root.write("top.jsonl", b"{}\n");
    // Held with no sharing: any open of it fails, and would show as a retry and a logged read problem.
    let held = OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&old)
        .unwrap();
    let source = Shared::new(SourceId::VsCode);
    source.set(|f| f.wanted = vec![recent(root.path(), 1, Some("chatSessions"))]);
    let (mut agents, log) = agents(&[&source]);

    assert!(agents.on_tick().is_empty(), "nothing to read again");
    assert_eq!(
        source.fed(),
        [whole(&new, b"{\"n\":1}\n"), whole(&b, b"{\"b\":1}\n")]
    );
    agents.snapshots(&FakeLiveness::default(), START);
    assert!(lines(&log).is_empty(), "{:?}", lines(&log));
    drop(held);
    agents.on_tick();
    assert!(
        source.fed().is_empty(),
        "an unchanged file and an old one are not read"
    );
}

#[test]
fn recent_reads_a_changed_file_whole_again_even_rewritten_in_place_and_feeds_gone() {
    let root = TempDir::new("agents-recent-change");
    let file = root.write(r"ws\chatSessions\s.jsonl", b"{\"kind\":0}\n");
    let old = root.write(r"ws\chatSessions\old.jsonl", b"{\"kind\":0}\n");
    set_written(&old, 3 * DAY);
    let source = Shared::new(SourceId::VsCode);
    source.set(|f| f.wanted = vec![recent(root.path(), 1, Some("chatSessions"))]);
    let (mut agents, _) = agents(&[&source]);
    agents.on_tick();
    assert_eq!(source.fed(), [whole(&file, b"{\"kind\":0}\n")]);

    // A notice: the file is read whole, not as a tail; names that do not fit the pattern are not read.
    append(&file, b"{\"kind\":1}\n");
    let names = [
        PathBuf::from(r"ws\chatSessions\s.jsonl"),
        PathBuf::from(r"ws\CHATSESSIONS\s.jsonl"),
        PathBuf::from(r"ws\other\s.jsonl"),
        PathBuf::from(r"ws\chatSessions\s.json"),
        PathBuf::from(r"ws\chatSessions\deep\s.jsonl"),
        PathBuf::from(r"..\ws\chatSessions\s.jsonl"),
        PathBuf::from(r"ws\chatSessions"),
    ];
    agents.on_change(root.path(), Some(&names));
    assert_eq!(
        source.fed(),
        [whole(&file, b"{\"kind\":0}\n{\"kind\":1}\n")]
    );

    // Compaction rewrites the file in place, shorter: read whole again.
    fs::write(&file, b"{\"kind\":0,\"c\":1}\n").unwrap();
    agents.on_tick();
    assert_eq!(source.fed(), [whole(&file, b"{\"kind\":0,\"c\":1}\n")]);
    agents.on_tick();
    assert!(source.fed().is_empty(), "unchanged");

    // An old file that changes is new work.
    append(&old, b"{\"kind\":1}\n");
    let old_name = [PathBuf::from(r"ws\chatSessions\old.jsonl")];
    agents.on_change(root.path(), Some(&old_name));
    assert_eq!(source.fed(), [whole(&old, b"{\"kind\":0}\n{\"kind\":1}\n")]);

    fs::remove_file(&file).unwrap();
    agents.on_tick();
    assert_eq!(source.fed(), [gone(&file)]);
    fs::remove_file(&old).unwrap();
    agents.on_change(root.path(), Some(&old_name));
    assert_eq!(source.fed(), [gone(&old)]);
    agents.on_tick();
    assert!(source.fed().is_empty(), "a file is gone once");
}

#[test]
fn recent_reads_a_file_again_once_when_the_source_asks() {
    let root = TempDir::new("agents-recent-retry");
    let bytes = b"{\"kind\":0}\n{\"kind\":1,";
    let file = root.write(r"emptyWindowChatSessions\s.jsonl", bytes);
    let source = Shared::new(SourceId::VsCode);
    source.set(|f| {
        f.wanted = vec![recent(root.path(), 0, Some("emptyWindowChatSessions"))];
        f.retry = vec![FileId(file.clone())];
    });
    let (mut agents, _) = agents(&[&source]);
    let retries = agents.on_tick();
    assert_eq!(
        retries.iter().map(Retry::file).collect::<Vec<_>>(),
        [FileId(file.clone())]
    );
    assert_eq!(source.fed(), [whole(&file, bytes)]);

    assert!(agents.retry(&retries).is_empty());
    assert_eq!(
        source.fed(),
        [whole(&file, bytes)],
        "fed again though unchanged"
    );
    agents.on_tick();
    assert!(source.fed().is_empty(), "once");
}

#[test]
fn recent_reads_files_written_since_the_start_even_days_later() {
    let root = TempDir::new("agents-recent-window");
    let since_start = root.write(r"a\s1.jsonl", b"1\n");
    set_written(&since_start, DAY + DAY / 2);
    let before = root.write(r"a\s2.jsonl", b"2\n");
    set_written(&before, 3 * DAY);
    let real = SystemTime::now();
    let wall = Arc::new(Mutex::new(real - 2 * DAY));
    let clock = Arc::clone(&wall);
    let io = AgentIo {
        wall: Box::new(move || *clock.lock().unwrap()),
        ..AgentIo::default()
    };
    let source = Shared::new(SourceId::VsCode);
    source.set(|f| f.wanted = vec![recent(root.path(), 1, None)]);
    // Started two days ago, read now.
    let mut agents = Agents::with_io(vec![source.boxed()], Arc::new(MemoryLog::default()), io);
    *wall.lock().unwrap() = real;
    agents.on_tick();
    assert_eq!(source.fed(), [whole(&since_start, b"1\n")]);
}

struct FakeDb {
    answer: sqlite::Answer,
    calls: Vec<(PathBuf, String)>,
}

/// One source that wants one query on `<dir>\opencode.db`, a fake SQLite, a fake run time and fake holders: the
/// database is held by an agent ([`AGENT`]) unless a test says otherwise.
struct QueryRig {
    dir: TempDir,
    db: PathBuf,
    source: Shared,
    agents: Agents,
    fake: Arc<Mutex<FakeDb>>,
    clock: Arc<Mutex<Instant>>,
    owners: Owners,
    log: Arc<MemoryLog>,
}

/// The agent process that holds the rig's database open.
const AGENT: u32 = 4242;

const ROWS: &[u8] = br#"{"rows":[["ses_1",1793750400000]]}"#;

fn query_want(db: &Path, name: &'static str, sql: &'static str) -> WantedFile {
    WantedFile::Query {
        db: db.to_path_buf(),
        name,
        sql,
    }
}

impl QueryRig {
    fn new(tag: &str) -> Self {
        let dir = TempDir::new(tag);
        let db = dir.path().join("opencode.db");
        let source = Shared::new(SourceId::OpenCode);
        let want = query_want(&db, "tables", "SELECT name FROM sqlite_master");
        source.set(|f| f.wanted = vec![want]);
        let fake = Arc::new(Mutex::new(FakeDb {
            answer: sqlite::Answer::Json(ROWS.to_vec()),
            calls: Vec::new(),
        }));
        let clock = Arc::new(Mutex::new(Instant::now()));
        let (db_fake, now) = (Arc::clone(&fake), Arc::clone(&clock));
        let owners = Owners::default();
        owners.answer(Ok(vec![process(AGENT)]));
        let io = AgentIo {
            owners: Box::new(owners.clone()),
            query: Box::new(move |db, sql| {
                let mut fake = db_fake.lock().unwrap();
                fake.calls.push((db.to_path_buf(), sql.to_owned()));
                fake.answer.clone()
            }),
            now: Box::new(move || *now.lock().unwrap()),
            ..AgentIo::default()
        };
        let log = Arc::new(MemoryLog::default());
        let agents = Agents::with_io(vec![source.boxed()], log.clone(), io);
        Self {
            dir,
            db,
            source,
            agents,
            fake,
            clock,
            owners,
            log,
        }
    }

    fn now(&self) -> Instant {
        *self.clock.lock().unwrap()
    }

    /// A `-shm` whose WAL-index header is `mark`, as after the agent's commit number `mark`; the read marks and
    /// lock bytes after it are `rest`.
    fn shm(&self, mark: u8, rest: u8) {
        let mut bytes = vec![mark; sqlite::HEADER_LEN];
        bytes.resize(32_768, rest);
        self.dir.write("opencode.db-shm", &bytes);
    }

    fn open(&self) {
        self.dir.write("opencode.db", b"");
        self.dir.write("opencode.db-wal", b"");
    }

    fn runs(&self) -> usize {
        std::mem::take(&mut self.fake.lock().unwrap().calls).len()
    }

    fn later(&self, ms: u64) {
        *self.clock.lock().unwrap() += Duration::from_millis(ms);
    }

    fn answer(&self, answer: sqlite::Answer) {
        self.fake.lock().unwrap().answer = answer;
    }

    fn id(&self) -> PathBuf {
        FileId::query(&self.db, "tables").0
    }

    fn change(&mut self, names: &[&str]) -> Vec<Retry> {
        let names: Vec<PathBuf> = names.iter().map(PathBuf::from).collect();
        let dir = self.dir.path().to_path_buf();
        self.agents.on_change(&dir, Some(&names))
    }
}

#[test]
fn a_query_runs_only_while_the_wal_exists_and_its_end_is_fed_gone_once() {
    let mut rig = QueryRig::new("agents-query-wal");
    rig.dir.write("opencode.db", b"");
    rig.agents.on_tick();
    assert_eq!(rig.runs(), 0, "no -wal: the agent has not opened it");
    assert!(rig.source.fed().is_empty(), "nothing was answered yet");

    rig.dir.write("opencode.db-wal", b"");
    rig.later(1_000);
    rig.agents.on_tick();
    assert_eq!(rig.runs(), 1);
    assert_eq!(
        rig.source.fed(),
        [(FileId(rig.id()), Feed::Whole(ROWS.to_vec()))]
    );

    fs::remove_file(rig.dir.path().join("opencode.db-wal")).unwrap();
    rig.later(1_000);
    rig.agents.on_tick();
    assert_eq!(rig.runs(), 0);
    assert_eq!(rig.source.fed(), [(FileId(rig.id()), Feed::Gone)]);
    rig.later(1_000);
    rig.agents.on_tick();
    assert!(rig.source.fed().is_empty(), "gone once");

    rig.open();
    fs::remove_file(&rig.db).unwrap();
    rig.later(1_000);
    rig.agents.on_tick();
    assert_eq!(rig.runs(), 0, "no database, only its -wal");

    rig.open();
    rig.later(1_000);
    rig.agents.on_tick();
    assert_eq!(rig.runs(), 1);
    assert_eq!(
        rig.source.fed(),
        [(FileId(rig.id()), Feed::Whole(ROWS.to_vec()))],
        "back: the answer is fed again"
    );
}

/// Windows without a usable SQLite, and a database an agent has open without its `-wal`, give no signal and no
/// notice, so each is said once while it lasts.
#[test]
fn a_query_error_and_a_held_database_without_its_wal_are_logged() {
    let mut rig = QueryRig::new("agents-query-log");
    let live = FakeLiveness::default();
    live.alive
        .lock()
        .unwrap()
        .push((process(AGENT), Alive::Yes));
    let minute = |n: u64| Now {
        mono_ms: START.mono_ms + n * LOG_EVERY_MS,
        ..START
    };
    rig.open();
    rig.answer(sqlite::Answer::Json(br#"{"error":"no_sqlite"}"#.to_vec()));
    rig.agents.on_tick();
    rig.agents.snapshots(&live, minute(0));
    rig.agents.snapshots(&live, minute(1));
    assert_eq!(
        lines(&rig.log),
        [
            "OpenCode: Windows' own SQLite (winsqlite3.dll) is missing, older than 3.22.0 or not usable, so its \
                 database is not read"
        ]
    );

    // Rows again: over, and nothing more is said.
    rig.log.lines.lock().unwrap().clear();
    rig.answer(sqlite::Answer::Json(ROWS.to_vec()));
    rig.later(1_000);
    rig.change(&["opencode.db-wal"]);
    assert_eq!(rig.runs(), 2);
    rig.agents.snapshots(&live, minute(2));
    assert!(lines(&rig.log).is_empty());

    // The agent still has it open, but the `-wal` went: it is not read.
    fs::remove_file(rig.dir.path().join("opencode.db-wal")).unwrap();
    rig.later(1_000);
    rig.agents.on_tick();
    rig.agents.snapshots(&live, minute(3));
    assert_eq!(
        lines(&rig.log),
        ["OpenCode: its database is open but has no -wal file, so it is not read"]
    );
}

/// A closed database has no `-wal`: that is the normal state of an agent that is not running.
#[test]
fn a_database_nobody_holds_without_its_wal_says_nothing() {
    let mut rig = QueryRig::new("agents-query-closed");
    rig.owners.answer(Ok(vec![]));
    rig.dir.write("opencode.db", b"");
    rig.agents.on_tick();
    rig.agents.snapshots(&FakeLiveness::default(), START);
    assert_eq!(rig.runs(), 0);
    assert!(lines(&rig.log).is_empty());
}

#[test]
fn a_change_of_the_database_files_runs_a_query_at_most_once_a_second() {
    let mut rig = QueryRig::new("agents-query-limit");
    let poll = Duration::from_millis(QUERY_POLL_MS);
    rig.open();
    rig.agents.on_tick();
    assert_eq!(rig.runs(), 1);
    assert_eq!(rig.agents.next_due(), Some(rig.now() + poll), "the poll");

    // Other files in the folder do not run it.
    rig.later(2_000);
    rig.change(&["auth.json", r"log\x.log", "opencode.dbx.tmp\\a"]);
    assert_eq!(rig.runs(), 0);

    let at = *rig.clock.lock().unwrap();
    rig.change(&["opencode.db-wal"]);
    assert_eq!(rig.runs(), 1);
    rig.later(300);
    rig.change(&["OPENCODE.DB-WAL"]);
    rig.later(300);
    rig.change(&["opencode.db"]);
    assert_eq!(rig.runs(), 0, "held back by the limit");
    assert_eq!(
        rig.agents.next_due(),
        Some(at + Duration::from_millis(QUERY_EVERY_MS))
    );
    assert!(rig.agents.run_due().is_empty());
    assert_eq!(rig.runs(), 0, "not due yet");

    rig.later(400);
    rig.agents.run_due();
    assert_eq!(rig.runs(), 1, "the held-back run, once");
    assert_eq!(
        rig.agents.next_due(),
        Some(rig.now() + poll),
        "the poll again"
    );
    rig.agents.run_due();
    assert_eq!(rig.runs(), 0);

    // A notice that lost its names counts as a change of every file.
    rig.later(1_000);
    let dir = rig.dir.path().to_path_buf();
    rig.agents.on_change(&dir, None);
    assert_eq!(rig.runs(), 1);
}

/// A crashed agent leaves its `-wal` and `-shm`, and each read then rewrites the `-shm`, so only the database and
/// its `-wal` run a query.
#[test]
fn only_a_notice_of_the_database_or_its_wal_runs_a_query() {
    let mut rig = QueryRig::new("agents-query-shm");
    rig.open();
    rig.agents.on_tick();
    assert_eq!(rig.runs(), 1);
    let poll = rig.agents.next_due();
    for name in [
        "opencode.db-shm",
        "OPENCODE.DB-SHM",
        "opencode.db-journal",
        "opencode.db-wal2",
        "opencode.dbx",
        r"sub\opencode.db-wal",
    ] {
        rig.later(2_000);
        rig.change(&[name]);
        assert_eq!(rig.runs(), 0, "{name}");
        assert_eq!(rig.agents.next_due(), poll, "{name}: only the poll");
    }
    for name in ["opencode.db-wal", "OpenCode.DB", "OPENCODE.DB-WAL"] {
        rig.later(2_000);
        rig.change(&[name]);
        assert_eq!(rig.runs(), 1, "{name}");
    }
}

#[test]
fn a_busy_database_gives_no_feed_and_an_unchanged_answer_is_not_fed_again() {
    let mut rig = QueryRig::new("agents-query-busy");
    rig.open();
    rig.agents.on_tick();
    assert_eq!(rig.source.fed().len(), 1);

    rig.shm(1, 0);
    rig.later(1_000);
    rig.agents.on_tick();
    assert_eq!(rig.runs(), 2);
    assert!(rig.source.fed().is_empty(), "the same answer");

    rig.answer(sqlite::Answer::Busy);
    rig.shm(2, 0);
    rig.later(1_000);
    rig.agents.on_tick();
    assert_eq!(rig.runs(), 1);
    assert!(rig.source.fed().is_empty(), "busy: the last answer stands");

    // No commit since, but the busy read took nothing, so it reads again.
    let error = br#"{"error":"open"}"#.to_vec();
    rig.answer(sqlite::Answer::Json(error.clone()));
    rig.later(1_000);
    rig.agents.on_tick();
    assert_eq!(rig.source.fed(), [(FileId(rig.id()), Feed::Whole(error))]);
}

#[test]
fn a_query_a_feed_asks_for_runs_in_the_same_call() {
    let mut rig = QueryRig::new("agents-query-new");
    rig.open();
    let db = rig.db.clone();
    rig.source.inner().after_feed = Some(Box::new(move |fake, file| {
        if file.0.ends_with("opencode.db?tables") && fake.wanted.len() == 1 {
            fake.wanted
                .push(query_want(&db, "v2", "SELECT id FROM session_v2"));
        }
    }));
    rig.agents.on_tick();
    let sqls: Vec<String> = rig
        .fake
        .lock()
        .unwrap()
        .calls
        .iter()
        .map(|c| c.1.clone())
        .collect();
    assert_eq!(
        sqls,
        [
            "SELECT name FROM sqlite_master",
            "SELECT id FROM session_v2"
        ]
    );
    let fed: Vec<FileId> = rig.source.fed().into_iter().map(|(f, _)| f).collect();
    assert_eq!(fed, [FileId(rig.id()), FileId::query(&rig.db, "v2")]);
}

/// A commit of OpenCode's gives no change notice (seen live), so a query is due again two seconds after
/// each run while its database is there, and reads when the `-shm`'s header changed.
#[test]
fn a_query_is_polled_while_its_database_is_there() {
    let mut rig = QueryRig::new("agents-query-poll");
    let poll = Duration::from_millis(QUERY_POLL_MS);
    rig.agents.on_tick();
    assert_eq!(rig.agents.next_due(), None, "no database: no poll");

    rig.open();
    rig.shm(1, 0);
    rig.later(QUERY_EVERY_MS);
    rig.change(&["opencode.db"]);
    assert_eq!(rig.runs(), 1, "its making is a notice");
    let first = rig.now();
    assert_eq!(rig.agents.next_due(), Some(first + poll));

    // A commit: the header changes, and no notice comes.
    rig.answer(sqlite::Answer::Json(br#"{"rows":[]}"#.to_vec()));
    rig.shm(2, 0);
    rig.later(QUERY_POLL_MS - 1);
    rig.agents.run_due();
    assert_eq!(rig.runs(), 0, "not due yet");
    rig.later(1);
    rig.agents.run_due();
    assert_eq!(rig.runs(), 1);
    assert_eq!(
        rig.source.fed(),
        [
            (FileId(rig.id()), Feed::Whole(ROWS.to_vec())),
            (FileId(rig.id()), Feed::Whole(br#"{"rows":[]}"#.to_vec()))
        ],
        "the commit is read with no notice and no tick"
    );
    assert_eq!(rig.agents.next_due(), Some(first + 2 * poll));

    fs::remove_file(&rig.db).unwrap();
    rig.later(QUERY_POLL_MS);
    rig.agents.run_due();
    assert_eq!(rig.agents.next_due(), None, "the database went");
}

/// Each poll wakes the thread for about 1 ms of CPU, and only a process that holds the database
/// open can commit, so a database nobody holds is not polled. Wakeshot's own reads do not count.
#[test]
fn a_query_is_not_polled_while_nobody_holds_its_database() {
    let mut rig = QueryRig::new("agents-query-nobody");
    rig.owners.answer(Ok(vec![]));
    rig.open();
    rig.shm(1, 0);
    rig.agents.on_tick();
    assert_eq!((rig.runs(), rig.owners.asked()), (1, 1), "the first read");
    assert_eq!(rig.agents.next_due(), None);

    // Ticks still look at the header, and ask nobody.
    for _ in 0..3 {
        rig.later(30_000);
        rig.agents.on_tick();
    }
    assert_eq!((rig.runs(), rig.owners.asked()), (0, 0));
    assert_eq!(rig.agents.next_due(), None);

    // Only Wakeshot holds it, as during its own read.
    let mut me = QueryRig::new("agents-query-me");
    me.owners
        .answer(Ok(vec![crate::process::current().unwrap()]));
    me.open();
    me.agents.on_tick();
    assert_eq!(me.agents.next_due(), None);
}

#[test]
fn a_shm_notice_while_nobody_held_the_database_starts_the_poll_without_a_read() {
    let mut rig = QueryRig::new("agents-query-arm");
    let poll = Duration::from_millis(QUERY_POLL_MS);
    rig.owners.answer(Ok(vec![]));
    rig.open();
    rig.shm(1, 0);
    rig.agents.on_tick();
    assert_eq!((rig.runs(), rig.owners.asked()), (1, 1));
    assert_eq!(rig.agents.next_due(), None);

    // OpenCode starts: its first open rebuilds the `-shm` it finds, the same header again.
    rig.owners.answer(Ok(vec![process(AGENT)]));
    rig.later(2_000);
    rig.change(&["opencode.db-shm"]);
    assert_eq!(rig.runs(), 0, "a -shm notice never reads by itself");
    assert_eq!(rig.owners.asked(), 1);
    assert_eq!(rig.agents.next_due(), Some(rig.now() + poll));

    // Its commit gives no notice: the poll reads it.
    rig.shm(2, 0);
    rig.later(QUERY_POLL_MS);
    rig.agents.run_due();
    assert_eq!(rig.runs(), 1);

    // While it holds the database, notices ask nothing.
    rig.later(2_000);
    rig.change(&["opencode.db-shm", "opencode.db-wal"]);
    assert_eq!((rig.runs(), rig.owners.asked()), (1, 0));
}

#[test]
fn the_poll_stops_after_every_holder_of_its_database_ended() {
    let mut rig = QueryRig::new("agents-query-hosts-end");
    let live = FakeLiveness::default();
    let agent = process(AGENT);
    live.alive.lock().unwrap().push((agent, Alive::Yes));
    rig.open();
    rig.agents.on_tick();
    assert_eq!(rig.owners.asked(), 1);
    assert!(rig.agents.next_due().is_some());
    assert_eq!(
        rig.agents.snapshots(&live, START).live,
        [agent],
        "its exit is waited on"
    );
    rig.agents.snapshots(&live, START);
    assert_eq!(rig.owners.asked(), 0, "a live holder is only checked");

    // It ended: the poll, still due, asks again and finds nobody.
    live.alive.lock().unwrap().clear();
    rig.owners.answer(Ok(vec![]));
    assert!(rig.agents.snapshots(&live, START).live.is_empty());
    rig.later(QUERY_POLL_MS);
    rig.agents.run_due();
    assert_eq!(rig.owners.asked(), 1);
    assert_eq!(rig.agents.next_due(), None);
}

/// A start that gave no notice is still read at the next tick, and then polled.
#[test]
fn a_commit_a_tick_finds_while_nobody_held_the_database_asks_again() {
    let mut rig = QueryRig::new("agents-query-tick-commit");
    rig.owners.answer(Ok(vec![]));
    rig.open();
    rig.shm(1, 0);
    rig.agents.on_tick();
    assert_eq!((rig.runs(), rig.owners.asked()), (1, 1));
    assert_eq!(rig.agents.next_due(), None);

    rig.owners.answer(Ok(vec![process(AGENT)]));
    rig.shm(2, 0);
    rig.later(30_000);
    rig.agents.on_tick();
    assert_eq!((rig.runs(), rig.owners.asked()), (1, 1));
    assert_eq!(
        rig.agents.next_due(),
        Some(rig.now() + Duration::from_millis(QUERY_POLL_MS))
    );
}

/// A process that held the database only for a moment (a virus scan when the agent exits, another reader such as
/// a second Wakeshot) must not keep the poll on for as long as it lives (seen live).
#[test]
fn a_database_still_held_is_asked_about_again_after_the_recheck_time() {
    let mut rig = QueryRig::new("agents-query-recheck");
    rig.open();
    rig.agents.on_tick();
    assert_eq!(rig.owners.asked(), 1);
    let polls = HOSTS_RECHECK_MS / QUERY_POLL_MS;
    for _ in 1..polls {
        rig.later(QUERY_POLL_MS);
        rig.agents.run_due();
    }
    assert_eq!(
        rig.owners.asked(),
        0,
        "a holder is not asked about at each poll"
    );
    assert!(rig.agents.next_due().is_some());

    // The scan ended long ago, the scanner lives on.
    rig.owners.answer(Ok(vec![]));
    rig.later(QUERY_POLL_MS);
    rig.agents.run_due();
    assert_eq!(rig.owners.asked(), 1);
    assert_eq!(rig.agents.next_due(), None);
}

/// The queries on one database share one question per call: one costs about 20 ms of CPU.
#[test]
fn the_queries_on_one_database_ask_once_per_call() {
    let mut rig = QueryRig::new("agents-query-ask-once");
    let db = rig.db.clone();
    rig.source.set(|f| {
        f.wanted
            .push(query_want(&db, "v2", "SELECT id FROM session_v2"));
        f.wanted
            .push(query_want(&db, "s", "SELECT id FROM schedule"));
    });
    rig.owners.answer(Ok(vec![]));
    rig.open();
    rig.shm(1, 0);
    rig.agents.on_tick();
    assert_eq!((rig.runs(), rig.owners.asked()), (3, 1), "the first reads");

    // A commit a tick finds: three reads, one question.
    rig.shm(2, 0);
    rig.later(30_000);
    rig.agents.on_tick();
    assert_eq!((rig.runs(), rig.owners.asked()), (3, 1));
}

#[test]
fn a_failed_question_polls_as_before_and_asks_again_only_on_the_tick() {
    let mut rig = QueryRig::new("agents-query-rm-failed");
    rig.owners.answer(Err(RM_FAILED));
    rig.open();
    rig.agents.on_tick();
    assert_eq!(rig.owners.asked(), 1);
    for _ in 0..3 {
        rig.later(QUERY_POLL_MS);
        rig.agents.run_due();
        assert!(rig.agents.next_due().is_some(), "polled");
    }
    assert_eq!(rig.owners.asked(), 0);
    rig.later(QUERY_POLL_MS);
    rig.agents.on_tick();
    assert_eq!(rig.owners.asked(), 1);
}

/// OpenCode leaves its `-wal` and `-shm` after it stops (seen live), so a look that finds the header as
/// it was after the last read does not read: same header, same rows. Readers write past it (read marks, locks).
#[test]
fn a_query_reads_again_only_when_the_wal_index_header_changed() {
    let mut rig = QueryRig::new("agents-query-header");
    rig.open();
    rig.shm(1, 0);
    rig.agents.on_tick();
    assert_eq!(rig.runs(), 1, "the first read");

    for (tick, rest) in [(false, 0), (true, 0), (false, 9), (true, 7)] {
        rig.shm(1, rest);
        rig.later(QUERY_POLL_MS);
        if tick {
            rig.agents.on_tick();
        } else {
            rig.agents.run_due();
        }
        assert_eq!(rig.runs(), 0, "tick {tick}, bytes after the header {rest}");
    }

    rig.shm(2, 0);
    rig.later(QUERY_POLL_MS);
    rig.agents.run_due();
    assert_eq!(rig.runs(), 1, "a commit");

    // A notice reads whatever the header says.
    rig.later(QUERY_POLL_MS);
    rig.change(&["opencode.db-wal"]);
    assert_eq!(rig.runs(), 1);

    // The `-shm` went: that is a change too, once.
    fs::remove_file(rig.dir.path().join("opencode.db-shm")).unwrap();
    rig.later(QUERY_POLL_MS);
    rig.agents.run_due();
    assert_eq!(rig.runs(), 1);
    rig.later(QUERY_POLL_MS);
    rig.agents.run_due();
    assert_eq!(rig.runs(), 0, "and then no more");
}

/// The header is taken after the read: on files a stopped agent left, the read itself rebuilds it.
#[test]
fn a_read_that_rewrites_the_header_is_no_change_for_the_next_look() {
    let rig = QueryRig::new("agents-query-rebuild");
    rig.open();
    rig.shm(1, 0);
    let shm = rig.dir.path().join("opencode.db-shm");
    let db_fake = Arc::clone(&rig.fake);
    let now = Arc::clone(&rig.clock);
    let io = AgentIo {
        query: Box::new(move |db, sql| {
            let mut bytes = vec![5u8; sqlite::HEADER_LEN];
            bytes.resize(32_768, 0);
            fs::write(&shm, bytes).unwrap();
            let mut fake = db_fake.lock().unwrap();
            fake.calls.push((db.to_path_buf(), sql.to_owned()));
            fake.answer.clone()
        }),
        now: Box::new(move || *now.lock().unwrap()),
        ..AgentIo::default()
    };
    let mut agents = Agents::with_io(vec![rig.source.boxed()], Arc::new(MemoryLog::default()), io);
    agents.on_tick();
    assert_eq!(rig.runs(), 1);
    rig.later(QUERY_POLL_MS);
    agents.on_tick();
    assert_eq!(rig.runs(), 0, "the header the read wrote is the one kept");
}

const OPENCODE: &[&str] = &["opencode.exe", "opencode-cli.exe"];

fn opencode_session(since_wall_ms: Option<i64>) -> SessionFacts {
    SessionFacts {
        key: SessionKey {
            source: SourceId::OpenCode,
            id: "ses_1".to_owned(),
        },
        liveness: sources::Liveness::ImageSince {
            images: OPENCODE,
            since_wall_ms,
        },
        activity: Activity::Working,
        timers: vec![],
        open_tasks: 0,
        label: "OpenCode",
        format: FormatHealth::Ok,
    }
}

/// The wall ms of a FILETIME.
fn wall_ms(start: FileTime) -> i64 {
    (start.0 / 10_000) as i64 - 11_644_473_600_000
}

#[test]
fn image_since_counts_a_process_of_the_image_that_started_early_enough() {
    let source = Shared::new(SourceId::OpenCode);
    let (mut agents, log) = agents(&[&source]);
    let live = FakeLiveness::default();
    // A start on a whole millisecond, so the row times below are exact.
    let server = ProcessRef {
        pid: 7,
        start: FileTime(134_381_211_391_950_000),
    };
    live.processes.lock().unwrap().extend([
        entry(server, "OpenCode-CLI.EXE"),
        entry(process(8), "node.exe"),
        entry(process(9), "opencode.exe.bak"),
    ]);
    let started = wall_ms(server.start);
    let alive_for = |agents: &mut Agents, since| {
        source.set(|f| f.sessions = vec![opencode_session(since)]);
        agents.forget_posted();
        let snapshots = agents.snapshots(&live, START);
        (only_alive(&snapshots), snapshots.live)
    };

    assert_eq!(alive_for(&mut agents, None), (Alive::Yes, vec![server]));
    assert_eq!(
        alive_for(&mut agents, Some(started)),
        (Alive::Yes, vec![server])
    );
    assert_eq!(
        alive_for(&mut agents, Some(started - IMAGE_SINCE_SLACK_MS)),
        (Alive::Yes, vec![server]),
        "started within the slack after the row's time"
    );
    assert_eq!(
        alive_for(&mut agents, Some(started - IMAGE_SINCE_SLACK_MS - 1)),
        (Alive::Gone, vec![]),
        "every live process started after the row: a crash leftover"
    );
    assert!(live.checked.lock().unwrap().is_empty(), "no handle check");
    assert_eq!(
        lines(&log),
        [format!(
            "OpenCode: session {} shows work but counts as ended: no opencode.exe or opencode-cli.exe that \
                 started before it runs",
            short_id("ses_1")
        )],
        "said once, when it ends a working session"
    );

    log.lines.lock().unwrap().clear();
    *live.list_error.lock().unwrap() = Some(RM_FAILED);
    source.set(|f| f.sessions = vec![opencode_session(None)]);
    let later = Now {
        mono_ms: START.mono_ms + LOG_EVERY_MS,
        ..START
    };
    let snapshots = agents.snapshots(&live, later);
    assert_eq!(
        (only_alive(&snapshots), snapshots.live),
        (Alive::Unknown, vec![])
    );
    assert_eq!(
        lines(&log),
        ["OpenCode: a session's process could not be checked, so its liveness is unknown"]
    );
}

#[test]
fn image_since_says_nothing_of_an_idle_session_or_no_image_at_all() {
    let source = Shared::new(SourceId::OpenCode);
    let (mut agents, log) = agents(&[&source]);
    let live = FakeLiveness::default();
    let idle = SessionFacts {
        activity: Activity::Idle,
        ..opencode_session(None)
    };
    source.set(|f| f.sessions = vec![idle]);
    assert_eq!(only_alive(&agents.snapshots(&live, START)), Alive::Gone);
    assert!(
        lines(&log).is_empty(),
        "an idle session holds nothing either way"
    );

    // A schedule's entry: any start counts, and no process of the image runs at all.
    let timer = SessionFacts {
        activity: Activity::Idle,
        timers: vec![START.wall_ms + 60_000],
        ..opencode_session(None)
    };
    source.set(|f| f.sessions = vec![timer]);
    assert_eq!(only_alive(&agents.snapshots(&live, START)), Alive::Gone);
    assert_eq!(
        lines(&log),
        [format!(
            "OpenCode: session {} shows work but counts as ended: no opencode.exe or opencode-cli.exe runs",
            short_id("ses_1")
        )]
    );
}

#[test]
fn image_since_asks_for_the_process_list_once_per_call() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[derive(Default)]
    struct Counting(FakeLiveness, AtomicUsize);
    impl Liveness for Counting {
        fn check(&self, process: ProcessRef) -> Alive {
            self.0.check(process)
        }
        fn start_of_pid(&self, pid: u32) -> wakeshot_core::process::Lookup {
            self.0.start_of_pid(pid)
        }
        fn list(&self) -> Result<Vec<ProcessEntry>, crate::WinError> {
            self.1.fetch_add(1, Ordering::SeqCst);
            self.0.list()
        }
    }
    let a = Shared::new(SourceId::OpenCode);
    a.set(|f| f.sessions = vec![opencode_session(None), opencode_session(Some(0))]);
    let b = Shared::new(SourceId::VsCode);
    b.set(|f| f.sessions = vec![opencode_session(None)]);
    let (mut agents, _) = agents(&[&a, &b]);
    let live = Counting::default();
    agents.snapshots(&live, START);
    assert_eq!(live.1.load(Ordering::SeqCst), 1);
}

/// The real process list: this test's own process, by its own image name and start.
#[test]
fn image_since_finds_a_real_process_by_its_image_and_start() {
    let me = crate::process::current().unwrap();
    let exe = std::env::current_exe().unwrap();
    let image = exe.file_name().unwrap().to_str().unwrap().to_uppercase();
    let images: &'static [&'static str] =
        Box::leak(vec![&*Box::leak(image.into_boxed_str())].into_boxed_slice());
    let session = |since_wall_ms| SessionFacts {
        liveness: sources::Liveness::ImageSince {
            images,
            since_wall_ms,
        },
        ..opencode_session(None)
    };
    let source = Shared::new(SourceId::OpenCode);
    let (mut agents, _) = agents(&[&source]);
    let live = crate::process::SystemLiveness;

    source.set(|f| f.sessions = vec![session(Some(wall_ms(me.start)))]);
    let snapshots = agents.snapshots(&live, START);
    assert_eq!(only_alive(&snapshots), Alive::Yes);
    assert!(snapshots.live.contains(&me), "{:?}", snapshots.live);

    // A row from before this process started: every process of the image is newer.
    source.set(|f| f.sessions = vec![session(Some(wall_ms(me.start) - 60_000))]);
    let snapshots = agents.snapshots(&live, START);
    assert!(!snapshots.live.contains(&me), "{:?}", snapshots.live);
}

/// A workspace folder that cannot be listed hides its files, so they are not gone; the others still are.
#[test]
fn recent_files_under_a_folder_that_cannot_be_listed_are_not_gone() {
    let root = TempDir::new("agents-recent-no-list");
    let hidden = root.write(r"ws1\chatSessions\a.jsonl", b"a\n");
    let other = root.write(r"ws2\chatSessions\b.jsonl", b"b\n");
    let source = Shared::new(SourceId::VsCode);
    source.set(|f| f.wanted = vec![recent(root.path(), 1, Some("chatSessions"))]);
    let (mut agents, log) = agents(&[&source]);
    agents.on_tick();
    assert_eq!(source.fed().len(), 2);

    let _no_backup = NoBackup::on();
    let denied = NoListing::on(&root.path().join(r"ws1\chatSessions"));
    fs::remove_file(&other).unwrap();
    agents.on_tick();
    assert_eq!(source.fed(), [gone(&other)], "{hidden:?} is not gone");
    agents.snapshots(&FakeLiveness::default(), START);
    assert_eq!(
        lines(&log),
        ["VS Code chat: a folder it reads could not be listed (error 5)"]
    );

    drop(denied);
    agents.on_tick();
    assert!(source.fed().is_empty(), "unchanged, so not read again");
}
