// A small GIF writer and reader, with no package to install.
// The writer exists because a GIF made with one palette for the whole clip (Remotion's `palettegen`) moves colours:
// the old loop's dark background came out one level off the colour it was drawn on. Here every frame gets its own
// palette of exactly the colours it uses, so the background is GitHub's page colour to the bit and the brand colours
// are exact.
// The reader composes every frame as a viewer shows it, so the checks read frame count, length and colours from the
// file itself.
import {readFileSync, writeFileSync} from 'node:fs';

/** LZW-compresses colour indexes into GIF sub-blocks. */
function lzwEncode(minCode, idx) {
  const clear = 1 << minCode;
  const end = clear + 1;
  const bytes = [];
  let acc = 0;
  let bits = 0;
  let size = minCode + 1;
  const put = (code) => {
    acc |= code << bits;
    bits += size;
    while (bits >= 8) {
      bytes.push(acc & 0xff);
      acc >>>= 8;
      bits -= 8;
    }
  };
  let table = new Map();
  let next = end + 1;
  put(clear);
  let cur = idx[0];
  for (let i = 1; i < idx.length; i++) {
    const k = idx[i];
    const key = (cur << 8) | k;
    const found = table.get(key);
    if (found !== undefined) {
      cur = found;
      continue;
    }
    put(cur);
    if (next === 4096) {
      // The table is full: start a new one, as decoders expect a clear code before any further code.
      put(clear);
      table = new Map();
      next = end + 1;
      size = minCode + 1;
    } else {
      // The size grows before the code that needs it is stored, one step ahead of the decoder, which adds the
      // matching entry one code later.
      if (next >= 1 << size) size++;
      table.set(key, next++);
    }
    cur = k;
  }
  put(cur);
  put(end);
  if (bits > 0) bytes.push(acc & 0xff);
  const out = [];
  for (let i = 0; i < bytes.length; i += 255) {
    const n = Math.min(255, bytes.length - i);
    out.push(n);
    for (let j = 0; j < n; j++) out.push(bytes[i + j]);
  }
  out.push(0);
  return out;
}

/**
 * Writes a GIF that loops forever. `frames` are `{rgba, delay}` with RGBA of the whole `width` x `height` screen and
 * the delay in hundredths of a second. Repeated frames are merged into one with the summed delay; each later frame
 * stores only the box that changed, with unchanged pixels transparent. Every frame's palette holds exactly its own
 * colours while they fit; `worstMerge` reports how far, in levels of 255, a merged edge colour moved (0: none did).
 */
