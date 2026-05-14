import { defineConfig } from "rolldown";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const configDir = dirname(fileURLToPath(import.meta.url));

export default defineConfig({
  input: "src/index.tsx",
  platform: "node",
  external: ["ws"],
  resolve: {
    alias: {
      "react-devtools-core": resolve(configDir, ".shims/react-devtools-core/index.js"),
    },
  },
  tsconfig: "./tsconfig.json",
  output: {
    file: "dist/alan-tui.js",
    format: "esm",
    codeSplitting: false,
    minify: true,
    sourcemap: false,
  },
});
