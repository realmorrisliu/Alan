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
import { isExistingConfigFile } from "./config-path.js";
import { writeCanonicalSetupFiles } from "./init-files.js";

describe("writeCanonicalSetupFiles", () => {
  test("writes both agent and host config when host config is missing", () => {
    const tempRoot = mkdtempSync(join(tmpdir(), "alan-init-files-"));
    const agentConfigPath = join(
      tempRoot,
      ".alan",
      "agents",
      "default",
      "agent.toml",
    );
    const connectionsConfigPath = join(tempRoot, ".alan", "connections.toml");
    const hostConfigPath = join(tempRoot, ".alan", "host.toml");

    const result = writeCanonicalSetupFiles({
      agentConfigPath,
      agentConfigContent: "llm_request_timeout_secs = 180\n",
      connectionsConfigPath,
      connectionsConfigContent: 'version = 1\ndefault_profile = "openai-main"\n',
      globalPublicSkillsDir: join(tempRoot, ".agents", "skills"),
      hostConfigPath,
      hostConfigContent:
        'bind_address = "127.0.0.1:8090"\ndaemon_url = "http://127.0.0.1:8090"\n',
    });

    expect(result).toEqual({ hostConfigStatus: "created" });
    expect(readFileSync(agentConfigPath, "utf8")).not.toContain("connection_profile");
    expect(readFileSync(connectionsConfigPath, "utf8")).toContain(
      'default_profile = "openai-main"',
    );
    expect(readFileSync(hostConfigPath, "utf8")).toContain("bind_address");
    expect(statSync(join(tempRoot, ".agents", "skills")).isDirectory()).toBe(true);
    expect(statSync(agentConfigPath).mode & 0o777).toBe(0o600);
    expect(statSync(hostConfigPath).mode & 0o777).toBe(0o600);

    rmSync(tempRoot, { recursive: true, force: true });
  });

  test("preserves an existing host config file", () => {
    const tempRoot = mkdtempSync(join(tmpdir(), "alan-init-files-"));
    const agentConfigPath = join(
      tempRoot,
      ".alan",
      "agents",
      "default",
      "agent.toml",
    );
    const connectionsConfigPath = join(tempRoot, ".alan", "connections.toml");
    const hostConfigPath = join(tempRoot, ".alan", "host.toml");
    const existingHostConfig =
      'bind_address = "127.0.0.1:9123"\ndaemon_url = "http://127.0.0.1:9123"\n';

    mkdirSync(dirname(hostConfigPath), { recursive: true });
    writeFileSync(hostConfigPath, existingHostConfig, { mode: 0o600 });

    const result = writeCanonicalSetupFiles({
      agentConfigPath,
      agentConfigContent: "tool_timeout_secs = 30\n",
      connectionsConfigPath,
      connectionsConfigContent:
        'version = 1\ndefault_profile = "anthropic-main"\n',
      globalPublicSkillsDir: join(tempRoot, ".agents", "skills"),
      hostConfigPath,
      hostConfigContent:
        'bind_address = "127.0.0.1:8090"\ndaemon_url = "http://127.0.0.1:8090"\n',
    });

    expect(result).toEqual({ hostConfigStatus: "preserved" });
    expect(readFileSync(agentConfigPath, "utf8")).toContain("tool_timeout_secs");
    expect(readFileSync(connectionsConfigPath, "utf8")).toContain(
      'default_profile = "anthropic-main"',
    );
    expect(readFileSync(hostConfigPath, "utf8")).toBe(existingHostConfig);

    rmSync(tempRoot, { recursive: true, force: true });
  });

  test("fails before writing agent config when existing host config is invalid", () => {
    const tempRoot = mkdtempSync(join(tmpdir(), "alan-init-files-"));
    const agentConfigPath = join(
      tempRoot,
      ".alan",
      "agents",
      "default",
      "agent.toml",
    );
    const connectionsConfigPath = join(tempRoot, ".alan", "connections.toml");
    const hostConfigPath = join(tempRoot, ".alan", "host.toml");

    mkdirSync(dirname(hostConfigPath), { recursive: true });
    writeFileSync(hostConfigPath, "bind_address = 8090\n", { mode: 0o600 });

    expect(() =>
      writeCanonicalSetupFiles({
        agentConfigPath,
        agentConfigContent: "tool_timeout_secs = 30\n",
        connectionsConfigPath,
        connectionsConfigContent:
          'version = 1\ndefault_profile = "anthropic-main"\n',
        globalPublicSkillsDir: join(tempRoot, ".agents", "skills"),
        hostConfigPath,
        hostConfigContent:
          'bind_address = "127.0.0.1:8090"\ndaemon_url = "http://127.0.0.1:8090"\n',
      }),
    ).toThrow(
      `Existing host configuration at ${hostConfigPath} has a non-string bind_address.`,
    );

    expect(isExistingConfigFile(agentConfigPath)).toBe(false);
    expect(readFileSync(hostConfigPath, "utf8")).toBe("bind_address = 8090\n");

    rmSync(tempRoot, { recursive: true, force: true });
  });

  test("fails before writing agent config when host config is unavailable", () => {
    const tempRoot = mkdtempSync(join(tmpdir(), "alan-init-files-"));
    const agentConfigPath = join(tempRoot, "agent", "agent.toml");
    const connectionsConfigPath = join(tempRoot, "connections.toml");
    const blockedHostRoot = join(tempRoot, "blocked-host-root");
    const hostConfigPath = join(blockedHostRoot, "host.toml");

    writeFileSync(blockedHostRoot, "not-a-directory", { mode: 0o600 });

    expect(() =>
      writeCanonicalSetupFiles({
        agentConfigPath,
        agentConfigContent: "llm_request_timeout_secs = 180\n",
        connectionsConfigPath,
        connectionsConfigContent: 'version = 1\ndefault_profile = "openai-main"\n',
        globalPublicSkillsDir: join(tempRoot, ".agents", "skills"),
        hostConfigPath,
        hostConfigContent:
          'bind_address = "127.0.0.1:8090"\ndaemon_url = "http://127.0.0.1:8090"\n',
      }),
    ).toThrow(`Failed to write configuration file at ${hostConfigPath}`);

    expect(isExistingConfigFile(agentConfigPath)).toBe(false);
    expect(isExistingConfigFile(hostConfigPath)).toBe(false);

    rmSync(tempRoot, { recursive: true, force: true });
  });
});
