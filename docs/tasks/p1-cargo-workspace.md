# Create the Cargo workspace skeleton

Phase: 1. Status: done. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 120K tokens (two targets, `dumpbin`,
maybe a first toolchain download).
Runs after: Haik's design sign-off. Parallel with: `p1-repo-files` (it writes only `LICENSE`, `README.md`,
`.gitignore`, `.gitattributes`) and `p1-tray-menu-look` (it builds a spike; see the first deliverable about `exclude`
and the note on spike builds). No power settings, screen or sound in this card.

Spike builds after this card: cargo reads `.cargo/config.toml` and rustup reads `rust-toolchain.toml` from parent
folders, so every spike built from `spikes/` after deliverables 2 and 3 uses the pinned toolchain and the static CRT,
with a full rebuild. Spike sizes and imports then differ from what Phase 0 recorded. Note the time you write the two
files in the Log, so the orchestrator can tell `p1-tray-menu-look` if it still runs.

## Goal

The workspace from `docs/architecture.md` sections 2, 10 and 11: four crates that build for x64 and ARM64, one test
per crate, fmt and clippy clean. No product logic yet.

## Read first

- `CLAUDE.md`; `docs/plan.md` "How we work".
- `docs/architecture.md`: sections 2 (crates, modules), 10 (CI and build targets), 11 (dependencies).
- `docs/research.md`: "Toolchain", the "Crates" and "Manifest needed" bullets of "Tray, menu and notifications", and
  "Phase 1 design checks".
- `spikes/tray/Cargo.toml`, `spikes/tray/build.rs`, `spikes/tray/tray-spike.exe.manifest`: the tested manifest and
  resource setup. Read only.
- Current docs for `embed-resource` 3.0.12 on docs.rs (the call that embeds a `.rc` file), not memory.

## Deliverables

1. Root `Cargo.toml`: `[workspace]` with `members = ["crates/*"]`, `exclude = ["spikes"]` and `resolver = "3"`, in
   the same first write, so a spike build running in a parallel card never sees a workspace without `exclude`.
   `[workspace.package]` (edition 2024, version 0.1.0, license MIT, `rust-version = "1.99"`, repository
   `https://github.com/HaikAsatryan/wakeshot`). `[workspace.dependencies]` with the versions from architecture
   section 11, only for crates this card uses. `[profile.release]`: `lto = true`, `codegen-units = 1`,
   `panic = "abort"`, `strip = true`.
2. `rust-toolchain.toml`: channel `1.99.1`, components `rustfmt` and `clippy`, targets `x86_64-pc-windows-msvc` and
   `aarch64-pc-windows-msvc`. The first cargo call may download that toolchain; record it in the Log.
3. `.cargo/config.toml`: `-C target-feature=+crt-static` for both MSVC targets, with a one-line comment why.
4. `crates/wakeshot-core`: lib, `#![forbid(unsafe_code)]`, no dependencies yet. Module `time` with
   `Now { wall_ms: i64, mono_ms: u64 }` and one unit test.
5. `crates/wakeshot-windows`: lib, depends on `wakeshot-core` and `windows` 0.62.2 with only the features it uses.
   Module `clock` with `now() -> Now` (`GetSystemTimePreciseAsFileTime`, `QueryUnbiasedInterruptTime`) and one test
   (wall time after 2026-01-01, mono time above 0). No power API, nothing that reads or writes power settings.
6. `crates/wakeshot-tray`: bin `wakeshot-tray`, `#![windows_subsystem = "windows"]`. `main` parses the mode
   (`--guard`, `--restore`, `--dry-run`, none) and exits 0 at once. `build.rs` embeds a manifest (asInvoker, Common
   Controls 6, per-monitor v2 DPI, Windows 10 and 11 `supportedOS`, as in the tray spike) and a version resource
   (`FileDescription` and `ProductName` "Wakeshot"). One test for the mode parsing.
