import { statSync } from "node:fs";
import { join } from "node:path";

function expandHomePath(path: string, homeDir: string): string {
  if (!path.startsWith("~/")) {
    return path;
  }
  return join(homeDir, path.slice(2));
}

function dedupe(paths: string[]): string[] {
  const seen = new Set<string>();
  const out: string[] = [];
  for (const path of paths) {
    if (!seen.has(path)) {
      seen.add(path);
      out.push(path);
    }
  }
  return out;
}

export function resolveConfigPathCandidates(
  homeDir: string,
  env: NodeJS.ProcessEnv = process.env,
): string[] {
  const canonicalPath = join(homeDir, ".alan", "agent", "agent.toml");
  const overrideRaw = env.ALAN_CONFIG_PATH?.trim();
  if (!overrideRaw) {
    return [canonicalPath];
  }

  const overridePath = expandHomePath(overrideRaw, homeDir);
  return dedupe([overridePath, canonicalPath]);
}

export function defaultHostConfigPath(homeDir: string): string {
  return join(homeDir, ".alan", "host.toml");
}

export function defaultLegacyConfigPath(homeDir: string): string {
  return join(homeDir, ".config", "alan", "config.toml");
}

export function selectExistingConfigPath(
  candidates: string[],
  isConfigFile: (path: string) => boolean,
): string | null {
  for (const candidate of candidates) {
    if (isConfigFile(candidate)) {
      return candidate;
    }
  }
  return null;
}

export function shouldRunFirstTimeSetup(
  candidates: string[],
  isConfigFile: (path: string) => boolean,
): boolean {
  return selectExistingConfigPath(candidates, isConfigFile) === null;
}

export function legacyConfigRequiresMigration(
  candidates: string[],
  legacyConfigPath: string,
  isConfigFile: (path: string) => boolean,
): boolean {
  return (
    selectExistingConfigPath(candidates, isConfigFile) === null &&
    isConfigFile(legacyConfigPath)
  );
}

export function isExistingConfigFile(path: string): boolean {
  try {
    return statSync(path).isFile();
  } catch {
    return false;
  }
}
