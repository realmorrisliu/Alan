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
  wroteHostConfig: boolean;
}

export function writeCanonicalSetupFiles({
  agentConfigPath,
  agentConfigContent,
  hostConfigPath,
  hostConfigContent,
}: WriteCanonicalSetupFilesParams): WriteCanonicalSetupFilesResult {
  mkdirSync(dirname(agentConfigPath), { recursive: true });
  writeFileSync(agentConfigPath, agentConfigContent, { mode: 0o600 });
  chmodSync(agentConfigPath, 0o600);

  if (isExistingConfigFile(hostConfigPath)) {
    return { wroteHostConfig: false };
  }

  mkdirSync(dirname(hostConfigPath), { recursive: true });
  writeFileSync(hostConfigPath, hostConfigContent, { mode: 0o600 });
  chmodSync(hostConfigPath, 0o600);

  return { wroteHostConfig: true };
}
