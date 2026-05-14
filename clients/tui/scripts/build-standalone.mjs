import { chmod, mkdir, stat } from "node:fs/promises";
import { dirname, resolve } from "node:path";

const entrypoint = "dist/alan-tui.js";
const outfile = process.env.ALAN_TUI_BINARY_OUTFILE ?? "dist/alan-tui";
const resolvedOutfile = resolve(outfile);

await mkdir(dirname(resolvedOutfile), { recursive: true });

const result = await Bun.build({
  entrypoints: [entrypoint],
  target: "bun",
  format: "esm",
  minify: false,
  sourcemap: "none",
  compile: {
    outfile: resolvedOutfile,
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

let outfileStats;
try {
  outfileStats = await stat(resolvedOutfile);
} catch {
  console.error(
    `Standalone build reported success but did not create ${resolvedOutfile}. ` +
      "This Bun version may not support Bun.build({ compile }).",
  );
  process.exit(1);
}

if (!outfileStats.isFile() || outfileStats.size === 0) {
  console.error(`Standalone build did not create a non-empty executable at ${resolvedOutfile}.`);
  process.exit(1);
}

await chmod(resolvedOutfile, 0o755);
