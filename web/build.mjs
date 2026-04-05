import { fileURLToPath } from "node:url";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { resolve } from "node:path";

import { build } from "esbuild";
import postcss from "postcss";
import tailwindcss from "@tailwindcss/postcss";
import autoprefixer from "autoprefixer";

const root = fileURLToPath(new URL(".", import.meta.url));
const entry = resolve(root, "src/main.jsx");
const outdir = resolve(root, "../crates/dbg-core/ui");

await mkdir(outdir, { recursive: true });

// Process Tailwind CSS
const cssInput = resolve(root, "src/styles/globals.css");
const cssOutput = resolve(outdir, "globals.css");

const css = await readFile(cssInput, "utf-8");
const result = await postcss([
  tailwindcss(),
  autoprefixer,
]).process(css, { from: cssInput, to: cssOutput });

await writeFile(cssOutput, result.css);
console.log("✓ Built globals.css");

// Build JS bundle
await build({
  entryPoints: [entry],
  outfile: resolve(outdir, "app.js"),
  bundle: true,
  format: "esm",
  jsx: "automatic",
  legalComments: "none",
  loader: {
    ".js": "jsx",
    ".jsx": "jsx"
  },
  minify: true,
  platform: "browser",
  sourcemap: false,
  target: ["es2022"]
});

console.log("✓ Built app.js");
