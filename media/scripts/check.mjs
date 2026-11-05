// The house rules for media/, as a script:
// - no em dash, no en dash and no emoji in any text file here (Haik's rule for every doc and comment);
// - every asset (each file under public/ and out/, and each font package) has a line in CREDITS.md;
// - every text colour pair in src/theme.ts reaches WCAG AA (4.5:1) and every graphic pair 3:1 (WCAG 1.4.11).
import {readdirSync, readFileSync, statSync} from 'node:fs';
import {dirname, extname, join, relative} from 'node:path';
import {fileURLToPath} from 'node:url';
import {GRAPHIC_PAIRS, TEXT_PAIRS} from '../src/theme.ts';

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
for (const pkg of ['@fontsource/jetbrains-mono']) {
  if (!credits.includes(`\`${pkg}\``)) {
    failed = true;
    console.log(`FAIL font ${pkg}: no line in CREDITS.md`);
  }
}

// WCAG 2.2 relative luminance and contrast ratio.
const channel = (v) => (v <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4);
const luminance = (hex) => {
  const [r, g, b] = [1, 3, 5].map((i) => channel(parseInt(hex.slice(i, i + 2), 16) / 255));
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
};
const contrast = (a, b) => {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (hi + 0.05) / (lo + 0.05);
};
let pairs = 0;
for (const [list, min] of [
  [TEXT_PAIRS, 4.5],
  [GRAPHIC_PAIRS, 3],
]) {
  for (const [what, fg, bg] of list) {
    const r = contrast(fg, bg);
    pairs++;
    if (r < min) {
      failed = true;
      console.log(`FAIL ${what}: ${fg} on ${bg} is ${r.toFixed(2)}:1, needs ${min}:1`);
    }
  }
}

console.log(
  failed
    ? 'FAIL'
    : `ok: ${files.length} files, ${assets.length} assets credited, no em dash, en dash or emoji, ${pairs} colour pairs pass`,
);
process.exit(failed ? 1 : 0);
