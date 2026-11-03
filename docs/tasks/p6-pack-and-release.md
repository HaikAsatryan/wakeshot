# Pack script and release workflow

Phase: 6. Status: done. Needs Haik: no. His answer to Question 5 of `p6-plan-ship` is yes: the first real run is a
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

None blocking. The worker's calls for his review are in Result.

## Found outside scope

- `06-ship.md` "Hooks" says 12 s for the updated hook; `hooks.rs` uses 10 s (already known, `p6-hooks`).

## Log

- 2026-11-03: card written by `p6-plan-ship`.
- 2026-11-04 02:02: started (bundle with `p6-winget-manifest`, this card first). Build folder
  `<scratchpad>\p6-pack\target`, packs to `<scratchpad>\p6-pack\out` (the orchestrator's names). Inputs decided
  2026-11-04 for Haik's review: winget `RequireExplicitUpgrade: true`; the manifest version equals the tag and
  Velopack's DisplayVersion; the smoke test's DisplayVersion check and the winget job belong here
  (`docs/research/launch.md` section 2); no paid certificate, free paths only (Haik 01:14); the first real run is a
  public pre-release (Question 5).
- 2026-11-04 02:08: release builds of `wakeshot-tray` and `wakeshot-cli` (`--locked`, no `RUSTFLAGS`) for x64 (1 min
  8 s) and ARM64 (56 s, no clang needed: `velopack` is not in yet). Exes: x64 tray 426,496 and CLI 667,648 bytes;
  ARM64 406,016 and 615,936.
- 2026-11-04 02:09: `vpk download github` on this repo (no releases yet): "No releases found", exit 0, no folder
  written. So the first tagged run packs with no delta. Source facts (Velopack 1.2.161): DisplayVersion is
  `major.minor.patch` only, hooks get the full version; see `docs/research.md` "Release workflow checks".
- 2026-11-04 02:10: `pack.ps1` ran for real. x64 0.1.0: Setup.exe 10,539,825, full nupkg 2,982,193, portable zip
  2,974,717, `releases.win-x64.json` 283, `assets.win-x64.json` 251, `RELEASES-win-x64` 98 bytes; vpk took 3 s.
  ARM64 0.1.0: Setup.exe 8,821,095 (PE machine 0xAA64), full nupkg 2,869,607, zip 2,862,130. x64 0.1.1 with the 0.1.0
  folder as `-PreviousDir`: delta nupkg 8,975 bytes. `0.1.2-rc.1` packs. Refused, exit 1: ARM64 exes as x64 and the
  other way, a non-empty out folder, `0.1.0+abc`, a stray `VPK_DELTA`, a version not above an earlier one (vpk's own
  error, exit -1). actionlint download and run was denied by the auto mode classifier; the workflow gets a careful
  read instead.
- 2026-11-04 02:14: `smoke-test.ps1` written. Not run against a real Setup (no installs here). Run with a harmless
  fake Setup (`whoami.exe`, which installs nothing; this laptop has no Wakeshot trace, checked read-only first): every
  install check FAILs with a clear line, every gone check PASSes, the Path bytes compare equal, exit 1; with a missing
  Setup path, exit 2. Its registry reader returned the real user Path as type 2, 332 bytes (read only).
- 2026-11-04 02:16: `release.yml` written; its version step ran locally: tag `v0.1.0` passes, `v0.1.1` and `V0.1.0`
  fail, a branch passes, `0.1.0-rc.1` gives pre-release true, `0.1.0+build.5` fails.
- 2026-11-04 02:19: `packaging/README.md` and research section "Release workflow checks" written. Card done.

## Result

Done 2026-11-04 02:19.

**Built**
- `packaging/pack.ps1`: version, CPU, exe folder, out folder, optional icon and earlier-releases folder. Copies only the
  two exes to a temp staging folder, runs `vpk pack` with the phase file's values, stops on any failure. Also refuses a
  wrong-CPU exe (PE machine field), a non-empty out folder, a version with build metadata, and any `VPK_*` variable
  (vpk reads each as a pack option; source-checked).
- `packaging/smoke-test.ps1`: `Setup.exe --silent`, then the checks the card lists, plus DisplayVersion equal to the
  version's three numbers, `wakeshot.exe` with no arguments exits 0, and the data folder gone after uninstall. Runs the
  key's `QuietUninstallString`, waits for the delayed folder delete, compares the user Path type and bytes (P/Invoke
  `RegQueryValueExW`). One line per check; exit 0, 1 on a failed check, 2 when it refuses. Refuses when any trace of
  Wakeshot is there, so the hooks never find a journal to restore. Prints this run's Velopack and hook log lines on
  failure.
