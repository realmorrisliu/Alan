import { spawn } from "node:child_process";

export interface BrowserOpenCommand {
  command: string;
  args: string[];
}

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

export function resolveBrowserOpenCommand(
  url: string,
  platform: NodeJS.Platform = process.platform,
  browserOverride = process.env.BROWSER?.trim(),
): BrowserOpenCommand {
  if (browserOverride) {
    return { command: browserOverride, args: [url] };
  }

  switch (platform) {
    case "darwin":
      return { command: "open", args: [url] };
    case "win32":
      return {
        command: "rundll32",
        args: ["url.dll,FileProtocolHandler", url],
      };
    default:
      return { command: "xdg-open", args: [url] };
  }
}

export async function openUrlInBrowser(url: string): Promise<void> {
  const { command, args } = resolveBrowserOpenCommand(url);
  await spawnDetached(command, args);
}
