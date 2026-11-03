# Phase 1 skeleton: general code review

Reviewer: `wakeshot-reviewer`, 2026-11-03. Read only; no git, no registry, power or screen use. Builds went to
`<scratchpad>\p1-review-general\target`.

Scope: `crates/`, `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `.cargo/config.toml`, `.github/workflows/ci.yml`,
`LICENSE`, `README.md`, `.gitignore`, `.gitattributes`, `CLAUDE.md`; cards `p1-repo-files`, `p1-cargo-workspace`,
`p1-ci`, `p1-contracts`.

Counts: 0 blocker, 0 major, 8 minor. Nothing here blocks the first public push.

## What I ran

All with the pinned toolchain (`rustc 1.99.1`), no `RUSTFLAGS` or `CARGO_ENCODED_RUSTFLAGS` in the environment.

- `cargo fmt --all --check`: exit 0.
- `cargo clippy --workspace --all-targets --locked --target <t> -- -D warnings`: exit 0 for x64 and ARM64.
- `cargo clippy --workspace --locked --target x86_64-pc-windows-msvc -- -D warnings` (no `--all-targets`): exit 0, and
  it re-checked `wakeshot-core` (see minor 2).
- `cargo test --workspace --locked --target x86_64-pc-windows-msvc`: 25 tests pass (cli 1, core 16, tray 1, windows 7).
- `cargo build --release --workspace --locked --target <t>`: exit 0 for both. PE machine `0x8664` and `0xAA64`; sizes
  210,944 to 217,600 bytes; every exe imports only `KERNEL32.dll`, `ntdll.dll`, `api-ms-win-core-synch-l1-2-0.dll`.
- The CI CRT step's exact `dumpbin` call, with the forward-slash path: works.
- `cargo doc --no-deps --workspace --target x86_64-pc-windows-msvc`: one warning (minor 3).
- `cargo tree --workspace -e normal,features`: no `test-support` on any normal edge.
- `gh api`: `actions/checkout` v7.0.1, `actions/cache` v6.1.0, `actions/upload-artifact` v7.0.2 are the latest
  releases and the major tags exist. docs.github.com lists `windows-latest`, `windows-11-arm`, `ubuntu-latest` for
  public repos. Runner image readmes: rustup 1.29.1 on both Windows images, no `RUSTFLAGS` or `RUSTUP_TOOLCHAIN` set;
  `Install-Rust.ps1` installs the native `aarch64` rustup on ARM64.

Not run: the Linux job (no Linux target here, and none installed on purpose) and anything that needs the runners.

## Findings

### Minor 1. Local checks miss `--locked`, so a track can change `Cargo.lock` without noticing

- Where: `CLAUDE.md:13-15` (the checks line). CI uses `--locked` (`ci.yml:58`, `:61`, `:64`, `:114`).
- What breaks: tracks share one working tree (`plan.md:49-57`), and `Cargo.lock` may change only through a card the
  orchestrator runs alone (`Cargo.toml:14-15`, `plan.md:52-53`). Phase 4 is expected to add a crate (`jiff` or a cron
  crate, `architecture.md:724-725`, `:941`). Its worker runs `cargo test --workspace`, which rewrites the shared
  `Cargo.lock` with no error. The orchestrator commits the card's own paths only (`plan.md:56`), so either the push
  carries a `Cargo.toml` whose lock change was left out and every CI job fails on `--locked`, or the lock change
  rides along unreviewed with the next commit.
- Fix: add `--locked` to each command in the `CLAUDE.md` checks line (and in the track cards' acceptance checks), so
  the worker gets "the lock file needs to be updated" at once.

### Minor 2. Clippy never checks the libraries as they ship

- Where: `ci.yml:58`; same command in `CLAUDE.md:13`.
- What breaks: `--all-targets` builds test targets, so the dev-dependencies of the two bins turn on `test-support` for
  `wakeshot-core` and `wakeshot-windows` for the whole run (resolver 3 unifies features per invocation). Checked: a
  second clippy run without `--all-targets` re-checks `wakeshot-core`, so the feature sets differ. Scenario: a
  `pub(crate)` helper used only by a fake in `testing.rs`. With the feature it is used; without it, it is dead code.
  Clippy with `-D warnings` passes, the release build only warns, CI stays green. Today both forms are clean.
- Fix: one more step, `cargo clippy --workspace --locked --target ${{ matrix.target }} -- -D warnings` (no
  `--all-targets`), and the same line in `CLAUDE.md`. It costs seconds: the dependencies are already checked.

### Minor 3. A doc link breaks in normal builds, and nothing checks docs

- Where: `crates/wakeshot-windows/src/lib.rs:4`, `[`testing`]`.
- What breaks: without `test-support` the `testing` module does not exist. `cargo doc` warns "unresolved link to
  `testing`" (checked). No CI step runs rustdoc, so more broken links will pile up as the tracks add docs.
- Fix: write it as plain code text, without the brackets. A rustdoc step can wait.

### Minor 4. The Linux job does not prove the core has no OS code

- Where: `ci.yml:91-114`; claim in `ci.yml:91` and `architecture.md:924-925`.
- What breaks: a track adds `[target.'cfg(windows)'.dependencies] windows = ...` to `wakeshot-core` with
  `#[cfg(windows)]` code, or uses `std::os::windows` under `cfg(windows)`. The Linux build skips all of it and passes.
  `#![forbid(unsafe_code)]` stops raw Win32 calls, but not those two cases or safe WinRT methods.