7. `crates/wakeshot-cli`: bin `wakeshot`, console. With no arguments or `--help` it prints a short usage and exits 0.
   `build.rs` embeds a manifest (asInvoker, `supportedOS`) and a version resource (`FileDescription` "Wakeshot
   command line"). One test.
8. `Cargo.lock` (bins: it is kept).
9. `CLAUDE.md`: replace the "Status:" paragraph with the state after this card (Phase 1; workspace in `crates/`,
   skeleton only; spikes stay outside it but now build with the root toolchain and static CRT; say whether a git repo
   exists by reading the folder, not by running git) and add one line with the check commands. Replace lines; do not
   append history.

## Acceptance checks

Record each command and its outcome in the Log.

- `cargo fmt --all --check` passes.
- `cargo clippy --workspace --all-targets --target <t> -- -D warnings` passes for both targets.
- `cargo test --workspace` passes on x64: at least one test in each of the four crates.
- `cargo build --release --workspace --target <t>` passes for both targets. A script reads the PE machine field of
  each exe: `0x8664` for x64, `0xAA64` for ARM64. Record each exe's size.
- `dumpbin /dependents` on the four release exes: no `VCRUNTIME140.dll`, `MSVCP140.dll` or `api-ms-win-crt-*`
  import (static CRT). List what each imports.
- The tray exe holds an `RT_MANIFEST` with Common Controls 6 and `PerMonitorV2` (extract it with `mt.exe` from the
  Windows SDK, or read the resource with a script).
- Both x64 exes run with no arguments and exit 0 within 5 s (winget validation runs exes with no arguments). Start
  them with `Start-Process -PassThru -Wait` and stop only that PID if one hangs.
- `cargo tree -p wakeshot-core` lists no `windows` crate.
- `cargo metadata --no-deps --format-version 1 --manifest-path spikes\<name>\Cargo.toml` succeeds for each of the
  four spikes: they stay outside the workspace.

## Out of scope

- Product logic (Phases 2 to 6). Serde, proptest and velopack come with the code that uses them.
- CI (`p1-ci`), `.gitignore` and the other repo files (`p1-repo-files`), git of any kind.
- Spikes: never change them. Running the tray in a mode that touches power, the screen or sound.
- Installing a non-Windows Rust target. CI covers the Linux build of the core.

## Questions for Haik

Each with the worker's default.

## Found outside scope

Problems seen but not fixed.

- `p1-ci`: CI must never set `RUSTFLAGS` (for example the common `RUSTFLAGS=-D warnings`): it replaces the config
  flags, so the CI exes would need the Visual C++ runtime (control build, research "Workspace build"). Use
  `cargo clippy ... -- -D warnings` instead. A `dumpbin /dependents` step in CI would catch a regression.
- `p1-ci`: rust-analyzer is not in the pin's components (card scope). Each dev machine adds it once with
  `rustup component add rust-analyzer --toolchain 1.99.1`, or the LSP plugin fails in this repo. Change it when a
  second machine is set up: list it in `rust-toolchain.toml` (CI then downloads it too).
- `docs/architecture.md` section 10 says that the CRT DLL link and its removal by the flag are "not verified; the
  workspace card checks the imports". Now checked (research, "Workspace build"); the docs worker can drop the caveat.
- `docs/architecture.md` section 2 says both bins depend on both libs. The skeleton bins depend on neither yet,
  because nothing calls them; the first card that wires a bin adds them.
- `spikes/tray`: the release build warns `function is_menu_window is never used` (the spike's own code, edited by
  `p1-tray-menu-look`). Harmless; noted for that card.

## Log

- 2026-11-03: card written by `p1-detailed-design`.
- 2026-11-03 15:51: updated by `p1-design-review-fixes`: size, parallel lines, note on spike builds (general review
  m11). The crate layout stays two bin crates (architecture section 2 gives the reason).
- 2026-11-03 16:01: `wakeshot-worker` started the card. Status in progress.
- 2026-11-03 16:03: installed toolchain `1.99.1-x86_64-pc-windows-msvc` before writing the pin, so no parallel spike
  build triggers a download: `rustup toolchain install 1.99.1 --profile minimal --component rustfmt --component clippy
  --target x86_64-pc-windows-msvc --target aarch64-pc-windows-msvc`, 6 components, 16:03:17 to 16:03:34, exit 0. The
  `stable` toolchain (also 1.99.1) stays as it was.
- 2026-11-03 16:05: wrote root `Cargo.toml` (members, `exclude = ["spikes"]` and resolver in the first write).
- 2026-11-03 16:06: wrote `rust-toolchain.toml` and `.cargo/config.toml`. From now on every spike build under
  `spikes/` uses toolchain 1.99.1 (pinned) and the static CRT, with a full rebuild.
- 2026-11-03 16:06: added the `rust-analyzer` component to the pinned toolchain (local only, not in the file): with
  the pin, the rustup proxy runs the pinned toolchain's `rust-analyzer`, which the Claude Code LSP plugin needs.
- 2026-11-03 16:08: wrote the four crates. `Cargo.lock` created by the first cargo call (44 packages).
- 2026-11-03 16:09: checks, all with `CARGO_TARGET_DIR` in the worker's scratch folder:
  - `cargo fmt --all --check`: exit 0 (after one `cargo fmt --all` that rewrapped a test line).
  - `cargo clippy --workspace --all-targets --target <t> -- -D warnings`: exit 0 for x64 and ARM64.
  - `cargo test --workspace`: exit 0. Tests: core 1, windows 2, tray 1, cli 1.
  - `cargo build --release --workspace --target <t>`: exit 0 for both.
- 2026-11-03 16:10: release exes (PE machine read by a script; imports by `dumpbin /dependents`):
  - x64 `wakeshot-tray.exe` `0x8664`, 212,992 bytes; x64 `wakeshot.exe` `0x8664`, 217,600 bytes.
  - ARM64 `wakeshot-tray.exe` `0xAA64`, 210,944 bytes; ARM64 `wakeshot.exe` `0xAA64`, 216,064 bytes.
  - All four import only `KERNEL32.dll`, `ntdll.dll`, `api-ms-win-core-synch-l1-2-0.dll`. No `VCRUNTIME140`,
    `MSVCP140` or `api-ms-win-crt-*`.
  - Tray manifest extracted with `mt.exe -inputresource:<exe>;#1` (SDK 10.0.26100.0): Common-Controls 6.0.0.0,
    `PerMonitorV2`, `asInvoker`, supportedOS `{8e0f7a12-...}` all present. CLI manifest: `asInvoker` and supportedOS.
  - Version resource: tray "Wakeshot"/"Wakeshot", CLI "Wakeshot command line"/"Wakeshot", both 0.1.0.
- 2026-11-03 16:10: both x64 exes run with no arguments: exit 0, about 1 s each including `Start-Process`. Started
  with `Start-Process -PassThru -NoNewWindow` and `WaitForExit(5000)` instead of `-Wait`, so a hang stops at 5 s;
  none hung. The CLI printed its 9-line usage.
- 2026-11-03 16:10: `cargo tree -p wakeshot-core`: only `wakeshot-core` itself, no `windows`. `cargo metadata --no-deps`
  on the four spikes: exit 0, each with its own folder as workspace root.
- 2026-11-03 16:10: spike check for `p1-tray-menu-look`: `cargo build --release --locked` from `spikes\tray` into a
  scratch target: exit 0 in 40 s, toolchain 1.99.1 from the root pin, 676,864 bytes, no CRT DLL import, its
  `Cargo.lock` unchanged (hash). One warning from the spike's own code (`is_menu_window` never used). Not run.
- 2026-11-03 16:10: control build (CLI, x64) with `RUSTFLAGS=-C debuginfo=0`: 120,320 bytes and it imports
  `VCRUNTIME140.dll` and six `api-ms-win-crt-*` DLLs. So Rust links the CRT DLL by default, the config flag removes it
  (about 97 KB more per exe), and a `RUSTFLAGS` variable silently drops it.
- 2026-11-03 16:13: `CLAUDE.md` Status replaced and checks line added; research section "Workspace build" added;
  scratch spike and control builds deleted. Status done.

## Result

Done 2026-11-03 16:14. The workspace skeleton builds for x64 and ARM64; every acceptance check passes.

Files (all new except `CLAUDE.md` and `docs/research.md`):

- `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `.cargo/config.toml`.
- `crates/wakeshot-core`: `Cargo.toml`, `src/lib.rs`, `src/time.rs` (`Now`).
- `crates/wakeshot-windows`: `Cargo.toml`, `src/lib.rs`, `src/clock.rs` (`now()`, FILETIME to Unix ms).
- `crates/wakeshot-tray`: `Cargo.toml`, `build.rs`, `wakeshot-tray.exe.manifest`, `src/main.rs`, `src/mode.rs`.
- `crates/wakeshot-cli`: `Cargo.toml`, `build.rs`, `wakeshot.exe.manifest`, `src/main.rs`.
- `CLAUDE.md`: Status paragraph replaced, one checks line added. `docs/research.md`: new section "Workspace build".

Checks (details in the Log): fmt clean; clippy `-D warnings` clean on both targets; 5 tests pass (core 1, windows 2,
tray 1, cli 1); release builds pass on both targets with PE machine `0x8664` and `0xAA64`, sizes 210,944 to 217,600 bytes; no
CRT DLL import in any exe; the tray manifest has Common Controls 6 and PerMonitorV2; both x64 exes exit 0 at once with
no arguments; `wakeshot-core` has no `windows` in its tree; all four spikes stay outside the workspace. The tray spike
still builds (scratch target, `--locked`) with the root toolchain and static CRT.

Choices made inside the card:

- `rust-toolchain.toml` also sets `profile = "minimal"`, so CI skips the docs download.
- Both manifests keep the spike's `assemblyIdentity` (required by Microsoft) with a fixed version 1.0.0.0; the product
  version lives in the version resource, generated from `Cargo.toml` by `build.rs`.
- The tray exits 2 for an unknown command line; the CLI prints an error and exits 2 for any command (none is built
  yet; its usage says so).
- The exe run used `Start-Process -PassThru -NoNewWindow` with `WaitForExit(5000)`, not `-Wait`, so no window opened
  and a hang would stop at 5 s.

Open items: none for this card. See "Found outside scope" for `p1-ci` and the docs worker.
