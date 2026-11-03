# Confirm the free ship path

Phase: 0. Status: done. Needs Haik: no.
Runs after: none (the Velopack trial needs `p0-toolchain`). Parallel with: any card.

## Goal

Confirm that installer, updates, winget, signing and CI all work for free for a Rust app.

## Read first

- `CLAUDE.md`
- `docs/product.md`: "Distribution"

## Questions to answer

1. Velopack with a Rust app: `vpk pack`, per-user install path, the `--veloapp-uninstall` hook in Rust, silent
   install flag, updates from GitHub Releases. Does `vpk` run on the installed .NET 10 SDK?
2. winget: manifest for a Velopack `Setup.exe` (installer type, silent switches, per-user scope). Any winget-pkgs rule
   that blocks it or an unsigned installer?
3. SignPath Foundation: application steps, requirements for a new project, typical wait, GitHub Actions setup. Does
   the publisher name shown to users read "SignPath Foundation"?
4. GitHub Actions for a public repo: free Windows x64 and ARM64 runners? Limits?
5. scoop: what a manifest needs; own bucket vs extras.

## Deliverables

- `docs/research/distribution.md` with sources and verified or unverified marks.
- A short summary in `docs/research.md`.
- If cheap: a trial `vpk pack` of the hello-world from `p0-toolchain` (installing `vpk` as a .NET tool is allowed).
- An "Implications for the design" list in Result.

## Acceptance checks

- Every question has a sourced answer, or "unknown" with what would settle it.

## Out of scope

Creating accounts or applying to SignPath (Haik's call, later).

## Questions for Haik

## Found outside scope

- `docs/architecture.md` "Files" puts all data in `%LOCALAPPDATA%\Wakeshot\`. Velopack installs to
  `%LocalAppData%\{packId}`, deletes that whole folder on uninstall, and wipes it when Setup.exe runs over an existing
  install (tested). If the pack id is `Wakeshot`, the journal and config live inside the install folder and get lost.
  Not changed here: pick a different pack id or data folder in the Phase 1 design.
- The first `dotnet` command on this laptop ran the .NET first-run step and created an ASP.NET Core HTTPS dev
  certificate in the user store (not trusted). Harmless; left as is.

## Log

- 2026-11-03: card written.
- 2026-11-03: worker started. Toolchain card runs in parallel; Rust not installed yet, so the Velopack trial waits.
  Doing the web research first.
- 2026-11-03: Velopack facts done (source read plus docs), written to `docs/research/distribution.md`. `vpk`
  1.2.161 installed as a global .NET tool and runs on .NET 10. winget: an unsigned Velopack Setup.exe is in
  winget-pkgs (Flowersauce.FSClicker 1.4.0, hash matched, `NotSigned`). Orchestrator says toolchain is ready, so the
  trial will run after the research.
- 2026-11-03: research for all five questions written to `docs/research/distribution.md`. Starting the Velopack
  trial in the scratch folder (own throwaway crate, pack id `WakeshotTrial`, no shortcuts, no power settings).
- 2026-11-03: trial done: pack, silent install, update with hooks, reinstall over a running app, silent uninstall
  while running, no-arg exit codes. All hooks reach Rust. Install folder, registry key and temp folder removed after.
  Results in `docs/research/distribution.md`.
- 2026-11-03: summary added to `docs/research.md` ("Distribution"). Acceptance check run. Card done.

## Result

Done. The free ship path works for a Rust app, with one big caveat: signing will likely wait until Wakeshot has
users.

What was done:

- Answered all five questions from official docs, Velopack source, real winget and Scoop manifests, and applicant
  reports. Each fact is marked verified, reported or not verified.
- Velopack trial on this laptop (throwaway crate in the scratch folder, pack id `WakeshotTrial`): `vpk pack`, silent
  install, update from a local feed, Setup.exe over a running copy, silent uninstall over a running copy, no-arg exit
  codes. Everything it created was removed (install folder, registry key, temp folder; build outputs deleted).
  Kept: the trial source and its event log in
  `C:\Users\haika\AppData\Local\Temp\claude\C--Users-haika-code-wakeshot\1828f2de-68d5-4148-9124-8321986ceb05\scratchpad\vtrial`
  and `...\scratchpad\vtrial-markers`.
- `vpk` 1.2.161 is now installed as a global .NET tool (allowed by the card). Kept for Phase 6.

Files changed: `docs/research/distribution.md` (new), `docs/research.md` (new section "Distribution"), this card.

Acceptance check: every question has a sourced answer, or "unknown" with what would settle it. Pass. Open unknowns:

- How long winget validation waits for a GUI app that does not exit. Settles with our first winget PR.
- Whether a winget install skips the SmartScreen prompt for an unsigned installer. Settles with a Windows Sandbox
  test of our package.
- `GithubSource` end to end (needs a public repo with a release). Same update code as the tested local feed.
- SignPath: exact artifact configuration, and whether it will sign Velopack's `Update.exe`. Settles at onboarding.
- Whether Scoop refuses to uninstall a running app.

Implications for the design:

1. **Data folder must be outside the install folder.** See "Found outside scope". Phase 1 decides the pack id and
   data path.
2. **Velopack kills Wakeshot (and the guard, same exe, same folder) with no warning** before uninstall, before an
   update swap, when Setup.exe runs over an existing install, and after every hook. So restore cannot depend on a
   clean shutdown. The journal plus "restore on next start" are required, not optional.
3. **Setup.exe over an existing install runs no uninstall or obsolete hook** (this is what `winget upgrade` with
   `UpgradeBehavior: install` does). Only the new version's `--veloapp-install` runs. So the install hook must also
   restore from the journal when the last run left settings overridden. Same for `--veloapp-updated`.
4. **Uninstall hook**: runs after the app was killed, 30 s limit, no UI. It restores from the journal, removes
   start-at-login and any scheduled task, and must work with no running app.
5. **Silent install does not start the app**, and a hook cannot start it (Velopack kills what a hook leaves running).
   So winget users start Wakeshot from the Start menu once, or start-at-login is registered by the install hook and
   the app first runs at next sign-in. Phase 6 decision.
6. **`wakeshot.exe` with no arguments** is what winget validation runs. Add `--version` that exits 0, and expect a
   possible administrator review for `Update.exe` exiting 1.
7. **Two exes**: Velopack has one main exe and adds nothing to PATH (not found). The CLI needs its own way onto PATH
   (for example the install hook writes the user PATH, the uninstall hook removes it). Phase 1 design.
8. **One channel per architecture** (`win-x64`, `win-arm64`) and always `vpk pack -r`. The x86 default is wrong.
9. **Update checks**: unauthenticated GitHub API, 60 requests per hour per IP. Check every few hours, not every
   minute.
10. **Size**: Setup.exe about 10.7 MB, mostly Velopack's `Update.exe`. Do not pitch "tiny installer".
11. **Signing**: plan v1 unsigned. Apply to SignPath once there are stars and downloads; it needs a code signing
    policy page, a privacy statement, MFA, GitHub-hosted builds, and one manual approval per signing request. The
    publisher will read "SignPath Foundation". Sign the exes before `vpk pack` and Setup.exe after it.
12. **CI**: free `windows-latest` and `windows-11-arm` runners cover build and ARM64 tests. Every job before signing
    must run on GitHub-hosted runners.
13. **Scoop**: own bucket only (Extras needs about 100 stars). A Scoop uninstall skips the Velopack hook, so it needs
    a `pre_uninstall` restore command and self-update turned off. Low priority, matches "scoop later if cheap".
