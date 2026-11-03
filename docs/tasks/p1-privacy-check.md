# Check every file for private content before the first public push

Phase: 1. Status: dropped (2026-11-03, `p1-design-review-fixes`). Haik's answer 1 (15:29) makes the repo fully
public; only secrets and build output stay out. The orchestrator's secret scan on the staged files replaces this card
(`docs/phases/01-design-skeleton.md`, "Orchestrator steps"). Kept for the record only; do not run it.
Agent: `wakeshot-reviewer` (never fixes).
Runs after: `p1-repo-files`, `p1-cargo-workspace`, `p1-ci`, and orchestrator step 3 in
`docs/phases/01-design-skeleton.md` (the list of files git would add). Also after any other card whose files go in
the first push. Parallel with: any card that changes no file on the list. The reviewer writes only
`docs/reviews/p1-privacy.md`.

## Goal

The repo `HaikAsatryan/wakeshot` goes public with its first push. Before that, read every file git would add and list
everything that should not be public.

## Read first

- `CLAUDE.md`; `docs/plan.md` "How we work".
- `docs/phases/01-design-skeleton.md`: Haik's answer to Question 1 (which folders go public) and "Orchestrator steps".
- The file list from the orchestrator (its path is in the brief; it lives outside the repo).
- Then every file on the list.

## Deliverables

`docs/reviews/p1-privacy.md`, with one row per finding: file, line or region, class (below), and a verdict: **must
fix** (clearly private), **Haik decides** (a judgment call, with your default), or **fine** (looked private, is not,
and why). A summary at the top: files checked, findings per verdict.

Classes to look for:

1. Personal paths and names: `C:\Users\haika`, the user name in any form, other people's names, email addresses.
   Haik's own name as author is fine.
2. Machine and account IDs: computer name (read `$env:COMPUTERNAME`), `pidDomain`, user SIDs, logon SIDs, audio
   endpoint IDs, device and monitor serials, MAC or IP addresses, Wi-Fi names, volume or disk IDs.
3. Screenshots: every image viewed in full. Look for window titles, chat or mail text, other apps' icons that reveal
   private use, notification text, file names, wallpaper with personal content.
4. Anything from Haik's own sessions: transcript lines, prompt text, session ids, project or customer names, paths of
   his other repos, his Claude Code settings or memory, `.key` file names or content.
5. Secrets of any kind: tokens, keys, passwords, connection strings (`ghp_`, `sk-`, `token`, `password`, `secret`).
6. Raw tool logs that slipped past `.gitignore`.

Method: one grep pass per class over all text files (write the patterns into the review so `p1-privacy-fixes` can
re-run them), then a read of each fixture and log-like file, and a look at each image. Binary files you cannot read:
list them with why they are fine or what to do.

## Acceptance checks

- Every file on the orchestrator's list appears in the review as checked (a table or a count per folder that adds up
  to the list's length).
- Every finding names a file and a location, and has a verdict.
- The grep patterns are written out in the review.

## Out of scope

- Fixing anything, editing any file except `docs/reviews/p1-privacy.md`. Git of any kind.
- Files not on the list (they stay local).

## Questions for Haik

Each with the reviewer's default (they also go in the review under "Haik decides").

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-03: card written by `p1-detailed-design`.

## Result

What was done, files changed, checks and outcomes, open items.
