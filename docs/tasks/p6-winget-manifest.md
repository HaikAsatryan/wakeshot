# winget manifest templates

Phase: 6. Status: done. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 60K tokens.
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

Not blocking; carried from `p6-launch-research`, needed before Phase 8:

1. The winget token (a classic token with `public_repo`): on your own account or a bot account? Default: your own.

## Found outside scope

None.

## Log

- 2026-11-03: card written by `p6-plan-ship`.
- 2026-11-04 02:20: started, right after `p6-pack-and-release` in the same bundle. Its `release.yml` already calls
  `packaging/winget/fill.ps1 -Version <v> -OutDir <dir>` (with `GH_TOKEN` set) in the winget job, so that is the
  interface. Rules from `docs/research/launch.md` section 2 ("Rules for `p6-winget-manifest` and `release.yml`"),
  taken as decided 2026-11-04 for Haik's review: `RequireExplicitUpgrade: true`, version = tag = DisplayVersion, no
  pre-release to winget, wingetcreate for the PR. Filled samples go to `<scratchpad>\p6-pack\winget\`.
- 2026-11-04 02:20: schema check: winget-pkgs `doc/manifest/schema/` has 1.0.0 to 1.12.0 and 1.28.0 (newest). Of 40
  installer manifests listed from the 40 newest master commits (merged 2026-11-03), 21 are `ManifestVersion: 1.28.0`,
  13 are 1.12.0, 6 older. So 1.28.0.
- 2026-11-04 02:21: templates and `fill.ps1` written. `fill.ps1 -FromDir` on this bundle's local packs (x64 and ARM64
  `Setup.exe` 0.1.0, both copied into one scratch folder) wrote the three files; x64 hash
  `201937ACC7D397D57D87A2235FED9679716DF9AB9C0FB3E42339E5ADC16F13AC` matches `Get-FileHash`, ARM64
  `4ABB6C607DFE2D2D04F88573E0E2F7C9BAEC89A9AFC170D08058BA028746FDFA`. `winget validate --manifest`: "Manifest validation
  succeeded.", exit 0, with and without `ReleaseDate`. A copy with `InstallerType: bogus` fails (exit -1978335191).
  Refused, exit 1: `0.1.0-rc.1`, a non-empty out folder, `2026-13-01`, no release `v0.1.0` yet (API 404), a folder
  missing the ARM64 Setup.
- 2026-11-04 02:23: `README.md` and research section "winget manifest checks" written. Card done.
- 2026-11-04 02:24: the bundle's scratch folder (`<scratchpad>\p6-pack`: target, packs, filled manifests) deleted.

## Result

Done 2026-11-04 02:23.

**Built** (`packaging/winget/`)
- `templates/`: `HaikAsatryan.Wakeshot.yaml` (version), `.installer.yaml`, `.locale.en-US.yaml`, schema 1.28.0. Installer:
  `InstallerType: exe`, `Scope: user`, `InstallModes` silent and silentWithProgress, both switches `--silent`,
  `UpgradeBehavior: install`, `RequireExplicitUpgrade: true`, `Commands: wakeshot`, `ProductCode` and
  `AppsAndFeaturesEntries` (`Wakeshot`, `Haik Asatryan`, `HaikAsatryan.Wakeshot`, no `DisplayVersion`), `ReleaseDate`,
  one installer per CPU with its release URL and hash. Locale: MIT with license URL, short description, description,
  repo as homepage, moniker, 8 tags, release notes URL, `InstallationNotes`.
- `fill.ps1`: version and out folder in; hashes from the tag's published release (downloads both Setup.exe from the
  public URL, checks GitHub's asset digest, refuses a draft or pre-release) or from `-FromDir`; release date from the
  release or `-ReleaseDate`, else left out. Refuses a non-three-number version and a non-empty out folder. This is the
  call `release.yml`'s winget job makes.
- `README.md`: the rules, `fill.ps1` use, the tool pick (wingetcreate, reason, Komac as runner-up), the Phase 8 steps
  for the first submission and for turning on the CI job, and the validation-pipeline note the card asks for.
- `docs/research.md`: "winget manifest checks".

**Checks**
- `fill.ps1` on the local packs: folder written; `winget validate` passes (exit 0); a broken copy fails.
- `fill.ps1` parses: 0 errors.
- README holds the pipeline note: `wakeshot.exe` exits 0 (the smoke test checks it), `Update.exe` exited 1 and two
  Velopack apps merged anyway, `wakeshot-tray.exe` keeps running and its treatment is unknown.
- `winget validate` ran elevated (agent shell); not checked unelevated.
- The download path of `fill.ps1` waits for the first real release (no release yet: 404 handled).

**Decided by the worker, for Haik's review**
1. Schema 1.28.0 (newest folder in winget-pkgs; most merged manifests today).
2. wingetcreate, not Komac: it submits our own files unchanged, is Microsoft's, validates first. Change it when keeping
   `fill.ps1` costs more than it saves.
3. `InstallationNotes`: "Wakeshot starts at your next sign-in, or now from the Start menu. It updates itself, so winget
   upgrade --all skips it." A silent Setup does not start the tray, so a winget user would otherwise see nothing.
4. No `MinimumOSVersion`, though the product is Windows 11 only: the validation VMs' Windows version is unknown, and a
   too-high minimum could fail the first PR. Change it when Phase 8 learns that version.
5. `PackageUrl` is the repo. Change it to the Pages site when `p6-pages` makes it live.
6. Description, tags (`keep-awake`, `sleep`, `power`, `caffeine`, `ai-agent`, `claude-code`, `lid`, `tray`), moniker
   `wakeshot` and `Commands: wakeshot`: drafts; `p6-readme` wording may replace them.
7. `ReleaseDate` is the GitHub release's publish date in UTC.

**Before Phase 8 (Haik)**
- Answer the token question above, then follow `packaging/winget/README.md` "Phase 8: the first submission"; after the
  merge, set the repository variable `WINGET_SUBMIT` to `true` and add the `WINGET_TOKEN` secret.

**Needs another card**
- None. Phase file note (not edited): `06-ship.md` "Not in this track" could point at `packaging/winget/README.md` for
  the Phase 8 steps.
