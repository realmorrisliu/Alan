import { mkdir } from "node:fs/promises";
import { dirname, resolve } from "node:path";

const entrypoint = "dist/alan-tui.js";
const outfile = process.env.ALAN_TUI_BINARY_OUTFILE ?? "dist/alan-tui";

await mkdir(dirname(resolve(outfile)), { recursive: true });

const result = await Bun.build({
  entrypoints: [entrypoint],
  target: "bun",
  format: "esm",
  minify: false,
  sourcemap: "none",
  compile: {
    outfile,
    autoloadDotenv: false,
    autoloadBunfig: false,
    autoloadTsconfig: false,
    autoloadPackageJson: false,
  },
});

for (const log of result.logs) {
  console.error(log);
}

if (!result.success) {
  process.exit(1);
}
