import { describe, expect, test } from "bun:test";
import {
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { writeCanonicalSetupFiles } from "./init-files.js";

describe("writeCanonicalSetupFiles", () => {
  test("writes both agent and host config when host config is missing", () => {
    const tempRoot = mkdtempSync(join(tmpdir(), "alan-init-files-"));
    const agentConfigPath = join(tempRoot, ".alan", "agent", "agent.toml");
    const hostConfigPath = join(tempRoot, ".alan", "host.toml");

    const result = writeCanonicalSetupFiles({
      agentConfigPath,
      agentConfigContent: 'llm_provider = "openai_responses"\n',
      hostConfigPath,
      hostConfigContent:
        'bind_address = "127.0.0.1:8090"\ndaemon_url = "http://127.0.0.1:8090"\n',
    });

    expect(result).toEqual({ wroteHostConfig: true });
    expect(readFileSync(agentConfigPath, "utf8")).toContain("llm_provider");
    expect(readFileSync(hostConfigPath, "utf8")).toContain("bind_address");
    expect(statSync(agentConfigPath).mode & 0o777).toBe(0o600);
    expect(statSync(hostConfigPath).mode & 0o777).toBe(0o600);

    rmSync(tempRoot, { recursive: true, force: true });
  });

  test("preserves an existing host config file", () => {
    const tempRoot = mkdtempSync(join(tmpdir(), "alan-init-files-"));
    const agentConfigPath = join(tempRoot, ".alan", "agent", "agent.toml");
    const hostConfigPath = join(tempRoot, ".alan", "host.toml");
    const existingHostConfig =
      'bind_address = "127.0.0.1:9123"\ndaemon_url = "http://127.0.0.1:9123"\n';

    mkdirSync(dirname(hostConfigPath), { recursive: true });
    writeFileSync(hostConfigPath, existingHostConfig, { mode: 0o600 });

    const result = writeCanonicalSetupFiles({
      agentConfigPath,
      agentConfigContent: 'llm_provider = "anthropic_messages"\n',
      hostConfigPath,
      hostConfigContent:
        'bind_address = "127.0.0.1:8090"\ndaemon_url = "http://127.0.0.1:8090"\n',
    });

    expect(result).toEqual({ wroteHostConfig: false });
    expect(readFileSync(agentConfigPath, "utf8")).toContain("anthropic_messages");
    expect(readFileSync(hostConfigPath, "utf8")).toBe(existingHostConfig);

    rmSync(tempRoot, { recursive: true, force: true });
  });
});
