// Bakes the new tab's skies: each source picture, doubled, dissolving through
// a halftone screen into the page below its horizon. The dots are drawn here
// once, so a new tab only shows a picture and draws nothing of its own. Run
// with `node scripts/bake-sky.mjs` after changing a source in assets/sky
// (needs ffmpeg with libsvtav1).
import { execFileSync } from "node:child_process";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const source = (name) =>
  fileURLToPath(new URL(`../../assets/sky/${name}-source.png`, import.meta.url));
const out = (name) =>
  fileURLToPath(new URL(`../src/shared/sky/assets/sky-${name}.avif`, import.meta.url));

/** The sources' size; the baked picture is twice it, for a display's pixels. */
const WIDTH = 1672;
const HEIGHT = 941;
const SCALE = 2;
/** The screen's pitch and the band it covers, in the source's pixels. */
const CELL = 7;
const BAND = { top: 486, bottom: 889 };

/** Each sky with the page it dissolves into, and which of its clouds hold the
 *  edge longest: by day the blue between them, by night their moonlit tops.
 *  The page colours are the new tab's `--sky-paper`. */
const SCENES = [
  { name: "day", paper: [247, 247, 248], follow: -1.8 },
  { name: "night", paper: [26, 29, 37], follow: 1.1 },
];

const smooth = (t) => t * t * (3 - 2 * t);

function bake({ name, paper, follow }) {
  const width = WIDTH * SCALE;
  const height = HEIGHT * SCALE;
  const cell = CELL * SCALE;
  const top = BAND.top * SCALE;
  const bottom = BAND.bottom * SCALE;
  const sky = execFileSync(
    "ffmpeg",
    [
      "-v",
      "error",
      "-i",
      source(name),
      "-vf",
      `scale=${width}:${height}:flags=lanczos`,
      "-f",
      "rawvideo",
      "-pix_fmt",
      "rgb24",
      "-",
    ],
    { maxBuffer: width * height * 3 + 1024 },
  );

  // A dot to a cell: whole at the top of the band, gone at its foot, and each
  // row's brighter and darker cells holding on longer or letting go sooner,
  // so the edge follows the clouds rather than a ruler.
  const columns = Math.ceil(width / cell);
  const rows = Math.ceil((bottom - top) / cell);
  const radius = new Float32Array(columns * rows);
  // A little past a cell's corners, so the first rows close up under the
  // solid sky without a seam.
  const reach = cell * 0.74;
  for (let row = 0; row < rows; row++) {
    const luma = new Float32Array(columns);
    let sum = 0;
    for (let column = 0; column < columns; column++) {
      let total = 0;
      let count = 0;
      for (let y = top + row * cell; y < Math.min(bottom, top + (row + 1) * cell); y++)
        for (let x = column * cell; x < Math.min(width, (column + 1) * cell); x++) {
          const at = (y * width + x) * 3;
          total += 0.2126 * sky[at] + 0.7152 * sky[at + 1] + 0.0722 * sky[at + 2];
          count++;
        }
      luma[column] = total / count / 255;
      sum += luma[column];
    }
    const mean = sum / columns;
    const t = (row + 0.5) / rows;
    const base = 1 - smooth(t);
    const lean = 4 * t * (1 - t) * follow;
    for (let column = 0; column < columns; column++) {
      const cover = Math.min(1, Math.max(0, base + lean * (luma[column] - mean)));
      radius[row * columns + column] = reach * Math.sqrt(cover);
    }
  }

  // Each pixel of the band is the sky inside a dot and the page outside it,
  // edged over one pixel; a dot larger than its cell reaches its neighbours.
  const baked = Buffer.from(sky);
  for (let y = top; y < height; y++)
    for (let x = 0; x < width; x++) {
      let cover = 0;
      if (y < bottom) {
        const row = Math.floor((y - top) / cell);
        const column = Math.floor(x / cell);
        for (let dy = -1; dy <= 1; dy++)
          for (let dx = -1; dx <= 1; dx++) {
            const r = row + dy;
            const c = column + dx;
            if (r < 0 || r >= rows || c < 0 || c >= columns) continue;
            const reachOf = radius[r * columns + c];
            if (reachOf <= 0) continue;
            const cx = (c + 0.5) * cell;
            const cy = top + (r + 0.5) * cell;
            const distance = Math.hypot(x + 0.5 - cx, y + 0.5 - cy);
            cover = Math.max(cover, Math.min(1, Math.max(0, reachOf - distance + 0.5)));
            if (cover >= 1) break;
          }
        // Rows above the first close over the solid sky.
        if (y < top + cell / 2 && row === 0) cover = Math.max(cover, 1 - (y - top) / (cell / 2));
      }
      const at = (y * width + x) * 3;
      for (let channel = 0; channel < 3; channel++)
        baked[at + channel] = Math.round(sky[at + channel] * cover + paper[channel] * (1 - cover));
    }

  // Through a file: ffmpeg cuts a frame this large short when it is piped.
  const scratch = mkdtempSync(join(tmpdir(), "drift-sky-"));
  const raw = join(scratch, `${name}.rgb`);
  writeFileSync(raw, baked);
  execFileSync(
    "ffmpeg",
    [
      "-v",
      "error",
      "-y",
      "-f",
      "rawvideo",
      "-pix_fmt",
      "rgb24",
      "-s",
      `${width}x${height}`,
      "-i",
      raw,
      "-vf",
      "scale=out_color_matrix=bt709:out_range=full,format=yuv420p10le",
      "-c:v",
      "libsvtav1",
      "-svtav1-params",
      "tune=0:enable-variance-boost=1",
      "-crf",
      "22",
      "-preset",
      "2",
      "-colorspace",
      "bt709",
      "-color_primaries",
      "bt709",
      "-color_trc",
      "iec61966-2-1",
      "-color_range",
      "pc",
      "-frames:v",
      "1",
      out(name),
    ],
    { stdio: ["ignore", "ignore", "inherit"] },
  );
  rmSync(scratch, { recursive: true });
}

for (const scene of SCENES) bake(scene);
