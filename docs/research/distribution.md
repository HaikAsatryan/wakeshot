# Distribution facts: installer, updates, winget, signing, CI, scoop

Checked 2026-11-03. Each fact has a source and a mark:

- **verified**: seen in source code, official docs, a real manifest, or tested on the test laptop.
- **reported**: from a third party (issue, blog). Not checked by us.
- **not verified**: a guess or unclear. Says what would settle it.

## 1. Velopack with a Rust app

Current release: Velopack 1.2.161 (GitHub release 2026-09-29), MIT, crate `velopack` 1.2.161. Verified: GitHub API
and docs.rs.

### SDK use in Rust

- `VelopackApp::build().run()` must be the first thing in `main`. It may exit or restart the process. Verified:
  docs.velopack.io/getting-started/rust and `src/lib-rust/src/app.rs`.
- Hook callbacks on `VelopackApp`, all `FnOnce(Version)`, all `#[cfg(target_os = "windows")]`. Verified in
  `app.rs` (develop branch, 2026-11-03):
  - `on_after_install_fast_callback` (arg `--veloapp-install`)
  - `on_before_update_fast_callback` (arg `--veloapp-obsolete`, runs on the old version)
  - `on_after_update_fast_callback` (arg `--veloapp-updated`, runs on the new version)
  - `on_before_uninstall_fast_callback` (arg `--veloapp-uninstall`)
  - `on_first_run`, `on_restarted` (env vars `VELOPACK_FIRSTRUN`, `VELOPACK_RESTART`; the app keeps running)
- After a fast callback returns, `run()` calls `std::process::exit(0)`, unless env `VELOPACK_DEBUG` is set. The hook
  arg must be the first arg and a version must follow it. Verified in `app.rs`.
- docs.rs shows the Linux build, so the four fast callbacks do not appear there. Verified (docs.rs page lacks them,
  source has them).
- Hook time limits. Docs (docs.velopack.io/integrating/hooks): install 30 s, uninstall 30 s, obsolete and updated
  15 s; a hook that does not exit in time is killed; hooks cannot cancel the operation and must not show UI. The
  Rust doc comments say the same. The updater source passes 30 s for install, 15 s for update hooks and **60 s** for
  uninstall (`src/bins/src/commands/uninstall.rs`). Plan for the doc value, 30 s. Verified in docs and source.

### What Update.exe and Setup.exe do (source read, `src/bins/src/commands/`)

- **Uninstall** (`uninstall.rs`): first `force_stop_package` kills every process whose exe is inside the install
  folder (TerminateProcess, no clean shutdown), then runs the uninstall hook, removes shortcuts and the folder, the
  temp folder `%TEMP%\velopack_{packId}`, and the uninstall registry key. A failed hook does not stop the uninstall.
  Verified.
- **Update apply** (`apply_windows_impl.rs`): runs `--veloapp-obsolete` on the old version (15 s), then
  `force_stop_package` on the install folder, swaps the files, then runs `--veloapp-updated` on the new version
  (15 s). Verified.
- **Install** (`install.rs`): if the folder already has files, it asks to overwrite (yes when silent), kills the
  running app with `force_stop_package`, renames the old folder for rollback, installs, runs `--veloapp-install`
  (30 s; on failure only a warning dialog), writes the uninstall registry key, and starts the app **only when not
  silent**. Verified.
- **After every hook** (`run_hook` in `src/bins/src/windows/util.rs`), Velopack calls `force_stop_package` again
  "in case the hook left running processes". So a hook cannot start the tray app or a helper from the install
  folder: it gets killed. Verified in source; not tested.
- **PATH**: no Velopack feature that adds the app folder to PATH was found in the docs, `vpk pack -H` or a web
  search. Not verified that none exists.
- **Uninstall registry key**: `HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\{packId}` with
  `DisplayName` (pack title), `DisplayVersion`, `Publisher` (pack authors), `InstallLocation`,
  `UninstallString` = `"...\Update.exe" --uninstall`, `QuietUninstallString` = `"...\Update.exe" --uninstall
  --silent`, `NoModify`, `NoRepair`. Rewritten on each update, so `DisplayVersion` follows self-updates. Verified in
  `src/bins/src/windows/registry.rs`.

### Install path, scope, silent flag

- Setup.exe installs per user to `%LocalAppData%\{packId}`, no admin. Shortcuts default to Desktop and Start Menu
  root (`--shortcuts`). Verified: docs.velopack.io/packaging/installer.
