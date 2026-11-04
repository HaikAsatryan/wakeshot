# Packaging

How Wakeshot becomes a Velopack release, and how to cut one.

| File | What it does |
|---|---|
| `pack.ps1` | Packs one CPU: copies `wakeshot-tray.exe`, `wakeshot.exe` and the notice icon `assets/wakeshot-notice.png` into a staging folder and runs `vpk pack`. Writes `Setup.exe`, the full package, a delta when an earlier release is given, the portable zip and the release index. |
| `smoke-test.ps1` | Installs a `Setup.exe` silently, checks what it left, uninstalls silently and checks that it is all gone. |
| `e2e.ps1` | Installs, updates and uninstalls for real on a PC, with a journaled hold on a throwaway plan copy at each step, and checks that the PC is left as it was. Run it unelevated; `Get-Help ./packaging/e2e.ps1` says how. |
| `../.github/workflows/release.yml` | On a `v*` tag: build, test, pack and smoke-test both CPUs, then publish one GitHub release. By hand: the same as a dry run that uploads nothing. |
| `winget/` | The winget manifest templates and the script that fills them from a release (`winget/README.md`). |

Release values (`docs/phases/06-ship.md`, "Package and install values"): pack id `HaikAsatryan.Wakeshot`, title
`Wakeshot`, authors `Haik Asatryan`, main exe `wakeshot-tray.exe`, channels `win-x64` and `win-arm64`, a Start menu
shortcut only. Release files per CPU, with `<cpu>` `x64` or `arm64`:

- `HaikAsatryan.Wakeshot-win-<cpu>-Setup.exe` (no version in the name)
- `HaikAsatryan.Wakeshot-<version>-win-<cpu>-full.nupkg`, and `-delta.nupkg` from the second release on
- `HaikAsatryan.Wakeshot-win-<cpu>-Portable.zip`
- `releases.win-<cpu>.json`, the index the installed app reads to find updates

## Pack on this PC

Needs `vpk` 1.2.161 (`dotnet tool install -g vpk --version 1.2.161`), the same version as the `velopack` crate.

```powershell
cargo build --release --locked --target x86_64-pc-windows-msvc -p wakeshot-tray -p wakeshot-cli
./packaging/pack.ps1 -Version 0.1.0 -Cpu x64 -ExeDir target/x86_64-pc-windows-msvc/release -OutDir <new folder> -Icon assets/wakeshot.ico
```

- `-OutDir` must be new or empty, so an old file never passes for a new one.
- `-PreviousDir <folder>` takes earlier full packages for the delta (`vpk download github` writes such a folder).
- The script checks each exe is built for `-Cpu`, refuses a version with build metadata (`+...`), and refuses to run
  while any `VPK_*` variable is set: `vpk` reads every one as a pack option.
- It refuses to pack without the notice icon: the install and updated hooks point the notice header at it, and
  without it every notice header shows a plain square. `-NoticeIcon <file>` takes another file; it still ships as
  `wakeshot-notice.png`.
- Never set `RUSTFLAGS` for the build: it drops the static CRT.

Do not run `smoke-test.ps1` on a PC where Wakeshot is or was installed. It refuses when it finds any trace (the
install folder, the data folder, the uninstall key, the Run or RunOnce value, the Path entry, the shortcut): the
uninstall deletes the data folder, and a journal in it would make the hooks restore power settings. CI runners are
clean; `p6-install-e2e` covers this laptop.

## Cut a release

1. Pick the version. `X.Y.Z` is a normal release. `X.Y.Z-rc.N` (any SemVer pre-release part) becomes a GitHub
   pre-release: Velopack's GitHub feed skips it unless the app asks for pre-releases, and it never goes to winget.
   Velopack writes only `X.Y.Z` to Apps and Features, so an installed `0.1.0-rc.1` shows there as `0.1.0`.
2. Set `[workspace.package] version` in the root `Cargo.toml`, then run `cargo update --workspace` so `Cargo.lock`
   follows (every build uses `--locked` and fails on a stale lock). Commit both and push to `main`. Wait for CI.
3. Optional dry run: GitHub, Actions, Release, Run workflow, on `main`. It builds, packs and smoke-tests both CPUs and
   uploads nothing. The packs are kept 7 days as run artifacts. It packs against the last release for the delta, so
   it fails at Pack while the `Cargo.toml` version is not above the last release's.
4. Tag that commit and push the tag:

   ```powershell
   git tag v0.1.0
   git push origin v0.1.0
   ```

   The run fails at once when the tag is not `v` plus the `Cargo.toml` version.
5. After the run:
   - The release page shows both `Setup.exe` files, both portable zips, both full packages (and deltas), and
     `releases.win-x64.json` and `releases.win-arm64.json`. It is published, not a draft, and marked pre-release only
     for an `-rc` version.
   - The release text is empty: write the notes on GitHub, or add `--releaseNotes` to `pack.ps1` once there is a
     notes file per version.
   - For a normal release with `WINGET_SUBMIT` on, the winget job opens the winget-pkgs pull request
     (`winget/README.md`).

If the run fails after the x64 upload, a draft release with that tag is left. Delete the draft (not the tag) on the
releases page, then re-run the failed jobs: an upload without `--merge` refuses a release that already exists.

## What the smoke test checks

After `Setup.exe --silent` (what winget runs): exit 0; `Update.exe`, `Wakeshot.exe`, `current\wakeshot-tray.exe`,
`current\wakeshot.exe`, `current\sq.version`; the Start menu shortcut points at `current\wakeshot-tray.exe`; the
uninstall key has `DisplayName` `Wakeshot`, `Publisher` `Haik Asatryan`, `DisplayVersion` equal to the version's three
numbers (winget compares it with the manifest), and a `QuietUninstallString`; the Run value `Wakeshot` is the quoted
tray exe; the user Path has the `current` folder; the install hook's start line is in `%LocalAppData%\Wakeshot\logs\wakeshot.log`;
`wakeshot.exe` with no arguments exits 0 (the winget pipeline runs installed exes that way).

After the `QuietUninstallString` (what Apps and Features and winget run): exit 0; the install folder, the uninstall
key, the Run and RunOnce values, the Path entry, the shortcut and the data folder are gone, and the user Path has its
old type and bytes.

The install hook's log line is the only proof that the tray runs Velopack's hooks: `vpk pack` does not check it. So a
dry run fails until the tray calls `VelopackApp` (`p6-velopack-dependency`).

## Signing

None in v1 (decision 44; no paid certificate). `release.yml` marks the two places SignPath goes later: the exes before
`vpk pack` (the release feed records each package's hash), and `Setup.exe` after it.