- `.github/workflows/release.yml`: `v*` tag and `workflow_dispatch` (dry run). Version job checks tag = `v` +
  `[workspace.package] version`; build job per CPU on `windows-latest` and `windows-11-arm`: test, release build, CRT
  check (as `ci.yml`), the two SignPath comments, `dotnet tool install --global vpk --version 1.2.161`,
  `vpk download github`, `pack.ps1`, artifact, `smoke-test.ps1`. One upload job with `contents: write`: x64 as a
  draft, then ARM64 with `--merge --publish`, `--tag v<version>`. A winget job (stable only, gated). Actions by major
  tag (`checkout@v7`, `upload-artifact@v7`, `download-artifact@v8`); no third-party action; `wingetcreate.exe`
  pinned by SHA-256.
- `packaging/README.md`: pack locally, cut a release, recover from a half upload, what the smoke test checks.
- `docs/research.md`: "Release workflow checks".

**Checks**
- Both scripts parse: 0 errors each.
- `pack.ps1` for real: x64 Setup.exe 10.5 MB, full nupkg 3.0 MB, zip 3.0 MB, index files; ARM64 Setup.exe 8.8 MB,
  nupkg 2.9 MB, zip 2.9 MB; a delta of 9 KB. Exact bytes in the Log.
- actionlint: **not run**. The download was refused by the auto mode classifier and no YAML parser is on this laptop.
  Careful read instead; the version step's PowerShell ran locally on six cases. The orchestrator should run actionlint
  (as `p1-ci` did) before the push.
- The GitHub dry run waits for the next push (orchestrator).

**Decided by the worker, for Haik's review**
1. A pre-release is chosen by the version: `X.Y.Z-rc.N` gives a GitHub pre-release (`vpk upload --pre`) and no
   winget; `X.Y.Z` a normal release. So Question 5's public pre-release needs the Cargo version `0.1.0-rc.1` and tag
   `v0.1.0-rc.1`. Change it when Haik wants every 0.x release to be a pre-release.
2. No cargo cache in `release.yml` (clean builds; no cache poisoning). Change it when a release run passes 30 minutes.
3. The winget job runs only when the repository variable `WINGET_SUBMIT` is `true` and uses secret `WINGET_TOKEN`
   (rule 8: the first submission is a Phase 8 hand step).
4. The smoke test runs the exact winget commands with no `--log` flag, and reads Velopack's logs from
   `%LocalAppData%\velopack` instead.
5. Release artifacts are uploaded before the smoke test, kept 7 days, so a failed smoke test still leaves the packs.

**Before the first tag (Haik or the orchestrator)**
- Run actionlint on `release.yml`; push; run the dry run from Actions.
- The dry run's smoke test fails at the Run value, Path entry and hook log checks until the tray calls `VelopackApp`
  and the hooks (`p6-velopack-dependency`). That is the check working.
- For Question 5: set the version to `0.1.0-rc.1`, `cargo update --workspace`, commit, tag `v0.1.0-rc.1`, push the tag.

**Needs another card**
- `p6-velopack-dependency`: `velo.rs` must build `GithubSource` with `prerelease: false` for users, or pre-releases
  reach them; the update e2e that uses a GitHub pre-release (Question 5) needs a way to ask for pre-releases.
- Phase file `06-ship.md` "Release" (not edited here): add the pre-release-by-version rule, the winget job and its
  gate, and that a dry run fails while the version is not above the last release.

**Outside scope**
- None new. Known: `06-ship.md` "Hooks" still says 12 s for the updated hook (code uses 10 s).
- 2026-11-04 02:40, orchestrator (session `wakeshot-46`): manual dry run of `release.yml` on 8ff7fa5, run 41731897399.
  Version job passed; on both CPUs build, pack and the silent install and uninstall passed, with 22 smoke checks each.
  3 checks failed on each, as expected until `p6-velopack-dependency` wires `VelopackApp`: the Run value, the user Path
  entry, the install hook's log line (Velopack logs "Hook exited with non-zero exit code: 2": the tray exe does not
  know the hook arguments yet). Upload and winget jobs skipped. Run it again after `p6-velopack-dependency`.