- `--instLocation` (`PerUser`, `PerMachine`, `Either`, default `Either`) applies to the optional MSI (`--msi`), not
  to Setup.exe. Verified in the same doc.
- Setup.exe flags (1.2.161): `-s, --silent`, `-v, --verbose`, `-l, --log <FILE>`, `-t, --installto <DIR>`; args for
  the app go after `--`. Verified: docs.velopack.io/reference/cli/content/setup-windows.
- Update.exe commands: `apply`, `start`, `patch`, `uninstall`, global `-s, --silent`. Verified:
  docs.velopack.io/reference/cli/content/update-windows.

### Updates from GitHub Releases

- `velopack::sources::GithubSource::new(repo_url: &str, access_token: Option<String>, prerelease: bool)`. Verified
  on docs.rs 1.2.161.
- It reads `https://api.github.com/repos/{owner}/{repo}/releases?per_page=10&page=1`, picks the newest by
  `published_at`, drops pre-releases unless asked, and downloads assets by `browser_download_url`. Doc comment:
  "Without [a token], rate limited to 60 req/hr per IP." Verified in `src/lib-rust/src/sources/github.rs`.
- Publish with `vpk upload github --repoUrl ... --token ... [--publish] [--pre] [--tag ...]`. Verified: `vpk upload
  github -h` on the test laptop.
- The default channel on Windows is `win` (`--channel`). Verified: `vpk pack -H`.

### vpk on the installed .NET 10 SDK

- Verified on the test laptop: the only SDK is 10.0.403 (runtime 10.0.14). `dotnet tool install -g vpk` installed
  `vpk` 1.2.161 and `vpk --help` runs. The tool package ships `net8.0`, `net9.0` and `net10.0` builds, so it runs
  natively on .NET 10 with no roll-forward. The Rust guide still says ".NET Core SDK 8"; that is a minimum, not a
  limit.
- Side effect seen: the first `dotnet` command on the test laptop printed "Installed an ASP.NET Core HTTPS development
  certificate" (the .NET first-run step, in the user store, not trusted). Harmless.

### Signing hooks in vpk

`vpk pack` has `--signParams` (signtool args), `--signTemplate` (custom command, `{{file}}` substituted),
`--signExclude`, `--signParallel`, `--azureTrustedSignFile`. Verified: `vpk pack -H` on the test laptop.

### Trial on the test laptop (2026-11-03, all verified)

A throwaway Rust crate (`velopack = "=1.2.161"`, Rust 1.99.1) that logs every hook to a file outside the install
folder. Packed with `vpk pack -u WakeshotTrial -v 0.1.0 -r win-x64 -p <dir> -e vtrial.exe --packTitle "Wakeshot
Trial" --packAuthors "Trial Author" --channel win-x64 --shortcuts None`. No power settings touched; everything
removed at the end (folder, registry key, temp folder all gone).

- **Sizes.** The release exe grew from about 130 KB (plain hello-world) to 1.49 MB with the
  `velopack` crate. The package also carries Velopack's `Update.exe` (5.2 MB) and a root launcher stub (467 KB).
  Full `.nupkg` 3.2 MB, `Setup.exe` **10.7 MB**, portable zip 3.2 MB. A 0.1.0 to 0.2.0 delta with one changed file
  was 2.2 KB. `vpk pack` took 1 to 6 s.
- Without `-r win-x64`, `vpk` warns "No architecture specified with --runtime, defaulting to x86". Always pass `-r`.
- **Silent install** (`Setup.exe --silent`): exit 0 in 3 s, no admin prompt, no window. Layout:
  `%LocalAppData%\WakeshotTrial\Update.exe`, `Wakeshot Trial.exe` (stub named after the pack title),
  `current\vtrial.exe`, `current\sq.version`, `packages\WakeshotTrial-0.1.0-full.nupkg`. Registry key written as in
  the source. Only `--veloapp-install 0.1.0` ran; **the app was not started** and `on_first_run` did not fire.
- **Update** from a local folder with `FileSource` and `apply_updates_and_restart`: hooks ran in this order:
  `--veloapp-obsolete 0.1.0` (old exe), `--veloapp-updated 0.2.0` (new exe), then a normal start with
  `on_restarted(0.2.0)`. ARP `DisplayVersion` became 0.2.0. `GithubSource` was not tested (no repo yet); it shares
  the same update code, only the feed differs.
