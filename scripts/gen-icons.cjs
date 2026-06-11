#!/usr/bin/env node
/**
 * gen-icons.cjs — Tauri app-icon generator (uses @resvg/resvg-js)
 *
 * Usage:
 *   node scripts/gen-icons.cjs          # Normal mode
 *   node scripts/gen-icons.cjs --silent # Quiet mode
 */

"use strict";

const { Resvg } = require("@resvg/resvg-js");
const fs = require("fs");
const path = require("path");
const { execSync } = require("child_process");

const ROOT = path.resolve(__dirname, "..");
const ICONS_DIR = path.join(ROOT, "src-tauri", "icons");
const SOURCE_PNG = path.join(ICONS_DIR, "icon.png");
const IS_SILENT = process.argv.includes("--silent");

const log = (msg) => !IS_SILENT && console.log(msg);
const error = (msg) => console.error(msg);

const ICON_SVG = `<svg width="240" height="240" viewBox="5 5 90 90" fill="none" xmlns="http://www.w3.org/2000/svg">
  <path d="M25 45V25H75V45" stroke="#cc785c" stroke-width="6" stroke-linecap="round" stroke-linejoin="round"/>
  <path d="M35 35H65" stroke="#cc785c" stroke-width="4"/>

  <line x1="20" y1="50" x2="80" y2="50" stroke="#e6dfd8" stroke-width="2" stroke-dasharray="4 4"/>

  <path d="M25 55V75H75V55" stroke="#a9583e" stroke-width="6" stroke-linecap="round" stroke-linejoin="round"/>
  <path d="M35 65H65" stroke="#a9583e" stroke-width="4"/>
</svg>`;

(async () => {
  try {
    log("Rendering SVG → icon.png (1024×1024)…");

    const resvg = new Resvg(ICON_SVG, {
      fitTo: { mode: "width", value: 1024 },
      imageRendering: 1,
      shapeRendering: 2,
      textRendering: 2,
    });

    const pngBuffer = resvg.render().asPng();

    fs.mkdirSync(ICONS_DIR, { recursive: true });
    fs.writeFileSync(SOURCE_PNG, pngBuffer);
    log(`Saved ${path.basename(SOURCE_PNG)} (${Math.round(pngBuffer.length / 1024)} KB)`);

    log("Running tauri icon generator…");
    execSync(`bun run tauri -- icon "${SOURCE_PNG}"`, {
      cwd: ROOT,
      stdio: IS_SILENT ? "pipe" : "inherit",
      timeout: 120_000,
    });

    log("All icons generated successfully in src-tauri/icons/");
    log("Rebuild the app to apply: bun run tauri -- build");
  } catch (err) {
    error("Process failed:");
    error(err.message || err);
    process.exit(1);
  }
})();
