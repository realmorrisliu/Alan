import { chmodSync, mkdirSync, writeFileSync } from "node:fs";
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

export function writeCanonicalSetupFiles({
  agentConfigPath,
  agentConfigContent,
  hostConfigPath,
  hostConfigContent,
}: WriteCanonicalSetupFilesParams): WriteCanonicalSetupFilesResult {
  if (isExistingConfigFile(hostConfigPath)) {
    writeConfigFile(agentConfigPath, agentConfigContent);
    return { hostConfigStatus: "preserved" };
  }

  writeConfigFile(hostConfigPath, hostConfigContent);
  writeConfigFile(agentConfigPath, agentConfigContent);
  return { hostConfigStatus: "created" };
}
