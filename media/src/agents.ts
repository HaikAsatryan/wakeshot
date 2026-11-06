// The AI agents Wakeshot supports, in one place. The video's name chips (shot 2) and the agent lists in `README.md`
// and `site/index.html` (written by `scripts/agent-lists.mjs`) all come from here, so adding or dropping an agent is
// one line. An agent shows only once its live test passed: a pending or failed test never puts a name on screen or in
// the docs (`docs/tasks/p6-agents-video-readme.md`). A to Z by name, so no agent comes first by our choice. Plain
// types only: Node runs this file too.

export type Agent = {
  /** The name the README and the site use. */
  name: string;
  /** The chip it shares in the video with agents of the same `chip`, once all of them passed. */
  chip?: string;
  /** `passed` once its live card passed; until then `pending`. */
  live: 'passed' | 'pending';
  /** Its live card in `docs/tasks/`, for the record. */
  card: string;
};

export const AGENTS: Agent[] = [
  {name: 'Antigravity', live: 'pending', card: 'p4-live-antigravity'},
  {name: 'Claude Code', live: 'passed', card: 'p4-live-claude'},
  {name: 'Codex CLI', live: 'passed', card: 'p4-live-codex'},
  {name: 'Copilot CLI', chip: 'Copilot', live: 'pending', card: 'p4-live-copilot'},
  {name: 'Copilot in VS Code', chip: 'Copilot', live: 'pending', card: 'p4-live-vscode'},
  {name: 'Cursor', live: 'pending', card: 'p4-live-cursor'},
  {name: 'OpenCode', live: 'pending', card: 'p4-live-opencode'},
];

/** The agents the docs and the video name. */
export const SUPPORTED: Agent[] = AGENTS.filter((a) => a.live === 'passed');

/**
 * The video's chips: one shared chip once every agent that shares it passed, else each agent that passed under its
 * own name, so a chip never claims more than was tested.
 */
export function chips(): string[] {
  const out: string[] = [];
  for (const a of SUPPORTED) {
    const all = a.chip !== undefined && AGENTS.filter((b) => b.chip === a.chip).every((b) => b.live === 'passed');
    const name = all ? (a.chip as string) : a.name;
    if (!out.includes(name)) out.push(name);
  }
  return out;
}

/** "A", "A and B", "A, B and C". */
export function listText(names: string[]): string {
  return names.length < 2 ? names.join('') : `${names.slice(0, -1).join(', ')} and ${names[names.length - 1]}`;
}