- **Setup.exe over an existing install while the app runs** (what `winget upgrade` with `UpgradeBehavior: install`
  does): the running app was killed (exit code 1), **no uninstall or obsolete hook ran**, the old folder was renamed
  and then deleted, and a test file placed in the install folder was gone. Only the new version's install hook ran.
- **Silent uninstall while the app runs** (`Update.exe --uninstall --silent`, the `QuietUninstallString`): exit 0 in
  0.6 s, no dialog. Log: "Killing process ... vtrial.exe", then the `--veloapp-uninstall 0.2.0` hook ran (48 ms),
  then the folder, `%TEMP%\velopack_WakeshotTrial` and the registry key were removed.
- **Run with no arguments** (what winget validation does): `Update.exe` exited **1**, the root stub exited 0, the app
  exited 0. Two Velopack packages are merged in winget-pkgs anyway, so the `Update.exe` exit code did not block them;
  whether it needed an administrator to clear: unknown.

### x64 and ARM64

- Give each architecture its own channel (`win-x64`, `win-arm64`): "every unique os/rid has its own channel", and the
  default channel is just `win`. With the channel left unset at runtime, the app looks only at the channel it was
  built for. Verified: docs.velopack.io/packaging/channels.
- Setup.exe refuses to install on an unsupported CPU architecture (`CpuArchUnsupported`). Verified in `install.rs`.

## 2. winget

### Manifest for a Velopack Setup.exe

Two real Velopack apps in `microsoft/winget-pkgs` use the same shape. Verified by reading the merged manifests
(`Flowersauce.FSClicker` 1.4.0, `ptmrio.phrasevault` 4.0.0):

```yaml
InstallerType: exe
Scope: user
InstallModes: [silent, silentWithProgress]
InstallerSwitches:
  Silent: --silent
  SilentWithProgress: --silent
UpgradeBehavior: install
ProductCode: <packId>          # the HKCU Uninstall subkey name Velopack writes
AppsAndFeaturesEntries:
- DisplayName: <packTitle>
  Publisher: <packAuthors>
  ProductCode: <packId>
Installers:
- Architecture: x64
  InstallerUrl: https://github.com/<owner>/<repo>/releases/download/v<ver>/<file>-Setup.exe
  InstallerSha256: <hash>
```

- Velopack is not one of the installer types winget knows (`nullsoft`, `inno`, `burn`, `wix`, `msi`, `msix`), so the
  type is `exe` and the silent switch must be given. Verified: `doc/Validation.md` in winget-pkgs.
- ARM64: add a second `Installers` entry with `Architecture: arm64` and the ARM64 Setup.exe. Verified as a schema
  feature (manifests use `Architecture`); no Velopack ARM64 manifest was checked.
- Velopack keeps the ARP `DisplayVersion` current after self-updates (see section 1), so `winget list` and
  `winget upgrade` see the right version. Verified in Velopack source; not tested with winget.

### Rules and checks

- **Unsigned is allowed.** The repository policies (Microsoft Learn, updated 2026-08-30) have no signing rule.
  `Flowersauce.FSClicker` 1.4.0 is merged and its Setup.exe is unsigned: downloaded on the test laptop, SHA256 matched
  the manifest, `Get-AuthenticodeSignature` returned `NotSigned`. Verified.
- Validation runs the installer silently as a standard (non-elevated) user, then scans with Defender and several AV
  engines, checks PUA, checks URL SmartScreen reputation, and looks for the installed executable. Verified:
  `doc/Validation.md`.
- **The pipeline runs installed executables without arguments** and flags a non-zero exit code
  (`Validation-Executable-Error`, needs an administrator to clear). Verified: `doc/ValidationFailureGuide.md` and a
  moderator comment in winget-pkgs issue #323262 ("the validation pipelines ... run applications without
  arguments"). How long it waits for a GUI app that does not exit: unknown. What settles it: our first PR.
- `InstallerUrl` must be the ISV's own release location (policy 1.1.4). GitHub Releases qualifies. Verified.
- Known bug: WinGet 1.29.360 broke Velopack installs (sharing violation on the temp folder); fixed in 1.29.380
  (2026-09-18). Verified: winget-cli issue #6527.
- Tools that open the version-bump PR from CI: Komac (v2.17.1, 2026-10-15, active), `wingetcreate` (v1.12.13.0),
  `vedantmgoyal9/winget-releaser` (last release v2 in 2025-01). Verified: GitHub API. Wakeshot's `release.yml` uses
  `wingetcreate`.
