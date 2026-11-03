# Pack script and release workflow

Phase: 6. Status: ready. Needs Haik: no. His answer to Question 5 of `p6-plan-ship` is yes: the first real run is a
public pre-release, which Phase 8 deletes (`docs/phases/02-06-signoff.md` item 9, default, Haik 2026-11-03 18:09).
Agent: `wakeshot-worker`. Size: about 110K tokens.
Real laptop: no. It packs into the scratch folder and never installs; `smoke-test.ps1` first runs on this laptop in
`p6-install-e2e`, and on the runners after the next push.
Runs after: `p6-velopack-facts` (names, flags, the upload options). Parallel with: any card that does not own the
files below.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p6-pack-and-release\target`; packs go to
`<scratchpad>\p6-pack-and-release\out`.

## Goal

One script packs a Velopack release for one CPU, here and in CI; one workflow builds, packs, smoke-tests and uploads
both CPUs when a version tag is pushed.

## Read first

- `docs/phases/06-ship.md`: "Package and install values", "Release".
- `docs/architecture.md` section 10 (targets, static CRT, never `RUSTFLAGS`, action pinning).
- `.github/workflows/ci.yml` (the build, test and CRT check steps to match).
- `docs/research/distribution.md` sections 1, 3 ("Velopack plus SignPath") and 4; `docs/research.md`: "CI workflow",
  "Ship track planning checks", "Velopack checks for Wakeshot".

## Deliverables

- `packaging/pack.ps1`: inputs version, CPU (`x64` or `arm64`), the folder with the release exes, the output folder,
  an optional icon and an optional folder of earlier releases (for deltas). Copies only `wakeshot-tray.exe` and
  `wakeshot.exe` into a staging folder, runs `vpk pack` with the values in the phase file, and stops on any non-zero
  exit code.
- `packaging/smoke-test.ps1`: silent install of a given `Setup.exe`; checks the files in `current`, the Start menu
  shortcut, the uninstall key, the Run value, the Path entry and the install hook's log line; silent uninstall;
  checks that the folder, the key, the Run and RunOnce values, the Path entry and the shortcut are gone and that the
  user Path has its old bytes. Prints one line per check and exits non-zero on any failure. It never touches power
  settings.
- `.github/workflows/release.yml`:
  - on a `v*` tag, and by hand (`workflow_dispatch`) as a dry run that packs and smoke-tests but uploads nothing;
  - fails when the tag is not `v` plus `[workspace.package] version`;
  - per CPU on its own runner (`windows-latest`, `windows-11-arm`): the same build, test and CRT check as `ci.yml`,
    `--locked`, never `RUSTFLAGS`; `dotnet tool install -g vpk --version 1.2.161`; `vpk download github` for deltas;
    `pack.ps1`; `smoke-test.ps1`; the release files as artifacts;
  - one upload job, the only one with `contents: write`, runs `vpk upload github` for both channels into one release,
    as `p6-velopack-facts` found;
  - two comments mark where SignPath goes later: the exes before `vpk pack`, `Setup.exe` after it (decision 44);
  - actions from `actions/` by major tag, any other action pinned by commit SHA (section 10).
- `packaging/README.md`: how to cut a release (bump the version, tag, push the tag, what to check after).

## Acceptance checks

- `actionlint` reports 0 errors on `release.yml` (as in research "CI workflow": the release exe, its SHA-256 checked
  against the release checksum list).
- Both scripts parse (`[System.Management.Automation.Language.Parser]::ParseFile`, no errors).
- `pack.ps1` run here on the repo's x64 release exes produces `Setup.exe`, the full `.nupkg`, the portable zip and the
  release index in the out folder; the same for ARM64 with the ARM64 release exes (if `velopack` is in and this laptop
  has no clang, use the exes of a build from before it, or record ARM64 as CI only). Sizes in the Log.
- The GitHub run waits for the next push: the orchestrator runs the manual dry run then and records it here; the first
  tagged run is the public pre-release Haik said yes to (Question 5).

## Out of scope

- Installing anything on this laptop; the winget manifest (`p6-winget-manifest`); `ci.yml`; signing itself.

## Questions for Haik

Each with the worker's default.

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-03: card written by `p6-plan-ship`.

## Result

What was done, files changed, checks and outcomes, open items.
