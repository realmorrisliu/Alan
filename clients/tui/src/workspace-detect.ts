import { statSync } from "node:fs";
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
): string | undefined {
  const normalizedCwd = resolve(cwd);

  if (basename(normalizedCwd) === ".alan" && isDirectory(normalizedCwd)) {
    return dirname(normalizedCwd);
  }

  const alanDir = join(normalizedCwd, ".alan");
  return isDirectory(alanDir) ? normalizedCwd : undefined;
}
