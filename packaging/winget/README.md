# winget manifest

The winget manifest of Wakeshot, package `HaikAsatryan.Wakeshot`, schema 1.28.0 (the newest that winget-pkgs
accepts; `docs/research.md`, "winget manifests: schema, validation and the submit fork").

| File | What it is |
|---|---|
| `templates/` | The three manifest files (version, installer, default locale `en-US`) with `{{VERSION}}`, `{{RELEASE_DATE}}`, `{{X64_SHA256}}` and `{{ARM64_SHA256}}` in place of the values |
| `fill.ps1` | Writes a filled manifest folder for one release |

## The rules the manifest follows

From `docs/research/launch.md` section 2 ("Rules for the winget manifest and `release.yml`"): one updater at a time.
Velopack updates Wakeshot; winget installs it, and upgrades it only when the user names it.

- `RequireExplicitUpgrade: true`: `winget upgrade --all` skips Wakeshot; `winget upgrade HaikAsatryan.Wakeshot` still
  works. This only holds for installs winget made; a user who installed `Setup.exe` from GitHub can run
  `winget pin add HaikAsatryan.Wakeshot` (not tested on Wakeshot).
- `UpgradeBehavior: install`: `Setup.exe` over an install is Velopack's own upgrade path. Not `deny`, which also blocks
  an upgrade the user asks for.
- `PackageVersion` = the tag without `v` = the `DisplayVersion` Velopack writes. Three numbers only, so pre-releases
  never go to winget, and `AppsAndFeaturesEntries` has no `DisplayVersion`.
- `ProductCode` `HaikAsatryan.Wakeshot`, the uninstall key name, also in `AppsAndFeaturesEntries` with `DisplayName`
  `Wakeshot` and `Publisher` `Haik Asatryan`, so winget also matches an install made from GitHub.
- `InstallerUrl` and `InstallerSha256` come from the tag's published release: `fill.ps1` downloads both `Setup.exe`
  files and hashes them, and fails when a hash differs from the digest GitHub lists for the asset.
- `Scope: user`, `InstallerType: exe`, silent switch `--silent`, one installer per CPU (`x64`, `arm64`).
- `InstallationNotes` tells a winget user that Wakeshot starts at the next sign-in or from the Start menu (a silent
  Setup does not start it) and that it updates itself.

## fill.ps1

```powershell
# From the published release (what CI and the first submission use):
./packaging/winget/fill.ps1 -Version 0.1.0 -OutDir <new folder>
# From local packs, before a release exists (both Setup.exe files in one folder):
./packaging/winget/fill.ps1 -Version 0.1.0 -OutDir <new folder> -FromDir <folder> -ReleaseDate 2026-11-04
winget validate --manifest <new folder>
```

It refuses a version that is not three numbers, a draft or a GitHub pre-release, a missing asset, and a non-empty out
folder (`wingetcreate submit` sends every file in the folder). `GH_TOKEN`, when set, goes to the GitHub API only.

## The PR tool: wingetcreate

`wingetcreate submit` sends a manifest folder we wrote, so every field is ours and nothing is rebuilt from the
installer. It is Microsoft's own tool, it validates the manifest before it opens the PR, and the release workflow
pins its exe by SHA-256 (v1.12.13.0). Runner-up: Komac (or the `winget-releaser` Action that runs it); change to it
when keeping `fill.ps1` and the templates costs more than it saves.

Both need a classic personal access token with the `public_repo` scope: fine-grained tokens are not supported. The
token opens the PR from a fork of `microsoft/winget-pkgs` under the token's account, which `wingetcreate` creates
when missing and syncs before each PR (its source, `GitHub.cs`). The account can be the maintainer's own or a bot
account.

## The first submission

The first version is a new package, so it goes in by hand; CI opens PRs only for later versions.

1. A normal release `vX.Y.Z` is published on GitHub (not a draft, not a pre-release).
2. `./packaging/winget/fill.ps1 -Version X.Y.Z -OutDir <new folder>`, then `winget validate --manifest <new folder>`.
3. Try the install from the local manifest on a clean Windows (Windows Sandbox): local manifests must be turned on
   once by an administrator (`winget settings --enable LocalManifestFiles`; the setting name is from the winget
   settings docs, not run here), then `winget install --manifest <new folder>`. Check that Wakeshot appears in Apps
   and Features with version `X.Y.Z`, that `winget list HaikAsatryan.Wakeshot` finds it, and whether SmartScreen asks
   (it may not for a winget install; not verified).
4. Submit with the token in a variable, so it stays out of the shell history:
   `$env:WINGET_CREATE_GITHUB_TOKEN = '<token>'`, then
   `wingetcreate submit --prtitle "New package: HaikAsatryan.Wakeshot version X.Y.Z" --no-open <new folder>`.
5. Watch the PR's validation and answer the moderators. Merged new-package PRs took a median of about 2 hours, the
   90th percentile about 33 hours (`docs/research/launch.md`; only merged PRs were counted, so the real wait is
   longer).
6. After the merge: in the GitHub repo settings, add the repository variable `WINGET_SUBMIT` = `true` and the secret
   `WINGET_TOKEN` (the same kind of token). From then on each normal release tag runs the winget job in
   `release.yml`, which fills the manifest from the published release and opens the PR.

## A note for the validation pipeline

The pipeline runs installed executables without arguments and flags a non-zero exit
(`Validation-Executable-Error`, which needs an administrator to clear; `docs/research/distribution.md`, section 2).
What the Wakeshot install holds:

- `current\wakeshot.exe` prints its usage and exits 0; the smoke test checks this on every release.
- `Update.exe` exited 1 with no arguments in a trial on the test laptop (2026-11-03; `docs/research/distribution.md`,
  section 1). Two Velopack apps (`Flowersauce.FSClicker`, `ptmrio.phrasevault`) were merged anyway; whether a
  moderator had to clear it is unknown.
- The root `Wakeshot.exe` stub exits 0 at once (it starts the tray through `Update.exe start`).
- `current\wakeshot-tray.exe` is the tray: it keeps running. How the pipeline treats an exe that does not exit is
  unknown. What settles it: the first PR. If it is flagged, answer on the PR that the tray is a background app.

## Not here

- `wingetcreate` collects telemetry; how to turn it off on a runner: not verified.
- scoop: later (`docs/research/distribution.md`, section 5).
