# Start at login and the CLI on the user Path

Phase: 6. Status: ready. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 70K tokens.
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

Each with the worker's default.

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-03: card written by `p6-plan-ship`.

## Result

What was done, files changed, checks and outcomes, open items.
