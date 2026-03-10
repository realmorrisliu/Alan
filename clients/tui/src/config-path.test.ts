import { describe, expect, test } from "bun:test";
import {
  mkdtempSync,
  mkdirSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  isExistingConfigFile,
  resolveConfigPathCandidates,
  selectExistingConfigPath,
  shouldRunFirstTimeSetup,
} from "./config-path.js";

describe("config path resolution", () => {
  const home = "/Users/tester";
  const defaultPath = `${home}/.config/alan/config.toml`;

  test("uses default config path when override is unset", () => {
    const candidates = resolveConfigPathCandidates(home, {});
    expect(candidates).toEqual([defaultPath]);
  });

  test("adds override before default when ALAN_CONFIG_PATH is set", () => {
    const override = "/tmp/custom.toml";
    const candidates = resolveConfigPathCandidates(home, {
      ALAN_CONFIG_PATH: override,
    });
    expect(candidates).toEqual([override, defaultPath]);
  });

  test("expands ~/ override path", () => {
    const candidates = resolveConfigPathCandidates(home, {
      ALAN_CONFIG_PATH: "~/custom.toml",
    });
    expect(candidates).toEqual([`${home}/custom.toml`, defaultPath]);
  });

  test("selects default path when override does not exist", () => {
    const candidates = resolveConfigPathCandidates(home, {
      ALAN_CONFIG_PATH: "/tmp/missing.toml",
    });
    const existing = selectExistingConfigPath(candidates, (path) => path === defaultPath);
    expect(existing).toBe(defaultPath);
  });

  test("does not run setup when default config exists and override is missing", () => {
    const candidates = resolveConfigPathCandidates(home, {
      ALAN_CONFIG_PATH: "/tmp/missing.toml",
    });
    const needsSetup = shouldRunFirstTimeSetup(
      candidates,
      (path) => path === defaultPath,
    );
    expect(needsSetup).toBe(false);
  });

  test("isExistingConfigFile returns false for directory and true for regular file", () => {
    const tempRoot = mkdtempSync(join(tmpdir(), "alan-config-path-"));
    const configDir = join(tempRoot, "config-dir");
    const configFile = join(tempRoot, "config.toml");
    mkdirSync(configDir);
    writeFileSync(configFile, 'llm_provider = "google_gemini_generate_content"\n');

    expect(isExistingConfigFile(configDir)).toBe(false);
    expect(isExistingConfigFile(configFile)).toBe(true);

    rmSync(tempRoot, { recursive: true, force: true });
  });
});
