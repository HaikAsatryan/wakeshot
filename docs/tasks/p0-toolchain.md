# Set up the Rust toolchain

Phase: 0. Status: done. Needs Haik: yes, before start: install Visual Studio Build Tools (admin, command below).
Runs after: none. Parallel with: `p0-distribution-facts`.

## Goal

A working Rust toolchain on this laptop that builds native x64 and ARM64 Windows binaries.

## Read first

`CLAUDE.md`.

## Deliverables

- rustup installed per user (`winget install Rustlang.Rustup`, no admin), stable toolchain, targets
  `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc`, components `rustfmt`, `clippy`, `rust-analyzer`.
- The official Claude Code Rust LSP plugin installed (Haik's global rule: install it without asking). Find its exact
  name in the official plugin marketplace first.
- A throwaway hello-world in your scratch folder built for both targets.
- Versions recorded in `docs/research.md` (new short section "Toolchain").

## Acceptance checks

- `cargo build --release --target x86_64-pc-windows-msvc` and `--target aarch64-pc-windows-msvc` both succeed.
- The x64 binary runs and prints.
- `cargo clippy` and `cargo fmt --check` run.

## Out of scope

Any project code. The Cargo workspace is Phase 1.

## Haik's command (admin, UAC prompt)

```
winget install --id Microsoft.VisualStudio.BuildTools --override "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --add Microsoft.VisualStudio.Component.VC.Tools.ARM64 --includeRecommended"
```

## Questions for Haik

None.

## Found outside scope

- `docs/plan.md` Status still says "Waiting on Haik: Build Tools install". That is done now.
- Sessions and shells started before the rustup install do not have `.cargo\bin` on PATH. That includes the
  orchestrator and the parallel `p0-distribution-facts` worker. They can call `%USERPROFILE%\.cargo\bin\cargo.exe`
  directly, or start a new session. The Rust LSP plugin also loads only in a new session.

## Log

- 2026-11-03: card written.
- 2026-11-03: worker started. Build Tools 2026 (18.11.2) installed by Haik, per the orchestrator.
- 2026-11-03: `winget install --id Rustlang.Rustup --scope user` failed ("No applicable installer found": the
  manifest declares no scope). Without `--scope` it installed rustup 1.29.2 per user, no admin, into
  `%USERPROFILE%\.cargo` and `%USERPROFILE%\.rustup`; `.cargo\bin` added to the user PATH. Stable 1.99.1, profile
  default. Added target `aarch64-pc-windows-msvc` and component `rust-analyzer` (rustfmt, clippy came with the profile).
- 2026-11-03: hello-world (`cargo new hello --vcs none`) in the scratch folder built in release for both targets.
  rustc found the Build Tools 2026 `link.exe` (HostX64\x64 and HostX64\arm64) with no env setup. x64 exe prints
  "Hello, world!". clippy and fmt pass.
- 2026-11-03: refreshed the `claude-plugins-official` marketplace, installed `rust-analyzer-lsp@claude-plugins-official`
  1.0.0 at user scope. `claude plugin list` shows it enabled.
- 2026-11-03: facts written to `docs/research.md`, section "Toolchain".

## Result

Done. The Rust toolchain on this laptop builds native x64 and ARM64 Windows binaries.

What was done:

- rustup 1.29.2 per user through winget (no admin). Stable Rust 1.99.1, targets `x86_64-pc-windows-msvc` and
  `aarch64-pc-windows-msvc`, components rustfmt, clippy, rust-analyzer.
- Claude Code plugin `rust-analyzer-lsp@claude-plugins-official` 1.0.0, user scope.
- Throwaway hello-world in
  `C:\Users\haika\AppData\Local\Temp\claude\C--Users-haika-code-wakeshot\1828f2de-68d5-4148-9124-8321986ceb05\scratchpad\hello`.
  Left in place in case the Velopack trial wants a small exe: `target\x86_64-pc-windows-msvc\release\hello.exe`.

Files changed: this card; `docs/research.md` (new section "Toolchain", including the Build Tools facts).

Acceptance checks:

- `cargo build --release --target x86_64-pc-windows-msvc`: pass. Exe machine type 0x8664.
- `cargo build --release --target aarch64-pc-windows-msvc`: pass. Exe machine type 0xAA64.
- x64 binary runs and prints "Hello, world!", exit 0: pass.
- `cargo clippy --all-targets -- -D warnings`: pass (also for the ARM64 target). `cargo fmt --check`: pass on clean
  code; prints a diff on badly formatted code.

Open items:

- The ARM64 exe was not run: no ARM64 device here.
- Not verified that the Rust LSP plugin serves a session: it loads only in a session started after the install.
- Current sessions need a restart (or the full path) to find `cargo`. See "Found outside scope".