- Whether a winget install avoids the SmartScreen prompt for an unsigned installer: reported yes by one project
  (PaperRoute-Tracker #216), not verified. What settles it: install our package by winget on a clean Windows Sandbox.

## 3. SignPath Foundation

### Requirements (signpath.org/terms, verified 2026-11-03)

- OSI license, no commercial dual license, no proprietary parts, no malware or PUP, actively maintained.
- "The project must already be released in the form that should be signed." So an unsigned release comes first.
- Functionality described on the download page.
- MFA for every team member on SignPath and on GitHub.
- Roles: authors, reviewers, approvers. "Every release needs manual approval for signing." A one-person project holds
  all roles (PaperRoute-Tracker does this, reported).
- Binaries built from source in a verifiable, automated build. For OSS, SignPath checks that every job before the
  signing request ran on GitHub-hosted runners. Verified: docs.signpath.io/trusted-build-systems/github.
- A "Code signing policy" on the home and download pages, with the line "Free code signing provided by SignPath.io,
  certificate by SignPath Foundation", the team roles, and a privacy statement.
- Upstream binaries may ship unsigned; SignPath "reserve[s] the right to require that signed packages only include
  signed program and library files in the future."
- **Reputation**: "For executable programs, we require a certain verifiable reputation" and "We cannot sign binaries
  based on source code that nobody knows."

### Publisher name

"The code signing certificate is issued to SignPath Foundation" and "SignPath Foundation is the publisher of the OSS
project." So Windows shows **SignPath Foundation**, not the author's name. Verified: signpath.org/terms.

### Application and wait

- Form (as described by an applicant, reported): repository URL, license, download URL, description. Follow-up by
  email. After approval: create the SignPath project, link the GitHub trusted build system, define the signing policy.
- **New projects get declined.** Reported cases from 2026:
  - PaperRoute-Tracker: applied 2026-10-30, declined 2026-10-31. SignPath asked for community adoption (stars,
    forks, contributors), independent articles, institutional backing, sustained activity; offered a paid plan.
  - wslkit/skrog (issue 2026-09-16): declined, program "is for projects with an established user base".
  - DiffViewer: applied 2026-05-30, issue still open on 2026-11-03.
  - Approvals reported to take from a few days to about two months.
- Conclusion: Wakeshot will very likely be declined at launch. Plan to ship unsigned and apply once it has real users.
  What settles it: apply after v1 with stars and downloads to show.

### GitHub Actions setup (docs.signpath.io, verified)

1. Upload the unsigned files with `actions/upload-artifact` (v4 or later; it makes a zip).
2. `signpath/github-action-submit-signing-request@v3` with `api-token` (secret), `organization-id`, `project-slug`,
   `signing-policy-slug`, `github-artifact-id`, `wait-for-completion: true`, `output-artifact-directory`.
3. The artifact configuration says what to sign inside the zip (`<zip-file>` with PE files; several files in one
   request are supported as a container). Exact syntax for our files: not verified.
4. A release-approval step in SignPath (manual approval per release, from the terms).

### Velopack plus SignPath: order of signing

Not verified; no public end-to-end example found. What we know:

- Velopack records each package hash in its release feed, so the `.nupkg` cannot be changed after `vpk pack`.
- So sign `wakeshot.exe` (and the CLI exe) **before** `vpk pack`, then sign `Setup.exe` **after** it. That is two
  SignPath requests per release, each needing approval. `vpk` itself signs Setup.exe last when given
  `--signTemplate`, which suggests signing Setup.exe after packing is safe. Not tested.
- Velopack's own `Update.exe` would stay unsigned unless SignPath allows signing an upstream binary with the
  Foundation certificate. Unknown. What settles it: ask SignPath during onboarding.

## 4. GitHub Actions runners (public repo)

Verified: docs.github.com (runners reference, limits, Actions billing), read 2026-11-03.

- "Use of the standard GitHub-hosted runners is free and unlimited on public repositories."
- Windows x64: `windows-latest`, `windows-2025`, `windows-2025-vs2026`, `windows-2022`. 4 CPU, 16 GB RAM, 14 GB SSD.
- Windows ARM64: `windows-11-arm`, `windows-11-vs2026-arm`. Same size. Not marked as preview on the page.
- Larger runners are billed even for public repos. Not needed.
- Limits: 6 h per job, 35 days per run, 20 concurrent jobs (Free plan), `GITHUB_TOKEN` 1,000 API requests per hour
  per repo.
- Storage: Free plan lists 500 MB artifact storage and 10 GB cache per repo. Whether public repos are exempt: not
  stated, not verified. It does not matter much: release files live in GitHub Releases, and CI artifacts are small and
  short-lived.
- We can cross-compile ARM64 on an x64 runner (a local ARM64 cross-build works) and use `windows-11-arm` only to run
  the ARM64 tests.

## 5. scoop

- Manifest: JSON with `version`, `description`, `homepage`, `license` required, plus `url` and `hash` (or per
  architecture under `architecture.64bit` / `architecture.arm64`), `bin`, `shortcuts`, `persist`, `checkver`,
  `autoupdate`. `installer` / `uninstaller` and `pre_uninstall` scripts exist. Verified: Scoop wiki "App Manifests".
- Scoop prefers portable apps (`persist` "highly preferred") and has a bot (Excavator, about every 4 hours) that
  bumps versions through `checkver` / `autoupdate`; version-bump-only PRs are refused. Verified: ScoopInstaller
  CONTRIBUTING.md.
- **Extras bucket** requires "Reasonably well-known and widely used (e.g. if it's a GitHub project, it should have at
  least 100 stars and/or 50 forks)". Verified: Extras `package-request.yml`. So at launch only our own bucket works.
