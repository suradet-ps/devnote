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

const ICON_SVG = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="100%" height="100%">
  <defs>
    <filter id="icon-shadow" x="-20%" y="-20%" width="140%" height="140%">
      <feDropShadow dx="0" dy="12" stdDeviation="16" flood-color="#0F172A" flood-opacity="0.25"/>
      <feDropShadow dx="0" dy="4" stdDeviation="6" flood-color="#0F172A" flood-opacity="0.15"/>
    </filter>
    <filter id="code-shadow" x="-20%" y="-20%" width="140%" height="140%">
      <feDropShadow dx="1" dy="6" stdDeviation="5" flood-color="#7C2D12" flood-opacity="0.25"/>
      <feDropShadow dx="0" dy="2" stdDeviation="2" flood-color="#000000" flood-opacity="0.15"/>
    </filter>
    <linearGradient id="paper-gradient" x1="0%" y1="0%" x2="0%" y2="100%">
      <stop offset="0%" stop-color="#FCFBF7"/>
      <stop offset="100%" stop-color="#F4F1EA"/>
    </linearGradient>
    <linearGradient id="ring-gradient" x1="0%" y1="0%" x2="100%" y2="0%">
      <stop offset="0%" stop-color="#94A3B8"/>
      <stop offset="50%" stop-color="#E2E8F0"/>
      <stop offset="100%" stop-color="#64748B"/>
    </linearGradient>
    <linearGradient id="code-orange" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#FF9F43"/>
      <stop offset="50%" stop-color="#FF5E36"/>
      <stop offset="100%" stop-color="#E03E1A"/>
    </linearGradient>
  </defs>
  <g filter="url(#icon-shadow)">
    <rect x="55" y="25" width="415" height="462" rx="32" fill="url(#paper-gradient)" stroke="#E2DEC9" stroke-width="2"/>
    <g stroke="#E8E4D3" stroke-width="5" stroke-linecap="round">
      <line x1="165" y1="105" x2="430" y2="105"/>
      <line x1="165" y1="155" x2="430" y2="155"/>
      <line x1="165" y1="365" x2="430" y2="365"/>
      <line x1="165" y1="415" x2="340" y2="415"/>
    </g>
    <g filter="url(#code-shadow)">
      <rect x="35" y="70" width="44" height="24" rx="12" fill="url(#ring-gradient)"/>
      <rect x="35" y="70" width="44" height="7" rx="3.5" fill="#FFFFFF" opacity="0.4"/>
      <rect x="35" y="135" width="44" height="24" rx="12" fill="url(#ring-gradient)"/>
      <rect x="35" y="135" width="44" height="7" rx="3.5" fill="#FFFFFF" opacity="0.4"/>
      <rect x="35" y="200" width="44" height="24" rx="12" fill="url(#ring-gradient)"/>
      <rect x="35" y="200" width="44" height="7" rx="3.5" fill="#FFFFFF" opacity="0.4"/>
      <rect x="35" y="265" width="44" height="24" rx="12" fill="url(#ring-gradient)"/>
      <rect x="35" y="265" width="44" height="7" rx="3.5" fill="#FFFFFF" opacity="0.4"/>
      <rect x="35" y="330" width="44" height="24" rx="12" fill="url(#ring-gradient)"/>
      <rect x="35" y="330" width="44" height="7" rx="3.5" fill="#FFFFFF" opacity="0.4"/>
      <rect x="35" y="395" width="44" height="24" rx="12" fill="url(#ring-gradient)"/>
      <rect x="35" y="395" width="44" height="7" rx="3.5" fill="#FFFFFF" opacity="0.4"/>
    </g>
    <g fill="#64748B" opacity="0.7">
      <circle cx="57" cy="82" r="6"/>
      <circle cx="57" cy="147" r="6"/>
      <circle cx="57" cy="212" r="6"/>
      <circle cx="57" cy="277" r="6"/>
      <circle cx="57" cy="342" r="6"/>
      <circle cx="57" cy="407" r="6"/>
    </g>
    <g transform="translate(153, 193) scale(1.25)" filter="url(#code-shadow)">
      <path d="M 60,15 L 15,55 L 60,95" fill="none" stroke="url(#code-orange)" stroke-width="16" stroke-linecap="round" stroke-linejoin="round"/>
      <line x1="105" y1="5" x2="90" y2="105" stroke="url(#code-orange)" stroke-width="16" stroke-linecap="round"/>
      <path d="M 135,15 L 180,55 L 135,95" fill="none" stroke="url(#code-orange)" stroke-width="16" stroke-linecap="round" stroke-linejoin="round"/>
    </g>
  </g>
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
