# Cargo and build files, spikes out, listed renames, whole-workspace checks

Phase: 8. Status: waits for the eight `p8-polish-*` cards (new 2026-11-07). Needs Haik: no (his go of 2026-11-07
02:39; every call is for his review).
Agent: `wakeshot-worker`. Size: about 110K tokens.
Runs after: `p8-polish-core-engine`, `p8-polish-core-ownership`, `p8-polish-core-sources`, `p8-polish-windows-power`,
`p8-polish-windows-tray`, `p8-polish-windows-app`, `p8-polish-windows-watch`, `p8-polish-bins`. Runs alone among
building cards (it changes `Cargo.toml` files and may change items used across crates); docs cards may run beside it.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p8-repo-config\target`; delete it when done.

## Goal

The Cargo and build files read like a normal workspace's, the throwaway spikes leave the repo, the few cross-crate
renames and moves the code cards proposed are done or set aside with a reason, and the whole workspace passes every
check on x64 and ARM64. This card closes the code polish.

## Read first

- `CLAUDE.md` (the checks; the ARM64 clang PATH note).
- `docs/phases/08-launch-cleanup.md`: "Polish rules", "Checks for a code card", "The no-code-change check", calls 1,
  6, 10.
- The Results of the eight `p8-polish-*` cards: "Rename candidates", "Structure candidates", "Found outside scope".
- Every `Cargo.toml`, `rust-toolchain.toml`, `.cargo/config.toml`, `crates/build-resources.rs`, `crates/*/build.rs`,
  `crates/*/*.exe.manifest`.

## Deliverables

1. Cargo files: comments say why, without card codes, tracks or phase file pointers (for example the `velopack` pin:
   it must match the `vpk` that `release.yml` packs with; the `idna_adapter` pin: it picks the stub without Unicode
   tables; the release profile: the tray mostly idles and its size matters). `publish = false` for every crate, through
   `[workspace.package]` if current Cargo docs allow inheriting it (call 10). No dependency, version or feature change:
   `Cargo.lock` must not change.
2. Workspace lints: add `[workspace.lints]` (and `[lints] workspace = true` in each crate) only with lints the code
   already passes and that guard something real. Try `clippy::undocumented_unsafe_blocks` first (about 590 `unsafe`
   blocks, about 590 `// SAFETY:` comments today). A lint that would need code changes is listed, not added.
3. `build-resources.rs`, `build.rs` files and the exe manifests: comments only (the `Owner:` line goes). The version
   resource's text stays as it is.
4. `spikes/` (call 1): first grep the repo outside `docs/tasks/`, `docs/reviews/`, `docs/phases/`, `docs/plan.md` for
   `spikes`; list every hit and its owner card (the docs cards should have rewritten theirs; `CLAUDE.md` is
   `p8-final-cleanup`'s). Then delete `spikes/` and the root `exclude = ["spikes"]` line with its comment.
5. Renames and moves from the code cards' candidate lists: do at most about 10 clear wins (a name that misleads, a
   module in the wrong crate folder), each with its reason; update every use in every crate; record old and new
   (`p8-final-cleanup` fixes docs that name them). Set the rest aside under "After v1" with one line each. Never rename
   an item named in a string literal (`write_art_files`), a public item `media/` or `packaging/` scripts read, or a
   test-support item without updating its users.
6. Whole-workspace checks, both targets (`x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc`; ARM64 needs the Build
   Tools `VC\Tools\Llvm\x64\bin` on that command's PATH only): `cargo fmt --all --check`; both clippy runs; `cargo test
   --workspace --locked` (x64; ARM64 tests run on ARM64 CI only); `cargo build --release --workspace --locked`.
   Then `cargo doc --workspace --no-deps --locked`: fix broken intra-doc links in comments only, in any crate.
7. Result: the Cargo changes; lints added or listed; spikes removed (file count, bytes); renames and moves done and set
   aside; every check with its outcome.

## Acceptance checks

1. `Cargo.lock` unchanged: every cargo command ran with `--locked` and passed.
2. The no-code-change check over all of `crates/` (copy it to `before\` first): every difference is a rename or move
   from deliverable 5.
3. Every check in deliverable 6 passes on both targets (tests on x64).
4. `spikes/` is gone, the `exclude` line is gone, and `cargo metadata --locked --format-version 1` succeeds.
5. The internal-name sweep (phase file, "Every card") on the owned files: zero hits, except the author's name in
   `build-resources.rs` and the manifests.

## Out of scope

Behavior, string literals, dependency versions, new dependencies, docs (other than recording renames), `media/`,
`site/`, `packaging/`, workflows. Git.

## Questions for Haik

None: decide, and log each call in the Log "for Haik's review".

## Found outside scope

## Log

- 2026-11-07: card written by `p8-plan`.

## Result
