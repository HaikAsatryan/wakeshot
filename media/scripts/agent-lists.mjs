// Writes the list of supported AI agents from `src/agents.ts` into `README.md` and `site/index.html`, between
// `<!-- agents -->` and `<!-- /agents -->`, so the video, the README and the page never name different agents.
// `--check` writes nothing and fails when a list differs, or when the README's "When each hold counts" has a
// paragraph for an agent that is not supported, or none for one that is (each starts with `**<name>.**`).
//   node scripts/agent-lists.mjs [--check]
import {readFileSync, writeFileSync} from 'node:fs';
import {dirname, join} from 'node:path';
import {fileURLToPath} from 'node:url';
import {AGENTS, SUPPORTED, listText} from '../src/agents.ts';

const repo = join(dirname(fileURLToPath(import.meta.url)), '..', '..');
const check = process.argv.includes('--check');
const FILES = ['README.md', 'site/index.html'];
const MARK = /<!-- agents -->(.*?)<!-- \/agents -->/g;
const list = listText(SUPPORTED.map((a) => a.name));

let failed = false;
const fail = (text) => {
  failed = true;
  console.log(`FAIL ${text}`);
};

for (const file of FILES) {
  const path = join(repo, file);
  const text = readFileSync(path, 'utf8');
  const found = [...text.matchAll(MARK)];
  if (found.length === 0) fail(`${file}: no <!-- agents --> list`);
  const wrong = found.filter((m) => m[1] !== list);
  if (wrong.length === 0) continue;
  if (check) {
    for (const m of wrong) fail(`${file}: the list says "${m[1]}", agents.ts says "${list}"`);
  } else {
    writeFileSync(path, text.replace(MARK, `<!-- agents -->${list}<!-- /agents -->`));
    console.log(`wrote ${file}: ${list}`);
  }
}

// The README's per-agent rules are written by hand; this only says which ones to add or delete.
const readme = readFileSync(join(repo, 'README.md'), 'utf8');
const section = readme.split('### When each hold counts')[1]?.split('\n### ')[0] ?? '';
const names = new Set(AGENTS.map((a) => a.name));
const leads = [...section.matchAll(/^\*\*(.+?)\.\*\*/gm)].map((m) => m[1]).filter((n) => names.has(n));
for (const a of SUPPORTED) if (!leads.includes(a.name)) fail(`README.md: "When each hold counts" has no **${a.name}.** paragraph`);
for (const n of leads) if (!SUPPORTED.some((a) => a.name === n)) fail(`README.md: "When each hold counts" has a **${n}.** paragraph, but agents.ts does not list it as passed`);

if (failed) process.exit(1);
console.log(`ok: ${SUPPORTED.length} of ${AGENTS.length} agents passed live (${list}); README and site lists agree`);
