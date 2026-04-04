import { fileURLToPath } from "node:url";
import { mkdir } from "node:fs/promises";
import { resolve } from "node:path";

import { build } from "esbuild";

const root = fileURLToPath(new URL(".", import.meta.url));
const entry = resolve(root, "src/main.jsx");
const outdir = resolve(root, "../crates/dbg-core/ui");

await mkdir(outdir, { recursive: true });

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
