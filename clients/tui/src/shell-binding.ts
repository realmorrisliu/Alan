import { mkdir, rename, rm, writeFile } from "node:fs/promises";
import { dirname } from "node:path";

export interface ShellBindingTarget {
  filePath: string;
  windowId?: string;
  spaceId?: string;
  tabId?: string;
  paneId?: string;
}

interface ShellBindingPayload {
  session_id: string;
  run_status: string;
  pending_yield: boolean;
  source: string;
  last_projected_at: string;
  window_id?: string;
  space_id?: string;
  tab_id?: string;
  pane_id?: string;
}

const pendingShellBindingMutations = new Map<string, Promise<void>>();

export function readShellBindingTarget(
  env: NodeJS.ProcessEnv,
): ShellBindingTarget | null {
  const filePath = env.ALAN_SHELL_BINDING_FILE?.trim();
  if (!filePath) {
    return null;
  }

  return {
    filePath,
    windowId: env.ALAN_SHELL_WINDOW_ID?.trim() || undefined,
    spaceId: env.ALAN_SHELL_SPACE_ID?.trim() || undefined,
    tabId: env.ALAN_SHELL_TAB_ID?.trim() || undefined,
    paneId: env.ALAN_SHELL_PANE_ID?.trim() || undefined,
  };
}

export async function writeShellBinding(
  target: ShellBindingTarget,
  sessionId: string,
  runStatus: string,
  pendingYield: boolean,
): Promise<void> {
  await enqueueShellBindingMutation(target, async () => {
    const payload: ShellBindingPayload = {
      session_id: sessionId,
      run_status: runStatus,
      pending_yield: pendingYield,
      source: "alan_tui",
      last_projected_at: new Date().toISOString(),
      window_id: target.windowId,
      space_id: target.spaceId,
      tab_id: target.tabId,
      pane_id: target.paneId,
    };
    const tempPath = `${target.filePath}.tmp`;
    await mkdir(dirname(target.filePath), { recursive: true });
    await writeFile(tempPath, JSON.stringify(payload, null, 2), "utf8");
    await rename(tempPath, target.filePath);
  });
}

export async function clearShellBinding(
  target: ShellBindingTarget | null,
): Promise<void> {
  if (!target?.filePath) {
    return;
  }
  await enqueueShellBindingMutation(target, async () => {
    await rm(target.filePath, { force: true });
  });
}

async function enqueueShellBindingMutation(
  target: ShellBindingTarget,
  mutation: () => Promise<void>,
): Promise<void> {
  const key = target.filePath;
  const previous = pendingShellBindingMutations.get(key) ?? Promise.resolve();
  const next = previous.catch(() => undefined).then(mutation);
  pendingShellBindingMutations.set(key, next);

  try {
    await next;
  } finally {
    if (pendingShellBindingMutations.get(key) === next) {
      pendingShellBindingMutations.delete(key);
    }
  }
}
