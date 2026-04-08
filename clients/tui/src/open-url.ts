import { spawn } from "node:child_process";

function spawnDetached(command: string, args: string[]): Promise<void> {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, {
      detached: true,
      stdio: "ignore",
    });
    child.once("error", reject);
    child.once("spawn", () => {
      child.unref();
      resolve();
    });
  });
}

export async function openUrlInBrowser(url: string): Promise<void> {
  const browserOverride = process.env.BROWSER?.trim();
  if (browserOverride) {
    await spawnDetached(browserOverride, [url]);
    return;
  }

  switch (process.platform) {
    case "darwin":
      await spawnDetached("open", [url]);
      return;
    case "win32":
      await spawnDetached("cmd", ["/c", "start", "", url]);
      return;
    default:
      await spawnDetached("xdg-open", [url]);
  }
}
