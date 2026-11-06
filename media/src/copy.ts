// Every word the video shows, in one place. Drafts: `p5-text` and `p6-readme` own the final wording, so change it
// here when theirs changes. Each line says only what Wakeshot v1 does (`docs/product.md`). `*word*` marks a line's
// one emphasis word. UI words (menu, tooltip) are never here: they come only from the captures.

export const COPY = {
  name: 'Wakeshot',
  hook: ['Agent *working*.', 'PC sleeping.'],
  tagline: 'keeps it *awake* while agents work.',
  /** The tray shot: Claude Code starts and the icon turns holding; the work ends and it lets go. */
  tray: ['Claude Code starts: it turns *green*.', 'Done: it lets *go*.'],
  /** One line for the lid shot (round 4 review, M1); the lock badge shows the lock. */
  lid: 'Close the lid: work goes *on*.',
  settings: 'Work done. Your settings, *back*.',
  /** After a time cut: no clock, no countdown, no claim that it sleeps at once (Haik, 2026-11-06 00:56). */
  sleeps: 'Later, it sleeps as *usual*.',
  menu: ['Keep awake on *demand*.', 'No second *tool*.'],
  endLine: 'Free and open source. Windows 11, x64 and ARM64.',
  /** The end card's speed line: `{ram}` and `{cpu}` come from `data/speed.json`. */
  speedLine: 'Idle: {ram} MB of memory, {cpu} ms of CPU in 10 minutes.',
  /** The GitHub address until the winget package is live; then `winget install HaikAsatryan.Wakeshot`. */
  install: 'github.com/HaikAsatryan/wakeshot',

  /** Drawn parts that are the video's own, not the app's. */
  sleepingChip: 'Sleeping',
  measured: (build: string, date: string) => `Measured on Wakeshot ${build}, ${date}, x64.`,

  /** The illustrated terminal: not a copy of Claude Code's screen. */
  terminal: {
    title: 'Terminal',
    path: 'PS C:\\code\\shop>',
    command: 'claude',
    prompt: 'migrate billing to API v2 and run all tests',
    /** The tray shot's shorter task: its terminal shows the prompt and one status line, large enough to read. */
    trayPrompt: 'fix the failing tests',
    working: 'Working',
    done: 'Done. All 128 tests pass.',
    /** The step the work chip shows while the lid is shut, then `done`. */
    behindLid: ['run', 'npm test'],
    lines: [
      ['read', 'src/billing/client.ts'],
      ['read', 'src/billing/invoice.ts'],
      ['edit', 'src/billing/client.ts', '+42 -17'],
      ['edit', 'src/billing/webhooks.ts', '+18 -9'],
      ['read', 'docs/api-v2.md'],
      ['run', 'npm test -- billing'],
      ['pass', '128 tests'],
      ['edit', 'src/billing/refunds.ts', '+11 -4'],
      ['run', 'npm test'],
      ['edit', 'src/billing/tax.ts', '+7 -3'],
      ['read', 'src/billing/currency.ts'],
      ['run', 'npm run lint'],
    ] as string[][],
  },

  draft: 'DRAFT',
  standIn: 'stand-in',
  speedStandIn: 'stand-in numbers: not measured yet',
};