export function writeGif(path, width, height, frames) {
  const merged = [];
  for (const f of frames) {
    const last = merged[merged.length - 1];
    if (last && Buffer.from(last.rgba.buffer, last.rgba.byteOffset, last.rgba.byteLength).equals(f.rgba)) {
      last.delay += f.delay;
    } else {
      merged.push({rgba: f.rgba, delay: f.delay});
    }
  }
  const out = [];
  const u16 = (v) => out.push(v & 0xff, v >> 8);
  out.push(...Buffer.from('GIF89a', 'latin1'));
  u16(width);
  u16(height);
  out.push(0, 0, 0);
  out.push(0x21, 0xff, 11, ...Buffer.from('NETSCAPE2.0', 'latin1'), 3, 1, 0, 0, 0);
  // What a viewer shows after each frame: later frames are compared with it, so a colour merged in one frame is
  // sent again as soon as it should change.
  let shown = null;
  let mostColours = 0;
  let worstMerge = 0;
  for (const f of merged) {
    for (let i = 3; i < f.rgba.length; i += 4) {
      if (f.rgba[i] !== 255) throw new Error(`${path}: a frame has a see-through pixel; the loop must be opaque`);
    }
    const px = (i) => (f.rgba[i * 4] << 16) | (f.rgba[i * 4 + 1] << 8) | f.rgba[i * 4 + 2];
    const same = (i) =>
      shown !== null &&
      shown[i * 4] === f.rgba[i * 4] &&
      shown[i * 4 + 1] === f.rgba[i * 4 + 1] &&
      shown[i * 4 + 2] === f.rgba[i * 4 + 2];
    let x0 = 0;
    let y0 = 0;
    let x1 = width;
    let y1 = height;
    if (shown) {
      x0 = width;
      y0 = height;
      x1 = 0;
      y1 = 0;
      for (let y = 0; y < height; y++) {
        for (let x = 0; x < width; x++) {
          if (same(y * width + x)) continue;
          x0 = Math.min(x0, x);
          y0 = Math.min(y0, y);
          x1 = Math.max(x1, x + 1);
          y1 = Math.max(y1, y + 1);
        }
      }
    }
    const w = x1 - x0;
    const h = y1 - y0;
    const transparent = shown !== null;
    const counts = new Map();
    for (let y = y0; y < y1; y++) {
      for (let x = x0; x < x1; x++) {
        const i = y * width + x;
        if (transparent && same(i)) continue;
        counts.set(px(i), (counts.get(px(i)) ?? 0) + 1);
      }
    }
    mostColours = Math.max(mostColours, counts.size);
    // Over a palette's room, the rarest colour joins its nearest: those are single edge pixels a level or two apart,
    // while the colours that fill areas (the page, the glyph, the brand colour) are the most common and stay exact.
    const room = transparent ? 255 : 256;
    const into = new Map();
    const rgb = (c) => [c >> 16, (c >> 8) & 0xff, c & 0xff];
    while (counts.size > room) {
      let rare = -1;
      for (const [c, n] of counts) {
        if (rare === -1 || n < counts.get(rare) || (n === counts.get(rare) && c < rare)) rare = c;
      }
      const a = rgb(rare);
      let best = -1;
      let bestD = Infinity;
      for (const c of counts.keys()) {
        if (c === rare) continue;
        const b = rgb(c);
        const d = (a[0] - b[0]) ** 2 + (a[1] - b[1]) ** 2 + (a[2] - b[2]) ** 2;
        if (d < bestD || (d === bestD && c < best)) [best, bestD] = [c, d];
      }
      counts.set(best, counts.get(best) + counts.get(rare));
      counts.delete(rare);
      into.set(rare, best);
      for (const [from, to] of into) if (to === rare) into.set(from, best);
    }
    for (const [from, to] of into) {
      worstMerge = Math.max(worstMerge, ...rgb(from).map((v, k) => Math.abs(v - rgb(to)[k])));
    }
    const index = new Map([...counts.keys()].map((c, k) => [c, k + (transparent ? 1 : 0)]));
    const next = shown ? shown.slice() : new Uint8Array(f.rgba.length);
    const idx = new Uint8Array(w * h);
    for (let y = 0; y < h; y++) {
      for (let x = 0; x < w; x++) {
        const i = (y0 + y) * width + x0 + x;
        if (transparent && same(i)) continue;
        const c = into.get(px(i)) ?? px(i);
        idx[y * w + x] = index.get(c);
        next.set([...rgb(c), 255], i * 4);
      }
    }
    const used = index.size + (transparent ? 1 : 0);
    let depth = 1;
    while (1 << depth < used) depth++;
    const table = new Array(3 << depth).fill(0);
    for (const [c, k] of index) table.splice(k * 3, 3, ...rgb(c));
    out.push(0x21, 0xf9, 4, (1 << 2) | (transparent ? 1 : 0));
    u16(f.delay);
    out.push(0, 0);
    out.push(0x2c);
    u16(x0);
    u16(y0);
    u16(w);
    u16(h);
    out.push(0x80 | (depth - 1), ...table);
    const minCode = Math.max(2, depth);
    out.push(minCode);
    for (const byte of lzwEncode(minCode, idx)) out.push(byte);
    shown = next;
  }
  out.push(0x3b);
  writeFileSync(path, Buffer.from(out));
  return {frames: merged.length, mostColours, worstMerge};
}

/** Decodes one image's LZW data into colour indexes. */
function lzw(minCode, data, count) {
  const out = new Uint8Array(count);
  const clear = 1 << minCode;
  const end = clear + 1;
  // Each code is stored as its last byte, its parent code and its length, so a string is walked from its end.
  const suffix = new Uint8Array(4096);
  const prefix = new Int16Array(4096);
  const length = new Uint16Array(4096);
  for (let i = 0; i < clear; i++) {
    suffix[i] = i;
    prefix[i] = -1;
    length[i] = 1;
  }
  let size = minCode + 1;
  let next = end + 1;
  let prev = -1;
  let first = 0;
  let at = 0;
  let bits = 0;
  let acc = 0;
  for (let i = 0; i < data.length && at < count; i++) {
    acc |= data[i] << bits;
    bits += 8;
    while (bits >= size && at < count) {
      const code = acc & ((1 << size) - 1);
      acc >>>= size;
      bits -= size;
      if (code === clear) {
        size = minCode + 1;
        next = end + 1;
        prev = -1;
        continue;
      }
      if (code === end) return out;
      let cur = code;
      if (prev === -1) {
        out[at++] = suffix[code];
        first = suffix[code];
        prev = code;
        continue;
      }
      if (code >= next) {
        // The code being defined right now: the previous string plus its own first byte.
        cur = prev;
      }
      const n = length[cur];
      let k = cur;
      for (let j = n - 1; j >= 0; j--) {
        if (at + j < count) out[at + j] = suffix[k];
        k = prefix[k];
      }
      first = out[at];
      at += n;
      if (code >= next) {
        if (at < count) out[at] = first;
        at++;
      }
      if (next < 4096) {
        prefix[next] = prev;
        suffix[next] = first;
        length[next] = length[prev] + 1;
        next++;
        if (next === 1 << size && size < 12) size++;
      }
      prev = code;
    }
  }
  return out;
}

