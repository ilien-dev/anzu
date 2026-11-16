// Renders hero.html to the README GIFs in assets/motion/ (anzu-hero-*.gif): one per language (en, es) and
// theme (dark, light), on GitHub's own dark and light grounds.
//
//   npm install
//   node export.mjs
//
// Every animation is frozen at time t before each screenshot, so no frame depends on the
// machine's speed. The GIF starts at its poster frame (the book already an EPUB), so a
// reader with autoplay off still sees the whole story, and loops forever. Frames are flat
// colours: a full 256-colour palette and no dithering. Needs ffmpeg on PATH; CHROME_PATH
// picks a local Chromium when Playwright's own browser is not installed.
import { chromium } from "playwright";
import { execFileSync } from "node:child_process";
import { mkdirSync, rmSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const out = join(here, "..");
const FPS = Number(process.env.FPS || 15);
const SCALE = Number(process.env.SCALE || 2); // sharp on high-density screens
const POSTER = 6; // seconds into the loop: the track is full and the EPUB is shown

const browser = await chromium.launch({ executablePath: process.env.CHROME_PATH || undefined });
const page = await browser.newPage({ deviceScaleFactor: SCALE, viewport: { width: 960, height: 400 } });
for (const lang of ["en", "es"]) {
  for (const theme of ["dark", "light"]) {
    const url = pathToFileURL(join(here, "hero.html")).href + `?lang=${lang}&theme=${theme}`;
    await page.goto(url, { waitUntil: "load" });
    await page.evaluate(() => document.fonts.ready);
    const duration = await page.evaluate(() => window.anzuHero.duration);

    const name = `anzu-hero-${lang}-${theme}`;
    const dir = join(here, "frames", name);
    rmSync(dir, { recursive: true, force: true });
    mkdirSync(dir, { recursive: true });
    const n = Math.round(duration * FPS);
    for (let i = 0; i < n; i++) {
      await page.evaluate((t) => window.anzuHero.frame(t), (POSTER + i / FPS) % duration);
      await page.locator("#hero").screenshot({ path: join(dir, `${String(i).padStart(4, "0")}.png`) });
    }

    const gif = join(out, `${name}.gif`);
    execFileSync("ffmpeg", ["-y", "-loglevel", "error", "-framerate", String(FPS), "-i", join(dir, "%04d.png"),
      "-vf", "split[a][b];[a]palettegen=max_colors=256:stats_mode=full[p];[b][p]paletteuse=dither=none:diff_mode=rectangle",
      "-loop", "0", gif]);
    console.log(`${name}.gif  ${960 * SCALE}x${400 * SCALE}  ${n} frames  ${(statSync(gif).size / 1024).toFixed(0)} KB`);
  }
}
await browser.close();
