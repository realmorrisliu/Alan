import { spawn } from "node:child_process";
import { mkdtemp, mkdir, rm, stat, writeFile } from "node:fs/promises";
import { createServer } from "node:net";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const tuiRoot = resolve(scriptDir, "..");
const repoRoot = resolve(tuiRoot, "../..");
const standaloneBinary = resolve(tuiRoot, "dist/alan-tui");
const alanBinary = process.env.ALAN_CLI_PATH ?? resolve(repoRoot, "target/debug/alan");
const smokeProfileId = "chatgpt-smoke";

function sleep(ms) {
  return new Promise((resolveSleep) => setTimeout(resolveSleep, ms));
}

function buildEnv(extra, removals = []) {
  const env = { ...process.env, ...extra };
  for (const key of removals) {
    delete env[key];
  }
  return env;
}

async function requireFile(path, label) {
  let metadata;
  try {
    metadata = await stat(path);
  } catch {
    throw new Error(`${label} was not found at ${path}`);
  }

  if (!metadata.isFile()) {
    throw new Error(`${label} is not a file: ${path}`);
  }
}

async function writeSmokeConnectionProfile(homeDir) {
  const alanHomeDir = join(homeDir, ".alan");
  await mkdir(alanHomeDir, { recursive: true });
  await writeFile(
    join(alanHomeDir, "connections.toml"),
    `version = 1
default_profile = "${smokeProfileId}"

[credentials.chatgpt]
kind = "managed_oauth"
provider_family = "chatgpt"
label = "Smoke ChatGPT login"
backend = "alan_home_auth_json"

[profiles.${smokeProfileId}]
provider = "chatgpt"
credential_id = "chatgpt"
source = "smoke"

[profiles.${smokeProfileId}.settings]
base_url = "https://chatgpt.com/backend-api/codex"
model = "gpt-5.3-codex"
account_id = ""
`,
  );
}

async function findOpenPort() {
  return new Promise((resolvePort, rejectPort) => {
    const server = createServer();
    server.unref();
    server.on("error", rejectPort);
    server.listen(0, "127.0.0.1", () => {
      const address = server.address();
      server.close(() => {
        if (address && typeof address === "object") {
          resolvePort(address.port);
        } else {
          rejectPort(new Error("Failed to allocate a localhost port"));
        }
      });
    });
  });
}

function appendOutput(output, chunk) {
  output.value += chunk.toString();
  if (output.value.length > 20000) {
    output.value = output.value.slice(-20000);
  }
}

function tail(output) {
  return output.trim().split("\n").slice(-40).join("\n");
}

function spawnDaemon(baseUrl, port, homeDir) {
  const output = { value: "" };
  const env = buildEnv(
    {
      HOME: homeDir,
      BIND_ADDRESS: `127.0.0.1:${port}`,
      RUST_LOG: process.env.RUST_LOG ?? "warn",
    },
    ["ALAN_AGENTD_URL", "ALAN_CONFIG_PATH", "ALAN_TUI_SMOKE_WEBSOCKET", "ALAN_TUI_SMOKE_WORKSPACE"],
  );

  const child = spawn(alanBinary, ["daemon", "start", "--foreground"], {
    cwd: repoRoot,
    env,
    stdio: ["ignore", "pipe", "pipe"],
  });

  child.stdout?.on("data", (chunk) => appendOutput(output, chunk));
  child.stderr?.on("data", (chunk) => appendOutput(output, chunk));
  child.on("error", (error) => appendOutput(output, `${error.message}\n`));

  return { child, output, baseUrl };
}

async function waitForHealth(daemon) {
  for (let attempt = 0; attempt < 80; attempt++) {
    try {
      const response = await fetch(`${daemon.baseUrl}/health`, {
        signal: AbortSignal.timeout(1000),
      });
      if (response.ok) {
        return;
      }
    } catch {
      // The daemon may still be binding its socket.
    }

    if (daemon.child.exitCode !== null) {
      throw new Error(
        `Daemon exited before becoming healthy at ${daemon.baseUrl}.\n${tail(daemon.output.value)}`,
      );
    }
    await sleep(150);
  }

  throw new Error(
    `Daemon did not become healthy at ${daemon.baseUrl}.\n${tail(daemon.output.value)}`,
  );
}

async function runProcess(command, args, options, timeoutMs) {
  return new Promise((resolveRun, rejectRun) => {
    const child = spawn(command, args, {
      cwd: options.cwd,
      env: options.env,
      stdio: ["ignore", "pipe", "pipe"],
    });
    let output = "";
    let timedOut = false;

    const timer = setTimeout(() => {
      timedOut = true;
      child.kill("SIGKILL");
    }, timeoutMs);

    child.stdout?.on("data", (chunk) => {
      output += chunk.toString();
    });
    child.stderr?.on("data", (chunk) => {
      output += chunk.toString();
    });
    child.on("error", (error) => {
      clearTimeout(timer);
      rejectRun(error);
    });
    child.on("close", (code, signal) => {
      clearTimeout(timer);
      resolveRun({ code, signal, output, timedOut });
    });
  });
}

async function stopChild(child) {
  if (!child || child.exitCode !== null || child.killed) {
    return;
  }

  await new Promise((resolveStop) => {
    const timer = setTimeout(() => {
      child.kill("SIGKILL");
      resolveStop();
    }, 3000);
    child.once("close", () => {
      clearTimeout(timer);
      resolveStop();
    });
    child.kill("SIGTERM");
  });
}

async function main() {
  await requireFile(standaloneBinary, "Standalone TUI binary");
  await requireFile(alanBinary, "Alan daemon binary");

  const tempRoot = await mkdtemp(join(tmpdir(), "alan-tui-standalone-smoke-"));
  const homeDir = join(tempRoot, "home");
  const workspaceDir = join(tempRoot, "workspace");
  let daemon = null;

  try {
    await mkdir(homeDir, { recursive: true });
    await mkdir(workspaceDir, { recursive: true });
    await writeSmokeConnectionProfile(homeDir);

    const port = await findOpenPort();
    const baseUrl = `http://127.0.0.1:${port}`;
    daemon = spawnDaemon(baseUrl, port, homeDir);
    await waitForHealth(daemon);

    const result = await runProcess(
      standaloneBinary,
      [],
      {
        cwd: workspaceDir,
        env: buildEnv(
          {
            HOME: homeDir,
            ALAN_AGENTD_URL: baseUrl,
            ALAN_TUI_SMOKE_PROFILE: smokeProfileId,
            ALAN_TUI_SMOKE_WEBSOCKET: "1",
            ALAN_TUI_SMOKE_WORKSPACE: workspaceDir,
            TERM: process.env.TERM ?? "xterm-256color",
          },
          ["ALAN_CONFIG_PATH", "BIND_ADDRESS"],
        ),
      },
      15000,
    );

    if (result.timedOut) {
      throw new Error(`Standalone WebSocket smoke timed out.\n${tail(result.output)}`);
    }
    if (result.code !== 0) {
      throw new Error(
        `Standalone WebSocket smoke failed with code ${result.code ?? result.signal}.\n${tail(
          result.output,
        )}`,
      );
    }
    if (!result.output.includes("standalone websocket smoke ok:")) {
      throw new Error(`Standalone WebSocket smoke did not report success.\n${tail(result.output)}`);
    }

    console.log(tail(result.output));
  } finally {
    if (daemon) {
      await stopChild(daemon.child);
    }
    await rm(tempRoot, { recursive: true, force: true });
  }
}

try {
  await main();
} catch (error) {
  console.error(error instanceof Error ? error.message : String(error));
  process.exit(1);
}
