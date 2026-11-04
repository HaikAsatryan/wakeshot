// A small PNG reader: 8-bit greyscale, RGB, palette and RGBA, not interlaced. Enough for the files the art code and
// the screen captures write, with no package to install.
import {inflateSync} from 'node:zlib';
import {readFileSync} from 'node:fs';

export function readPng(path) {
  const buf = readFileSync(path);
  if (buf.readUInt32BE(0) !== 0x89504e47) throw new Error(`${path}: not a PNG`);
  let at = 8;
  let width = 0;
  let height = 0;
  let depth = 0;
  let type = 0;
  let interlace = 0;
  let palette = null;
  let trns = null;
  const idat = [];
  while (at < buf.length) {
    const size = buf.readUInt32BE(at);
    const kind = buf.toString('latin1', at + 4, at + 8);
    const data = buf.subarray(at + 8, at + 8 + size);
    if (kind === 'IHDR') {
      width = data.readUInt32BE(0);
      height = data.readUInt32BE(4);
      depth = data[8];
      type = data[9];
      interlace = data[12];
    } else if (kind === 'PLTE') palette = data;
    else if (kind === 'tRNS') trns = data;
    else if (kind === 'IDAT') idat.push(data);
    else if (kind === 'IEND') break;
    at += 12 + size;
  }
  if (depth !== 8 || interlace !== 0) throw new Error(`${path}: only 8-bit, not interlaced (depth ${depth})`);
  const channels = {0: 1, 2: 3, 3: 1, 4: 2, 6: 4}[type];
  if (!channels) throw new Error(`${path}: colour type ${type} not supported`);
  const raw = inflateSync(Buffer.concat(idat));
  const stride = width * channels;
  const lines = Buffer.alloc(stride * height);
  for (let y = 0; y < height; y++) {
    const filter = raw[y * (stride + 1)];
    const src = raw.subarray(y * (stride + 1) + 1, (y + 1) * (stride + 1));
    const out = lines.subarray(y * stride, (y + 1) * stride);
    const prev = y > 0 ? lines.subarray((y - 1) * stride, y * stride) : Buffer.alloc(stride);
    for (let x = 0; x < stride; x++) {
      const a = x >= channels ? out[x - channels] : 0;
      const b = prev[x];
      const c = x >= channels ? prev[x - channels] : 0;
      let v = src[x];
      if (filter === 1) v += a;
      else if (filter === 2) v += b;
      else if (filter === 3) v += (a + b) >> 1;
      else if (filter === 4) {
        const p = a + b - c;
        const pa = Math.abs(p - a);
        const pb = Math.abs(p - b);
        const pc = Math.abs(p - c);
        v += pa <= pb && pa <= pc ? a : pb <= pc ? b : c;
      }
      out[x] = v & 0xff;
    }
  }
  const rgba = new Uint8Array(width * height * 4);
  for (let i = 0; i < width * height; i++) {
    let r;
    let g;
    let b;
    let a = 255;
    if (type === 0) r = g = b = lines[i];
    else if (type === 4) {
      r = g = b = lines[i * 2];
      a = lines[i * 2 + 1];
    } else if (type === 2) [r, g, b] = [lines[i * 3], lines[i * 3 + 1], lines[i * 3 + 2]];
    else if (type === 6) [r, g, b, a] = [lines[i * 4], lines[i * 4 + 1], lines[i * 4 + 2], lines[i * 4 + 3]];
    else {
      const p = lines[i];
      [r, g, b] = [palette[p * 3], palette[p * 3 + 1], palette[p * 3 + 2]];
      if (trns && p < trns.length) a = trns[p];
    }
    rgba.set([r, g, b, a], i * 4);
  }
  return {width, height, rgba};
}

/** The 32-bit frames of an ICO file as straight RGBA, as the art code writes them (DIB, bottom-up BGRA). */
export function readIco(path) {
  const buf = readFileSync(path);
  const count = buf.readUInt16LE(4);
  const frames = [];
  for (let i = 0; i < count; i++) {
    const e = 6 + i * 16;
    const offset = buf.readUInt32LE(e + 12);
    const size = buf.readUInt32LE(offset + 4);
    const bpp = buf.readUInt16LE(offset + 14);
    if (bpp !== 32) throw new Error(`${path}: frame ${i} is ${bpp} bpp`);
    const rgba = new Uint8Array(size * size * 4);
    for (let y = 0; y < size; y++) {
      for (let x = 0; x < size; x++) {
        const s = offset + 40 + ((size - 1 - y) * size + x) * 4;
        rgba.set([buf[s + 2], buf[s + 1], buf[s], buf[s + 3]], (y * size + x) * 4);
      }
    }
    frames.push({size, rgba});
  }
  return frames;
}