- Fix: one step in that job, `cargo tree -p wakeshot-core -e normal --target all --prefix none --locked`, failing if any
  line starts with `windows`. Today it lists only `serde`, `serde_json` and their dependencies (checked).

### Minor 5. The CRT step may pick a `dumpbin.exe` built for another CPU (unverified)

- Where: `ci.yml:71`.
- What breaks: `vswhere -find` lists the hits sorted by folder. Here the first is `Hostx64\arm64\dumpbin.exe` (an x64
  exe). A `HostARM64` folder sorts before `Hostx64`. If the x64 runner image has one, the step runs an ARM64 exe on
  x64 and fails at once: a false red, not a missed CRT. Unverified: the runner readmes do not list these folders.
  ARM64 is safe either way (it runs x64 exes).
- Fix: prefer the runner's own host folder, falling back to the first hit, for example
  `$dir = if ($env:PROCESSOR_ARCHITECTURE -eq 'ARM64') { 'HostARM64' } else { 'Hostx64' }`, then
  `$all | Where-Object { $_ -like "*\$dir\*" } | Select-Object -First 1`, else `$all[0]`. Do not name the variable
  `$host`: that is a PowerShell automatic variable.

### Minor 6. The `target/` cache is never pruned

- Where: `ci.yml:41-49`, `:103-111`.
- What breaks: a key hit never saves, so the cache changes only when `Cargo.lock` or the toolchain changes. Then the
  restore key brings back the old `target/`, cargo adds the new artifacts without deleting old ones, and the new
  entry saves both. Each lock change makes the entry bigger, toward the 10 GB repo limit, after which GitHub evicts
  entries. Small today; growth not measured.
- Fix: none now. Change it when one entry passes about 2 GB or runs get slow: `Swatinem/rust-cache` pinned by SHA (it
  prunes), as the `p1-ci` Result already says.

### Minor 7. The two `build.rs` files are copies

- Where: `crates/wakeshot-cli/build.rs:1-63`, `crates/wakeshot-tray/build.rs:1-63`. They differ only in the manifest
  name, the description and the file name.
- What breaks: Phases 5 and 6 add icons and version fields (for example `LegalCopyright`). A change made in one file
  and missed in the other gives the two exes different metadata, and nothing checks it.
- Fix: one shared file, for example `crates/build/resources.rs`, pulled in by `include!` from both `build.rs`, with a
  `cargo:rerun-if-changed` line for it. `include!` adds no crate, so `Cargo.lock` stays as it is.

### Minor 8. Two small text errors

- `CLAUDE.md:3` says the PC stays awake for "an app the user picks". `product.md:52` puts rules for any app under
  "Later, not v1". `README.md:3-4` has it right. Fix: drop the words, or add "later".
- `rust-toolchain.toml:4` says "CI on both runners"; three jobs read the file, the Linux one too. Fix: "on every
  runner".

## Checked, no finding

- **`RUSTFLAGS`:** only in comments (`ci.yml:13-14`, `.cargo/config.toml:2`). The static CRT is set per MSVC target,
  and the CI step would catch a lost one.
- **ARM64 job:** it runs on `windows-11-arm` with a native toolchain and passes `--target aarch64-pc-windows-msvc` to
  clippy, test and build, so it builds and runs the tests as ARM64 code. Artifacts are named per target, the paths
  match the `--target` output folder, `if-no-files-found: error`.
- **Cache keys:** `runner.arch` keeps x64 and ARM64 apart; `~/.cargo` is the cargo home on the Windows images (the
  install script writes to the Default profile).
- **Contracts:** no logic beyond constructors, serde and the fakes. `Guid` and `FileTime` parsing are their serde
  forms. `From<SettingKey> for PlanSetting` and `From<WinError> for Failure` are plain mappings. `Core` is a stub.
- **Fakes:** only behind `cfg(any(test, feature = "test-support"))`, and the feature arrives only through
  dev-dependencies (checked with `cargo tree`).
- **Unused dependencies:** the bins use none of `wakeshot-core`, `wakeshot-windows`, `windows`; `serde_json` is used
  only by tests in the core and not at all in `wakeshot-windows`; `proptest` is unused. All on purpose, so the tracks
  never change `Cargo.lock`. The `windows` 0.62.2 crate has no optional dependencies, so adding features to it does not
  change the lock either (checked in its manifest).
- **`Cargo.lock`:** 80 packages, crates.io only, no git or outside path sources.
- **Comments:** they give reasons; I found no comment that only repeats the code.
- **Repo files:** `LICENSE` equals the MIT text word for word, with "Copyright (c) 2026 Haik Asatryan". README claims are
  true today (in development, nothing to install; its links exist). `.gitignore` matches no file outside `target`.
  The secret file patterns were added by the orchestrator and are logged in the phase file. `.gitattributes` covers
  every extension in the tree.
- **Secrets:** a pattern grep over the scope files found nothing. Over the whole tree, binary logs included, it found
  only the known fine hits (redacted values in `spikes/claude-watch/fixtures/live-tests.log`, the test value in
  `watch.rs`, the pattern lists in the phase file and `p1-privacy-check.md`). No key, certificate or env file exists
  in the tree.
