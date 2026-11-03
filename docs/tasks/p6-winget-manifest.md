# winget manifest templates

Phase: 6. Status: ready. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 60K tokens.
Real laptop: no. It validates manifests only; installing from a local manifest needs an administrator to enable
local manifests, so Phase 8 does that once before the submission.
Runs after: `p6-pack-and-release` (the release file names and URLs); `p6-launch-research` (its winget rules for an
app that updates itself; this card takes them in before it starts). Parallel with: any card that does not own
`packaging/winget/`.
Build folder: none (no Rust). Filled samples go to `<scratchpad>\p6-winget-manifest\`.

## Goal

Have the winget manifest ready for Phase 8: templates for both CPUs and a script that fills them from a release, so
the submission is one command.

## Read first

- `docs/research/distribution.md` section 2 (the manifest shape of merged Velopack apps, validation rules, the PR
  tools).
- `docs/phases/06-ship.md`: "Package and install values", "Not in this track".
- `docs/research.md`: "Velopack checks for Wakeshot" (the uninstall key name, `DisplayName`, `Publisher`).
- `packaging/README.md`, `.github/workflows/release.yml` (`p6-pack-and-release` Result).

## Deliverables

`packaging/winget/`, owned whole:

- The three manifest files (version, installer, default locale `en-US`) as templates, at the newest schema version
  winget-pkgs accepts today (check it there): `PackageIdentifier` `HaikAsatryan.Wakeshot`; `InstallerType: exe`;
  `Scope: user`; `InstallModes`, `InstallerSwitches` `Silent` and `SilentWithProgress` `--silent`; `UpgradeBehavior:
  install`; `ProductCode` and `AppsAndFeaturesEntries` from the uninstall key `p6-velopack-facts` saw; one installer
  per CPU (`x64`, `arm64`) with its GitHub Releases URL and SHA-256; MIT license, short description, homepage.
- `fill.ps1`: version in, the release files' URLs and SHA-256 out (from a local folder or a published release), a
  filled manifest folder written to a given path.
- `README.md` in that folder: how Phase 8 submits, and the PR tool picked (Komac or `wingetcreate`) with the reason.
- `docs/research.md`: the schema version and the tool facts (additions only).

## Acceptance checks

- `fill.ps1` on the local packs from `p6-pack-and-release` (or fake files with known hashes) writes a manifest folder;
  `winget validate --manifest <folder>` passes. If it needs an administrator, record that and stop there.
- The script parses (`[System.Management.Automation.Language.Parser]::ParseFile`).
- A note for Phase 8 in the folder's README: the validation pipeline runs installed exes with no arguments and flags a
  non-zero exit; `wakeshot.exe` prints its usage and exits 0, `Update.exe` exited 1 in Phase 0 and two Velopack apps
  were merged anyway, and how it treats `wakeshot-tray.exe`, which keeps running, is unknown.

## Out of scope

- The submission and any PR to winget-pkgs (Phase 8); scoop.

## Questions for Haik

Each with the worker's default.

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-03: card written by `p6-plan-ship`.

## Result

What was done, files changed, checks and outcomes, open items.
