# Verify: agent hooks design, security review majors

Check of the fixes for the three majors of `reviews/p4-hooks-design-security.md`, in `docs/agent-hooks.md`,
`docs/architecture.md` section 12 and the "Review fixes" table of `docs/tasks/p4-hooks-design.md` (2026-11-06).
Minors not re-checked. Docs and `process.rs` read; nothing installed or run; no git.

Result: major 1 Fixed, major 2 Fixed, major 3 Partly fixed. No new blocker; no new major. Three new minors below.

## Major 1: an elevated hook writes and deletes in a Medium-controlled folder

**Fixed.** `agent-hooks.md:147-162` (steps 4 and 5), `:249-250` (tray), `:271-273` (uninstall), contract 6
(`:393-396`), live test 9, risk row 19.

- All file work in `agents\` and in agent folders runs at Medium only. An elevated hook passes its checked values to a
  Medium copy through `spawn_outside_tree`; the copy checks its own token and writes nothing if it is not Medium (the
  fallback of `spawn_outside_tree` is a normal, still elevated child, `process.rs:252-273`, so this check is needed
  and is there). An elevated tray skips; an elevated uninstall runs `hook <agent> remove` the same way.
- The prune now deletes only files that parse as version 1 and name a gone process, plus its own temp names. At
  Medium a junction or hard link gains nothing, so the old attack is gone.

New minors from this fix:

- **The copy must re-check its `--checked` values.** The doc says the values are "from fixed character sets" and that
  the copy re-checks its token, but not that it re-checks the values. `hook cursor beforeSubmitPrompt --checked
  ..\..\x ...` can be run by any process of the user. No privilege gain at Medium, but without the check the id rule
  (UUID, lowercased) does not hold for this path. Fix: the copy runs the same step 2 checks on its arguments and
  writes nothing if one fails.
- **A copy slower than the 2 s wait can land after the turn's `stop`.** The elevated `beforeSubmitPrompt` returns
  after 2 s even if its copy has not written yet; the turn may end and the `stop` copy run first, then the late copy
  writes "working", which holds until the agent process ends. The handle from `spawn_outside_tree` has only
  `SYNCHRONIZE` and `PROCESS_QUERY_LIMITED_INFORMATION` (`process.rs:277-278`), so it cannot kill the copy. Fix:
  pass a deadline (the elevated hook's start plus 2 s, as a FILETIME) among the checked values; the copy drops a
  "working" write past it.
- **Contract 6 claims too much.** It says `CREATE_NEW` "also closes the same gap for `wakeshot run` in an elevated
  shell". It closes the hard-link variant only. A junction plus an object-manager link still redirects the temp file's
  create, and now also the new delete-first step (a delete of whatever file the link names). Not new in class (the
  old `CREATE_ALWAYS` overwrite was as bad) and not part of the hooks any more, which run at Medium. Fix: drop the
  claim, and note that an elevated `wakeshot run` (and the uninstall's own journal and data-folder work under an
  elevated `winget uninstall`; not checked) keep this gap until they get the same Medium rule.

## Major 2: the uninstall and the rewrite trust the marker

**Fixed.** `agent-hooks.md:63-70` (fixed file set, fixed keys), `:242-248` (ours, unchanged, fixed names, reparse
check on every part), `:274-280` (uninstall), risk row 20.

- The file set is fixed in code; the marker has fixed keys and hashes only; Wakeshot never reads a path from it.
- Any extra key, file or folder makes the folder "changed by someone", so a dropped `rules\` or `mcp.json` is never
  called Wakeshot's.
- Deletes go by the fixed names, never `local\` or `plugins\`, never recursive, never through a link; an elevated
  uninstall hands the work to a Medium copy. The traversal and the elevated delete are gone.
- A folder that cannot be emptied is renamed to `.wakeshot-removed`. If that name exists already (a second uninstall),
  the rename fails and one log line is all; the README line covers it. Acceptable.

## Major 3: prompts on a command line, so in audit logs

**Partly fixed.** `agent-hooks.md:321-336` (Privacy, Gate), `:27-29` and `:365-367` (user view and README in the main
text), live test 0, card Question 4 (`p4-hooks-design.md:85-89`), risk row 7.

What holds: the gate (a hook only for an agent form known to use a temp file or stdin), the check as the first step
of the Cursor and Antigravity build cards, the README line in the main text, the Question 4 default "do not ship".

What does not hold: the gate is written per app, but Cursor's editor and CLI load the same folder,
`~\.cursor\plugins\local\wakeshot\`. "The editor hook does not ship" (`:330-331`) cannot be done while the plugin
ships for the CLI: the editor loads the same `hooks.json` and starts PowerShell with the prompt on its command line
before Wakeshot's exe even runs. Question 4's default text ("The Cursor editor is then not covered ... its CLI
already uses a temp file") reads as "ship for the CLI only". Failure: the check finds the command line, the build
follows the default, ships the plugin for the CLI, and every editor prompt goes into audit logs, which is the
original major.

Fix (one line in the Gate and in Question 4): if the editor uses the command line, Wakeshot adds no Cursor plugin at
all, for either app, since both load one folder. That costs little: the CLI's interactive loop does not fire
plugin-only prompt and stop hooks anyway (Question 1).

Also, minor: the transport is a runtime option, not fixed by version. In the CLI code `commandHookPayloadTransport`
is a parameter that callers pass in (`3026.index.js`, `4110.index.js`, `697.index.js` from a function; `2855.index.js`
from a variable), so a later build, or a setting, could switch to the command-line form. Question 4's "re-checks each
Cursor version" covers versions Wakeshot claims, not newer ones the user updates to. Optional guard (change it when a
Cursor version is seen using the command line): the hook reads its parent PowerShell's command line; if it holds the
here-string (`@'`), the hook records that agent as removed and the tray removes the plugin, one log line. This limits
the leak to one prompt.

## May the next steps start

- `p4-contracts-r3` may start now: none of the open points changes a contract item (the deadline fits in
  `values: Vec<String>`).
- The build may start once the one-line fix for major 3 is in `agent-hooks.md` (Gate) and Question 4; the three new
  minors can go into the build cards.
