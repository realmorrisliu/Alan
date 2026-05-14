import { realpathSync, statSync } from "node:fs";
import { homedir } from "node:os";
import { basename, dirname, join, resolve } from "node:path";

export type IsDirectoryFn = (path: string) => boolean;

function defaultIsDirectory(path: string): boolean {
  try {
    return statSync(path).isDirectory();
  } catch {
    return false;
  }
}

function canonicalizeExistingOrParent(path: string): string {
  const normalized = resolve(path);
  try {
    return realpathSync(normalized);
  } catch {
    // Continue below: the path may be a not-yet-created child under a symlinked parent.
  }

  const suffix: string[] = [];
  let cursor = normalized;
  while (true) {
    const parent = dirname(cursor);
    const name = basename(cursor);
    if (!name || parent === cursor) {
      return normalized;
    }
    suffix.push(name);
    try {
      return suffix.reduceRight(
        (canonical, component) => join(canonical, component),
        realpathSync(parent),
      );
    } catch {
      cursor = parent;
    }
  }
}

export function detectWorkspaceDirFromCwd(
  cwd: string,
  isDirectory: IsDirectoryFn = defaultIsDirectory,
  alanHomeDir: string = join(homedir(), ".alan"),
): string | undefined {
  const normalizedCwd = resolve(cwd);
  const comparableCwd = canonicalizeExistingOrParent(normalizedCwd);
  const comparableAlanHomeDir = canonicalizeExistingOrParent(alanHomeDir);

  if (comparableCwd === comparableAlanHomeDir) {
    return undefined;
  }

  if (basename(normalizedCwd) === ".alan" && isDirectory(normalizedCwd)) {
    return dirname(normalizedCwd);
  }

  const alanDir = join(normalizedCwd, ".alan");
  if (canonicalizeExistingOrParent(alanDir) === comparableAlanHomeDir) {
    return undefined;
  }

  return isDirectory(alanDir) ? normalizedCwd : undefined;
}
