// The house rules for media/, as a script:
// - no em dash, no en dash and no emoji in any text file here (Haik's rule for every doc and comment);
// - every asset (each file under public/ and out/, and each font package) has a line in CREDITS.md.
import {readdirSync, readFileSync, statSync} from 'node:fs';
import {dirname, extname, join, relative} from 'node:path';
import {fileURLToPath} from 'node:url';

const media = join(dirname(fileURLToPath(import.meta.url)), '..');
const SKIP = new Set(['node_modules', '.remotion']);
const TEXT = new Set(['.md', '.ts', '.tsx', '.mjs', '.js', '.json', '.ps1', '.txt', '']);

function walk(dir, out = []) {
  for (const name of readdirSync(dir)) {
    if (SKIP.has(name)) continue;
    const p = join(dir, name);
    if (statSync(p).isDirectory()) walk(p, out);
    else out.push(p);
  }
  return out;
}

const rel = (p) => relative(media, p).split('\\').join('/');
const files = walk(media);
let failed = false;

// Em dash, en dash, and the emoji blocks (pictographs, symbols and dingbats, flags, the emoji variation selector).
const BAD = /[\u2013\u2014\u2600-\u27BF\uFE0F]|[\u{1F000}-\u{1FAFF}]/u;
for (const f of files) {
  const r = rel(f);
  if (r.startsWith('out/') || r.startsWith('public/') || r === 'package-lock.json') continue;
  if (!TEXT.has(extname(f))) continue;
  readFileSync(f, 'utf8')
    .split('\n')
    .forEach((line, i) => {
      if (BAD.test(line)) {
        failed = true;
        console.log(`FAIL ${r}:${i + 1}: em dash, en dash or emoji: ${line.trim().slice(0, 80)}`);
      }
    });
}

const credits = readFileSync(join(media, 'CREDITS.md'), 'utf8');
const assets = files.map(rel).filter((r) => (r.startsWith('public/') || r.startsWith('out/')) && !r.startsWith('out/draft/'));
for (const a of assets) {
  if (!credits.includes(`\`${a}\``)) {
    failed = true;
    console.log(`FAIL ${a}: no line in CREDITS.md`);
  }
}
for (const pkg of ['@fontsource/inter', '@fontsource/jetbrains-mono']) {
  if (!credits.includes(`\`${pkg}\``)) {
    failed = true;
    console.log(`FAIL font ${pkg}: no line in CREDITS.md`);
  }
}

console.log(failed ? 'FAIL' : `ok: ${files.length} files, ${assets.length} assets credited, no em dash, en dash or emoji`);
process.exit(failed ? 1 : 0);
