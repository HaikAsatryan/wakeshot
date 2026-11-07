// The AI agents Wakeshot supports, in one place. The video's agent tiles (the agents shot) and the agent lists in
// `README.md` and `site/index.html` (written by `scripts/agent-lists.mjs`) all come from here, so adding or dropping an
// agent is one line, and the tiles lay themselves out for any count. An agent shows only once its live test passed: a
// pending or failed test never puts a name on screen or in the docs. A to Z by name, so no agent comes first by our
// choice. Plain types only: Node runs this file too.

export type Agent = {
  /** The name the README and the site use. */
  name: string;
  /** The tile it shares in the video with agents of the same `chip`, once all of them passed. */
  chip?: string;
  /**
   * `passed` once its live test passed; `partly` when the live test ran but a hold for a working turn was not seen,
   * which counts as not passed; until then `pending`.
   */
  live: 'passed' | 'partly' | 'pending';
  /** The internal name of the live test that decides `live`, for the record. */
  card: string;
};

// The v1 list is final: Cursor and Antigravity left v1, so they are not listed at all.
export const AGENTS: Agent[] = [
  {name: 'Claude Code', live: 'passed', card: 'p4-live-claude'},
  {name: 'Codex CLI', live: 'passed', card: 'p4-live-codex'},
  {name: 'Copilot CLI', chip: 'Copilot', live: 'passed', card: 'p4-live-copilot'},
  // Passed for the Copilot harness chats, every case exact; VS Code's Local chats left v1.
  {name: 'Copilot in VS Code', chip: 'Copilot', live: 'passed', card: 'p4-live-vscode'},
  // Tested live with 2.x (2.2.5); a 1.x database was not tested live.
  {name: 'OpenCode', live: 'passed', card: 'p4-live-opencode'},
];

/** The agents the docs and the video name. */
export const SUPPORTED: Agent[] = AGENTS.filter((a) => a.live === 'passed');

/**
 * One tile of the video's agents shot: the name only, in our type. No owner's logo: of the owners only GitHub's
 * rules allow one for "works with", and even that page also asks for prior written permission
 * (`docs/research/promo-video.md`, "Agent logos").
 */
export type Tile = {name: string};

/**
 * The video's tiles: one shared tile once every agent that shares it passed, else each agent that passed under its
 * own name, so a tile never claims more than was tested.
 */
export function tiles(list: Agent[] = AGENTS): Tile[] {
  const out: Tile[] = [];
  for (const a of list.filter((b) => b.live === 'passed')) {
    const all = a.chip !== undefined && list.filter((b) => b.chip === a.chip).every((b) => b.live === 'passed');
    const name = all ? (a.chip as string) : a.name;
    if (!out.some((t) => t.name === name)) out.push({name});
  }
  return out;
}

/** The tiles' names. */
export const chips = (list: Agent[] = AGENTS): string[] => tiles(list).map((t) => t.name);

/**
 * The same tiles for a list of names, as if those agents had passed: only for stills of a layout (the `preview` prop
 * of the video). The builds never pass it, so the video itself always shows `tiles()`.
 */
export function previewTiles(names: string[]): Tile[] {
  return tiles(AGENTS.map((a) => ({...a, live: names.includes(a.name) ? 'passed' : 'pending'})));
}

/** "A", "A and B", "A, B and C". */
export function listText(names: string[]): string {
  return names.length < 2 ? names.join('') : `${names.slice(0, -1).join(', ')} and ${names[names.length - 1]}`;
}