/**
 * Reads a GIF. Returns its size, its loop count (NETSCAPE2.0, 0 is forever, null when absent) and every frame as RGBA
 * of the whole screen after that frame is drawn, with the frame's delay in hundredths of a second.
 */
export function readGif(path) {
  const b = readFileSync(path);
  const version = b.toString('latin1', 0, 6);
  if (version !== 'GIF89a' && version !== 'GIF87a') throw new Error(`${path}: not a GIF`);
  const width = b.readUInt16LE(6);
  const height = b.readUInt16LE(8);
  const flags = b[10];
  let at = 13;
  let global = null;
  if (flags & 0x80) {
    const n = 3 << ((flags & 7) + 1);
    global = b.subarray(at, at + n);
    at += n;
  }
  const screen = new Uint8Array(width * height * 4);
  const frames = [];
  let loops = null;
  let gce = {delay: 0, disposal: 0, transparent: -1};
  while (at < b.length) {
    const kind = b[at++];
    if (kind === 0x3b) break;
    if (kind === 0x21) {
      const label = b[at++];
      const blocks = [];
      while (b[at] !== 0) {
        blocks.push(b.subarray(at + 1, at + 1 + b[at]));
        at += 1 + b[at];
      }
      at++;
      if (label === 0xf9) {
        const d = blocks[0];
        gce = {delay: d.readUInt16LE(1), disposal: (d[0] >> 2) & 7, transparent: d[0] & 1 ? d[3] : -1};
      } else if (label === 0xff && blocks[0]?.toString('latin1') === 'NETSCAPE2.0') {
        loops = blocks[1].readUInt16LE(1);
      }
      continue;
    }
    if (kind !== 0x2c) throw new Error(`${path}: unknown block 0x${kind.toString(16)} at ${at - 1}`);
    const x = b.readUInt16LE(at);
    const y = b.readUInt16LE(at + 2);
    const w = b.readUInt16LE(at + 4);
    const h = b.readUInt16LE(at + 6);
    const f = b[at + 8];
    at += 9;
    let table = global;
    if (f & 0x80) {
      const n = 3 << ((f & 7) + 1);
      table = b.subarray(at, at + n);
      at += n;
    }
    if (f & 0x40) throw new Error(`${path}: interlaced frames are not supported`);
    const minCode = b[at++];
    const parts = [];
    while (b[at] !== 0) {
      parts.push(b.subarray(at + 1, at + 1 + b[at]));
      at += 1 + b[at];
    }
    at++;
    const before = gce.disposal === 3 ? screen.slice() : null;
    const idx = lzw(minCode, Buffer.concat(parts), w * h);
    for (let j = 0; j < h; j++) {
      for (let i = 0; i < w; i++) {
        const c = idx[j * w + i];
        if (c === gce.transparent) continue;
        const sx = x + i;
        const sy = y + j;
        if (sx >= width || sy >= height) continue;
        const o = (sy * width + sx) * 4;
        screen[o] = table[c * 3];
        screen[o + 1] = table[c * 3 + 1];
        screen[o + 2] = table[c * 3 + 2];
        screen[o + 3] = 255;
      }
    }
    frames.push({delay: gce.delay, rgba: screen.slice()});
    if (gce.disposal === 2) {
      for (let j = y; j < Math.min(height, y + h); j++) {
        screen.fill(0, (j * width + x) * 4, (j * width + Math.min(width, x + w)) * 4);
      }
    } else if (before) {
      screen.set(before);
    }
    gce = {delay: 0, disposal: 0, transparent: -1};
  }
  return {version, width, height, loops, frames};
}