- **Own bucket**: any public GitHub repo with a `bucket/` folder (ScoopInstaller/BucketTemplate); users run
  `scoop bucket add <name> <url>`. Verified as a pattern (template linked from CONTRIBUTING.md); not tried.
- Velopack apps already in Extras use the **portable zip** (`<packId>-win-Portable.zip`), with a shortcut to the root
  exe and `persist` for data. Example: `bucket/stranslate.json`. Verified.
- Risk for Wakeshot: a Scoop uninstall does not run the Velopack uninstall hook, and a Velopack portable app can
  self-update inside Scoop's folder. Scoop manifests can run a `pre_uninstall` script (for example a `wakeshot
  restore` command). Whether Scoop refuses to uninstall while the app runs: not verified.

## Sources

All read 2026-11-03.

- Velopack docs: https://docs.velopack.io/getting-started/rust, https://docs.velopack.io/integrating/hooks,
  https://docs.velopack.io/packaging/installer, https://docs.velopack.io/packaging/channels,
  https://docs.velopack.io/reference/cli/content/vpk-windows, https://docs.velopack.io/reference/cli/content/setup-windows,
  https://docs.velopack.io/reference/cli/content/update-windows
- Velopack source (branch `develop`): https://github.com/velopack/velopack, files `src/lib-rust/src/app.rs`,
  `src/lib-rust/src/constants.rs`, `src/lib-rust/src/sources/github.rs`, `src/bins/src/commands/install.rs`,
  `uninstall.rs`, `apply_windows_impl.rs`, `src/bins/src/windows/registry.rs`, `src/bins/src/windows/util.rs`,
  `src/bins/src/shared/util_windows.rs`
- Crate docs: https://docs.rs/velopack/1.2.161/velopack/
- winget: https://learn.microsoft.com/en-us/windows/package-manager/package/windows-package-manager-policies,
  https://github.com/microsoft/winget-pkgs/blob/master/doc/Validation.md,
  https://github.com/microsoft/winget-pkgs/blob/master/doc/ValidationFailureGuide.md,
  manifests `manifests/f/Flowersauce/FSClicker/1.4.0` and `manifests/p/ptmrio/phrasevault/4.0.0`,
  https://github.com/microsoft/winget-pkgs/issues/323262, https://github.com/microsoft/winget-cli/issues/6527
- SignPath: https://signpath.org/terms, https://docs.signpath.io/trusted-build-systems/github,
  https://docs.signpath.io/artifact-configuration/reference
- SignPath applicant reports: https://github.com/JUhalt/PaperRoute-Tracker/issues/216,
  https://github.com/wslkit/skrog/issues/358, https://github.com/geevensingh/DiffViewer/issues/17,
  https://zenn.dev/shm_7ec/articles/signpath-oss-code-signing
- GitHub: https://docs.github.com/en/actions/reference/runners/github-hosted-runners,
  https://docs.github.com/en/actions/reference/limits,
  https://docs.github.com/en/billing/concepts/product-billing/github-actions
- Scoop: https://github.com/ScoopInstaller/Scoop/wiki/App-Manifests,
  https://github.com/ScoopInstaller/.github/blob/main/.github/CONTRIBUTING.md,
  `ScoopInstaller/Extras/.github/ISSUE_TEMPLATE/package-request.yml`, `ScoopInstaller/Extras/bucket/stranslate.json`
