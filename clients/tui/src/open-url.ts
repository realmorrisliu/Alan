import { spawn } from "node:child_process";

export interface BrowserOpenCommand {
  command: string;
  args: string[];
}

function splitCommandLine(commandLine: string): string[] {
  const tokens: string[] = [];
  let current = "";
  let quote: "'" | '"' | null = null;
  let escaping = false;

  for (const [index, character] of Array.from(commandLine).entries()) {
    const nextCharacter = commandLine[index + 1];

    if (escaping) {
      current += character;
      escaping = false;
      continue;
    }

    if (character === "\\") {
      if (quote === "'") {
        current += character;
        continue;
      }

      if (quote === '"') {
        if (nextCharacter === '"' || nextCharacter === "\\") {
          escaping = true;
        } else {
          current += character;
        }
        continue;
      }

      if (
        nextCharacter &&
        (/\s/.test(nextCharacter) ||
          nextCharacter === '"' ||
          nextCharacter === "'" ||
          nextCharacter === "\\")
      ) {
        escaping = true;
      } else {
        current += character;
      }
      continue;
    }

    if (quote) {
      if (character === quote) {
        quote = null;
      } else {
        current += character;
      }
      continue;
    }

    if (character === "'" || character === '"') {
      quote = character;
      continue;
    }

    if (/\s/.test(character)) {
      if (current) {
        tokens.push(current);
        current = "";
      }
      continue;
    }

    current += character;
  }

  if (escaping) {
    current += "\\";
  }

  if (quote) {
    return [commandLine];
  }

  if (current) {
    tokens.push(current);
  }

  return tokens;
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
    const [command, ...args] = splitCommandLine(browserOverride);
    if (command) {
      return { command, args: [...args, url] };
    }
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
