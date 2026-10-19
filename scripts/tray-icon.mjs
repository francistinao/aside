// Draws the menu bar extra glyph: a speech bubble with Aside's one sharp corner and two
// text lines, as template PNGs (black with alpha) at 18 and 36 px. Node only, no tools.
// Usage: node scripts/tray-icon.mjs
import { deflateSync } from 'node:zlib';
import { writeFileSync, mkdirSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const OUT = resolve(dirname(fileURLToPath(import.meta.url)), '../src-tauri/icons/tray');
const SAMPLES = 4;

// Signed distance to a rounded box with one radius per corner, centered at (cx, cy).
function roundedBox(px, py, cx, cy, hw, hh, radii) {
  const x = px - cx;
  const y = py - cy;
  const r = x > 0 ? (y > 0 ? radii.br : radii.tr) : y > 0 ? radii.bl : radii.tl;
  const qx = Math.abs(x) - hw + r;
  const qy = Math.abs(y) - hh + r;
  return Math.min(Math.max(qx, qy), 0) + Math.hypot(Math.max(qx, 0), Math.max(qy, 0)) - r;
}

// Signed distance to a capsule from (ax, ay) to (bx, by) with the given radius.
function capsule(px, py, ax, ay, bx, by, radius) {
  const abx = bx - ax;
  const aby = by - ay;
  const t = Math.max(0, Math.min(1, ((px - ax) * abx + (py - ay) * aby) / (abx * abx + aby * aby)));
  return Math.hypot(px - (ax + abx * t), py - (ay + aby * t)) - radius;
}

// The glyph in 18 pt units. Distance is negative inside the ink.
function ink(x, y) {
  const outline = Math.abs(roundedBox(x, y, 9, 8.5, 7, 5.5, { tl: 3, tr: 3, bl: 3, br: 0 })) - 0.75;
  const line1 = capsule(x, y, 5.5, 6.75, 12.5, 6.75, 0.7);
  const line2 = capsule(x, y, 5.5, 10.25, 9.5, 10.25, 0.7);
  return Math.min(outline, line1, line2);
}

function render(size) {
  const scale = size / 18;
  const rgba = Buffer.alloc(size * size * 4);
  for (let y = 0; y < size; y += 1) {
    for (let x = 0; x < size; x += 1) {
      let coverage = 0;
      for (let sy = 0; sy < SAMPLES; sy += 1) {
        for (let sx = 0; sx < SAMPLES; sx += 1) {
          const px = (x + (sx + 0.5) / SAMPLES) / scale;
          const py = (y + (sy + 0.5) / SAMPLES) / scale;
          coverage += Math.max(0, Math.min(1, 0.5 - ink(px, py) * scale));
        }
      }
      const alpha = Math.round((coverage / (SAMPLES * SAMPLES)) * 255);
      const i = (y * size + x) * 4;
      rgba[i] = 0;
      rgba[i + 1] = 0;
      rgba[i + 2] = 0;
      rgba[i + 3] = alpha;
    }
  }
  return rgba;
}

const CRC_TABLE = new Uint32Array(256).map((_, n) => {
  let c = n;
  for (let k = 0; k < 8; k += 1) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
  return c >>> 0;
});

function crc32(buffer) {
  let crc = 0xffffffff;
  for (const byte of buffer) crc = CRC_TABLE[(crc ^ byte) & 0xff] ^ (crc >>> 8);
  return (crc ^ 0xffffffff) >>> 0;
}

function chunk(type, data) {
  const length = Buffer.alloc(4);
  length.writeUInt32BE(data.length);
  const body = Buffer.concat([Buffer.from(type, 'ascii'), data]);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(body));
  return Buffer.concat([length, body, crc]);
}

function png(size, rgba) {
  const header = Buffer.alloc(13);
  header.writeUInt32BE(size, 0);
  header.writeUInt32BE(size, 4);
  header[8] = 8; // bit depth
  header[9] = 6; // RGBA
  const raw = Buffer.alloc((size * 4 + 1) * size);
  for (let y = 0; y < size; y += 1) {
    raw[y * (size * 4 + 1)] = 0; // no filter
    rgba.copy(raw, y * (size * 4 + 1) + 1, y * size * 4, (y + 1) * size * 4);
  }
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk('IHDR', header),
    chunk('IDAT', deflateSync(raw)),
    chunk('IEND', Buffer.alloc(0)),
  ]);
}

mkdirSync(OUT, { recursive: true });
for (const [name, size] of [
  ['trayTemplate.png', 18],
  ['trayTemplate@2x.png', 36],
]) {
  writeFileSync(resolve(OUT, name), png(size, render(size)));
}
