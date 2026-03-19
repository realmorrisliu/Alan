import { chmodSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";
import { isExistingConfigFile } from "./config-path.js";

interface WriteCanonicalSetupFilesParams {
  agentConfigPath: string;
  agentConfigContent: string;
  hostConfigPath: string;
  hostConfigContent: string;
}

interface WriteCanonicalSetupFilesResult {
  hostConfigStatus: "created" | "preserved";
}

function readConfigFile(path: string): string {
  try {
    return readFileSync(path, "utf8");
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    throw new Error(`Failed to read configuration file at ${path}: ${message}`);
  }
}

function writeConfigFile(path: string, content: string): void {
  try {
    mkdirSync(dirname(path), { recursive: true });
    writeFileSync(path, content, { mode: 0o600 });
    chmodSync(path, 0o600);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    throw new Error(
      `Failed to write configuration file at ${path}: ${message}`,
    );
  }
}

function validateExistingHostConfig(path: string): void {
  let parsed: unknown;
  try {
    parsed = Bun.TOML.parse(readConfigFile(path));
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    throw new Error(
      `Existing host configuration at ${path} is invalid: ${message}`,
    );
  }

  if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) {
    throw new Error(
      `Existing host configuration at ${path} must be a TOML table.`,
    );
  }

  const hostConfig = parsed as Record<string, unknown>;
  for (const key of ["bind_address", "daemon_url"] as const) {
    const value = hostConfig[key];
    if (value !== undefined && typeof value !== "string") {
      throw new Error(
        `Existing host configuration at ${path} has a non-string ${key}.`,
      );
    }
  }
}

export function writeCanonicalSetupFiles({
  agentConfigPath,
  agentConfigContent,
  hostConfigPath,
  hostConfigContent,
}: WriteCanonicalSetupFilesParams): WriteCanonicalSetupFilesResult {
  if (isExistingConfigFile(hostConfigPath)) {
    validateExistingHostConfig(hostConfigPath);
    writeConfigFile(agentConfigPath, agentConfigContent);
    return { hostConfigStatus: "preserved" };
  }

  writeConfigFile(hostConfigPath, hostConfigContent);
  writeConfigFile(agentConfigPath, agentConfigContent);
  return { hostConfigStatus: "created" };
}
