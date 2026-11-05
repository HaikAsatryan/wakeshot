// Proves the rebuild commands work from a clean checkout and give the same files: copies media/ without anything
// built or installed, plus the few repo files media reads, into a temp folder; runs `npm ci` and each build command
// there; then compares every output with the one in this media/ folder, byte for byte.
//   node scripts/check-rebuild.mjs           loop and draft (works before the captures exist)
//   node scripts/check-rebuild.mjs --final   also the final video and poster (after the captures and speed numbers)
// Build the same outputs here first. The Chrome copy Remotion downloads on first use is copied over, not fetched
// again: it is not in the lock file anyway, and Remotion's version pins it.
import {spawnSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import {cpSync, existsSync, mkdirSync, readFileSync, rmSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {dirname, join} from 'node:path';
import {fileURLToPath} from 'node:url';

const media = join(dirname(fileURLToPath(import.meta.url)), '..');
const repo = join(media, '..');
const final = process.argv.includes('--final');
const keep = process.argv.includes('--keep');

/** The repo files outside media/ that media's scripts read: the chimes (make-sounds), the art files (check-art). */
const REPO_INPUTS = [
  'crates/wakeshot-windows/src/sound/chime.rs',
  'docs/research/img/ux/icon-sheet-dark.png',
  'docs/research/img/ux/icon-sheet-light.png',
  'docs/research/img/ux/icon-sheet-contrast.png',
  'assets/wakeshot.ico',
  'assets/wakeshot-notice.png',
];
/** What each build mode writes, relative to media/. */
const OUTPUTS = {
  loop: ['out/icon-loop-light.gif', 'out/icon-loop-dark.gif'],
  draft: ['public/sound/soundtrack.wav', 'out/draft/wakeshot-promo-draft.mp4', 'out/draft/wakeshot-promo-poster-draft.png'],
  video: ['public/sound/soundtrack.wav', 'out/wakeshot-promo.mp4', 'out/wakeshot-promo-poster.png'],
};
const modes = final ? ['loop', 'draft', 'video'] : ['loop', 'draft'];

const root = join(tmpdir(), `wakeshot-media-rebuild-${process.pid}`);
const copy = join(root, 'media');
const NOT_SOURCE = /[\\/](node_modules|\.remotion)([\\/]|$)|[\\/]out[\\/]|[\\/]public[\\/]sound([\\/]|$)/;

function run(cmd, args, cwd) {
  console.log(`> ${cmd} ${args.join(' ')}`);
  // npm is a .cmd file on Windows, which Node starts only through a shell; the arguments here are fixed words.
  const r = spawnSync(cmd, args, {cwd, stdio: 'inherit', shell: cmd === 'npm'});
  if (r.status !== 0) throw new Error(`${cmd} ${args.join(' ')} failed (${r.status})`);
}

const sha = (p) => createHash('sha256').update(readFileSync(p)).digest('hex');

let failed = false;
try {
  mkdirSync(root, {recursive: true});
  cpSync(media, copy, {recursive: true, filter: (src) => !NOT_SOURCE.test(src.slice(media.length)) && !src.slice(media.length).match(/^[\\/]out$/)});
  for (const f of REPO_INPUTS) cpSync(join(repo, f), join(root, f));
  run('npm', ['ci', '--no-audit', '--no-fund'], copy);
  const browser = join(media, 'node_modules', '.remotion');
  if (existsSync(browser)) cpSync(browser, join(copy, 'node_modules', '.remotion'), {recursive: true});
  run(process.execPath, ['scripts/check-art.mjs'], copy);
  for (const mode of modes) run(process.execPath, ['scripts/build.mjs', mode], copy);
  for (const mode of modes) {
    for (const f of OUTPUTS[mode]) {
      const here = join(media, f);
      const there = join(copy, f);
      if (!existsSync(here)) {
        failed = true;
        console.log(`FAIL ${f}: not built in media/ (build it here first)`);
        continue;
      }
      const same = existsSync(there) && sha(here) === sha(there);
      if (!same) failed = true;
      console.log(`${same ? 'ok  ' : 'FAIL'} ${f}: ${same ? 'same bytes' : 'differs'}`);
    }
  }
} catch (e) {
  failed = true;
  console.log(`FAIL ${e.message}`);
} finally {
  if (keep) console.log(`kept ${root}`);
  else rmSync(root, {recursive: true, force: true});
}
console.log(failed ? 'FAIL: the clean rebuild differs' : 'ok: a clean rebuild gives the same files');
process.exit(failed ? 1 : 0);
