# Start at login and the CLI on the user Path

Phase: 6. Status: done. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 70K tokens.
Real laptop: no. Tests write only scratch keys, `HKCU\Software\Wakeshot-test\<pid>\<test name>`, and each test
deletes only its own; the leftover sweep at the start deletes only keys whose pid is not running (review M8: these
tests run inside other workers' runs and in parallel threads; use `p3-store`'s `ScratchKey`). Never the real `Run` key
or `HKCU\Environment`, and never broadcast.
Runs after: the contracts card (it creates `install.rs` and its module line); `p3-store` (`ScratchKey` and
`sweep_scratch_keys`, the test key helper, review M8). Parallel with: any card that does not
own `crates/wakeshot-windows/src/install.rs`.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p6-login-and-path\target`.

## Goal

The Path half of the install and uninstall hooks: write and remove the CLI folder in the user Path, without ever
changing any other part of the user's Path. (The start-at-login Run value is `store::AutoStart`'s, review m12; the
card name stays so links keep working.)

## Read first

- `docs/architecture.md` section 6: "Registry values Wakeshot writes" and "Velopack hooks"; section 8 (errors).
- `docs/phases/06-ship.md`: "Where the code goes", "Package and install values".
- `crates/wakeshot-windows/src/lib.rs`, `error.rs` (`WinError`).

## Deliverables

`crates/wakeshot-windows/src/install.rs`, owned whole:

- No Run value code here: `store::AutoStart` (`p3-store`) is its one writer, and `p6-hooks` calls it (review m12).
- User Path: add a folder once; remove only that folder. Rules:
  - an existing entry is found case-insensitively, with or without a trailing backslash;
  - the value keeps its type (REG_EXPAND_SZ stays REG_EXPAND_SZ); a missing value is created as REG_EXPAND_SZ;
  - remove gives back the exact old bytes when nothing else changed the value (a trailing `;`, an empty value, a
    missing value, our folder listed twice by hand);
  - a value that holds only our folder is deleted on remove, so a user who had no Path value has none again.
- After a real change to Path, broadcast `WM_SETTINGCHANGE` with `"Environment"` through `SendMessageTimeoutW`
  (`HWND_BROADCAST`, `SMTO_ABORTIFHUNG`, a short timeout), so new terminals see it. Injected, so tests never broadcast.
- Every function takes the key path and value name, so tests use the scratch key; the real locations are constants.
- Errors are `WinError` with the call name. No `unwrap` on an OS result outside tests.

## Acceptance checks

- Tests written first: the Path text edit as pure functions (a table of cases, including all the rules above); Path
  add, add again, remove on the scratch key, with the bytes and type compared. Each test deletes its own scratch key at
  the end; the leftover sweep deletes only keys whose pid is not running.
- A script reads the real `HKCU\Environment` `Path` (bytes and type) and every value under the real `Run` key before
  and after `cargo test`; the worker pastes both hashes in the Log; they match.
- `rustfmt --edition 2024 --check <owned files>`; `cargo clippy --locked -p wakeshot-windows --all-targets --target <t>
  -- -D warnings` for both targets; `cargo test --locked -p wakeshot-windows install::`. Clean in the owned files; the
  own tests pass.
- Check rule: `docs/plan.md` "How we work" step 2 and "Checks while other cards run" in this track's phase file. A
  failure in a file another card owns goes in the Log: wait and run again; if it is still there when this card's own
  work is done, list it under "Found outside scope". Never edit, format or `--fix` that file. Every cargo command takes
  `--locked`, and clippy runs once more without `--all-targets` (the libs as they ship; `CLAUDE.md` "Checks"). No
  `--workspace` or `--all` run here; the orchestrator runs those when it closes a wave.

## Out of scope

- The hooks themselves (`p6-hooks`), the RunOnce value (power track, `store`), any file but `install.rs`.

## Questions for Haik

None open. The choices the worker made are under Result, "Decided by the worker, for Haik's review".

## Found outside scope

- `docs/phases/06-ship.md`, "Hooks" table: the updated hook restores "until 12 s"; `docs/tasks/p6-hooks.md` says
  10 s ("Velopack kills it at 15 s, `p6-velopack-facts` Result"). One of them is stale. Not this card's files.
- For `p6-hooks`: `add` and `remove` broadcast inside the call, which can take up to 1 s for each top-level window
  that is slow but not hung (Microsoft: the timeout is per window). With "registry first, restore last", that time
  comes out of the restore's window. The hooks may want to count it in their limits.

## Log

- 2026-11-03: card written by `p6-plan-ship`.
- 2026-11-04 01:15: started by `wakeshot-worker` (bundle with `p6-update-schedule`). Build folder
  `<scratchpad>\p6-login-update\target`, shared by both cards.
- 2026-11-04, between 01:15 and 01:25 (clock not read for this line): tests written first in `install.rs` (pure
  text table, round trips on the scratch key) with `todo!()` bodies: `cargo test --locked -p wakeshot-windows
  install::` 19 failed on `todo!()`, 1 passed (the constants test). The crate compiled with the other cards' files as
  they were.
- 2026-11-04 01:25: implemented. One test failed 3 runs in 5, alone too: a value stored without a terminator
  sometimes came back with one. Windows' `RegSetValueExW` reads the unit just past string data and stores it when it
  is a NUL (pinned by `windows_stores_a_nul_just_past_string_data`); `write` now puts a non-zero guard unit after the
  data. Then 21 of 21 pass, 11 runs in a row; no key left under `HKCU\Software\Wakeshot-test`.
- 2026-11-04 01:25: real keys, read only (`<scratchpad>\p6-login-update\real-keys-hash.ps1`: `Path` type and text,
  every `Run` value), before and after the test runs: `B4D918DF704F4224313CCB0FC288F658FA2F8F24E2C74EA161D261C706955078`
  both times. Also `reg export` of both keys, SHA-256 before and after: `HKCU\Environment`
  `22BABA5E6906DEBEE8C62E2DAFCF1134D99B7E059E497D448EB821CDD34C42D2` both times; `Run`
  `3ACCD7FACF0E23936850D5DB2BA4375FF5BD14A2A5BE1D89DE66A8F5BE6A43B4` both times.
- 2026-11-04 01:30: checks done (Result). Status done.
- 2026-11-04 01:42: the bundle's shared build folder deleted; the copies of the registry exports deleted too.

## Result

Done 2026-11-04 01:30.

What was built: `crates/wakeshot-windows/src/install.rs`, the user Path half of the hooks. No Run value code.

API for `p6-hooks`:

```rust
pub const ENVIRONMENT_KEY: &str = "Environment"; // under HKCU
pub const PATH_VALUE: &str = "Path";
pub type Broadcast = fn() -> Result<(), WinError>;
pub enum PathChange { Changed { broadcast: Result<(), WinError> }, Unchanged }
pub struct UserPath; // Clone, Debug
impl UserPath {
    pub fn real() -> Self; // HKCU\Environment, Path, broadcast_environment
    pub fn at(key: &str, value: &str, broadcast: Broadcast) -> Self;
    pub fn add(&self, folder: &Path) -> Result<PathChange, WinError>;
    pub fn remove(&self, folder: &Path) -> Result<PathChange, WinError>;
}
// WM_SETTINGCHANGE "Environment" to HWND_BROADCAST, SMTO_ABORTIFHUNG, 1 s per window.
pub fn broadcast_environment() -> Result<(), WinError>;
```

Tests elsewhere: `UserPath::at(scratch.path(), PATH_VALUE, || Ok(()))` (a closure without captures is a
`Broadcast`), with `crate::store::ScratchKey`.

- `PathChange::Changed { broadcast: Err(_) }`: the value did change; only the announce failed (a log line is
  enough). Errors carry the call name: `RegCreateKeyExW`, `RegOpenKeyExW`, `RegQueryValueExW`, `RegSetValueExW`,
  `RegDeleteValueW`, `SendMessageTimeoutW`.
- How it keeps the rules: raw `RegQueryValueExW` and `RegSetValueExW`, so the type stays and nothing is expanded or
  terminated by Windows; the edit works on UTF-16 units up to the first NUL and keeps everything from that NUL on;
  a remove joins the kept entries, which drops one separator per dropped entry and keeps empty entries.
- A Windows fact found on the way (`docs/research.md`, "Path value checks"): `RegSetValueExW` stores a NUL that lies
  just past string data that has none. `write` puts a non-zero guard unit there.

Checks:

- `cargo test --locked -p wakeshot-windows install::`: 21 passed, 0 failed (stable over 11 runs). Tests: 5 pure
  (add table of 12 cases, remove table of 13, Armenian letters in any case, add then remove gives back the old text for
  9 values, remove after a create deletes) and 16 on the scratch key (create as `REG_EXPAND_SZ` then delete, round
  trips for `REG_SZ`, unexpanded `REG_EXPAND_SZ`, trailing `;`, empty value of 0 and 2 bytes, no terminator, bytes
  after the first NUL, our folder listed twice by hand, other types and odd lengths left alone, bad folders refused,
  too long refused, remove with no key or value, failed broadcast keeps the change, name case kept, the Windows NUL
  rule, the real constants). Each test deletes its own key through `ScratchKey`; none left after the runs.
- Real keys unchanged: hashes before and after the test runs match (Log, 01:25 and a second pair at 01:28, both
  `B4D918DF...06955078`).
- `rustfmt --edition 2024 --check crates/wakeshot-windows/src/install.rs`: clean.
- `cargo clippy --locked -p wakeshot-windows --all-targets --target <t> -- -D warnings` and the same without
  `--all-targets`, for `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc`: all four clean.
- `cargo build --release --locked -p wakeshot-windows --target x86_64-pc-windows-msvc`: exit 0.
- No other card's file failed any of these runs.

Decided by the worker, for Haik's review (his rule: with no work going on, the laptop behaves as if Wakeshot were not
there):

1. Remove takes out every entry that names our folder, also ones the user typed by hand: at uninstall the folder is
   deleted, so such an entry would point at nothing. If the folder was already listed before install, add writes
   nothing and the uninstall still removes it. Change it when a user reports a hand-added entry that should stay.
2. An empty Path, or one that ends with `;`, gets `folder;` (not `;folder`), so the remove gives back the exact old
   value, including an empty value that is kept, not deleted.
3. Broadcast timeout 1 s per window, `SMTO_ABORTIFHUNG` (rustup and `setx` are said to use 5 s: not verified). Short,
   because the hooks restore power settings after the Path step within Velopack's time limit. Change it when a new
   terminal after install does not see `wakeshot`.
4. An add that would make the Path longer than Windows' 32,767-character limit is refused (`ERROR_FILENAME_EXCED_RANGE`)
   and the value is left alone.
5. A Path value of another type than `REG_SZ` or `REG_EXPAND_SZ`, or with an odd byte count, is left alone
   (`ERROR_INVALID_DATA`); a folder that is empty or holds `;` or a NUL is refused (`ERROR_INVALID_PARAMETER`).
6. Matching ignores case (Rust's lower case) and trailing backslashes only. Not matched: quotes, forward slashes,
   `%VAR%` forms of our folder. Change it when such an entry shows up in a report.

Open items: none for this card. "Needs another card": none.
