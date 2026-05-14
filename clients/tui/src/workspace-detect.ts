import { statSync } from "node:fs";
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

export function detectWorkspaceDirFromCwd(
  cwd: string,
  isDirectory: IsDirectoryFn = defaultIsDirectory,
  alanHomeDir: string = join(homedir(), ".alan"),
): string | undefined {
  const normalizedCwd = resolve(cwd);
  const normalizedAlanHomeDir = resolve(alanHomeDir);

  if (normalizedCwd === normalizedAlanHomeDir) {
    return undefined;
  }

  if (basename(normalizedCwd) === ".alan" && isDirectory(normalizedCwd)) {
    return dirname(normalizedCwd);
  }

  const alanDir = join(normalizedCwd, ".alan");
  if (resolve(alanDir) === normalizedAlanHomeDir) {
    return undefined;
  }

  return isDirectory(alanDir) ? normalizedCwd : undefined;
}
