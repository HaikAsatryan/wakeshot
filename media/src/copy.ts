// Every word the video shows, in one place. Drafts: `p5-text` and `p6-readme` own the final wording, so change it
// here when theirs changes. Each line says only what Wakeshot v1 does (`docs/product.md`). `*word*` marks a line's
// one emphasis word. UI words (menu, tooltip) are never here: they come only from the captures.

export const COPY = {
  name: 'Wakeshot',
  /** The pain: the laptop sleeps under an agent at work. Only "working" takes the green, which means awake. */
  hook: ['Agent *working*.', 'Laptop sleeping.'],
  /** Haik's line, 2026-11-06. */
  question: 'Tired of sleepy laptops?',
  tagline: 'keeps it *awake* while agents work.',
  /**
   * Over the agent tiles, which come from `agents.ts`: only agents whose live test passed. It says what Wakeshot
   * does, not "works with": OpenAI asks non-partners not to write "worked with" (`docs/research/promo-video.md`,
   * "Agent logos"), and the line claims no tie to any owner.
   */
  agents: 'Stays *awake* for these agents.',
  /**
   * The example task under each tile's name: an illustration of work, not a claim about the agent. Keyed by the
   * tile's name; an agent with no line here gets `taskFallback`, so the list can grow with no new copy.
   */
  tasks: {
    Antigravity: 'run the e2e tests',
    'Claude Code': 'fix the failing tests',
    'Codex CLI': 'refactor the auth flow',
    Copilot: 'write the API docs',
    'Copilot CLI': 'write the API docs',
    'Copilot in VS Code': 'write the API docs',
    Cursor: 'build the settings page',
    OpenCode: 'migrate the database',
  } as Record<string, string>,
  taskFallback: 'work on a long task',
  /**
   * The tray shot: the agent starts and the icon turns holding; the work ends and it lets go. Claude Code is the
   * example on screen, the words say any agent (Haik, 2026-11-06).
   */
  tray: ['Your agent starts: it turns *green*.', 'Done: it lets *go*.'],
  /** One line for the lid shot (round 4 review, M1); the lock badge shows the lock. */
  lid: 'Close the lid: work goes *on*.',
  settings: 'Work done. Your settings, *back*.',
  /** After a time cut: no clock, no countdown, no claim that it sleeps at once (Haik, 2026-11-06 00:56). */
  sleeps: 'Later, it sleeps as *usual*.',
  endLine: 'Free and open source. Windows 11, x64 and ARM64.',
  /** The end card's speed line: `{ram}` and `{cpu}` come from `data/speed.json`. */
  speedLine: 'Idle: {ram} MB of memory, {cpu} ms of CPU in 10 minutes.',
  /** The GitHub address until the winget package is live; then `winget install HaikAsatryan.Wakeshot`. */
  install: 'github.com/HaikAsatryan/wakeshot',

  /** Drawn parts that are the video's own, not the app's. */
  sleepingChip: 'Sleeping',
  measured: (build: string, date: string) => `Measured on Wakeshot ${build}, ${date}, x64.`,
  /** The end card's small print, after the measured line: Google asks for a trademark line where its names show. */
  trademarks: 'Agent names are trademarks of their owners; Wakeshot is not affiliated with them.',

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
