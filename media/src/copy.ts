// Every word the video shows, in one place. Drafts: `p5-text` and `p6-readme` own the final wording, so change it
// here when theirs changes. Each line says only what Wakeshot v1 does (`docs/product.md`).

export const COPY = {
  name: 'Wakeshot',
  tagline: 'Your PC stays awake while your AI agent works.',
  start: 'Claude Code starts work. Wakeshot keeps the PC awake.',
  lid: 'Close the lid: the work goes on, and the PC locks.',
  end: 'Work done. Wakeshot lets go.',
  endSmall: 'Lid still closed? Once nothing has happened for 5 minutes, the laptop goes to sleep, as Windows would have.',
  settings: 'Your own power settings are back.',
  settingsHeld: 'during the work',
  settingsOwn: 'yours, back',
  keepAwake: 'Keep awake on demand. No second tool.',
  speedTitle: 'Light and fast',
  speedLabels: {
    ram: 'MB of memory when idle',
    cpu: 'ms of CPU in 10 idle minutes',
    delay: 's from Claude starting to the hold',
  },
  speedNotMeasured: 'not measured yet',
  outroLine: 'Free and open source. Windows 11, x64 and ARM64.',
  /** The GitHub address until the winget package is live; then `winget install HaikAsatryan.Wakeshot`. */
  install: 'github.com/HaikAsatryan/wakeshot',
  draft: 'DRAFT: captures and speed numbers missing',
  standIn: 'stand-in',
  missing: 'no capture yet',
};
