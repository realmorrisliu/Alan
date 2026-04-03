import { afterEach, describe, expect, test } from "bun:test";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  clearShellBinding,
  readShellBindingTarget,
  writeShellBinding,
} from "./shell-binding.js";

let tempRoot: string | null = null;

afterEach(async () => {
  if (tempRoot) {
    await rm(tempRoot, { recursive: true, force: true });
    tempRoot = null;
  }
});

describe("shell binding", () => {
  test("reads shell binding target from environment", () => {
    const target = readShellBindingTarget({
      ALAN_SHELL_BINDING_FILE: "/tmp/alan-binding.json",
      ALAN_SHELL_WINDOW_ID: "window_main",
      ALAN_SHELL_SPACE_ID: "space_main",
      ALAN_SHELL_SURFACE_ID: "surface_main",
      ALAN_SHELL_PANE_ID: "pane_1",
    });

    expect(target).toEqual({
      filePath: "/tmp/alan-binding.json",
      windowId: "window_main",
      spaceId: "space_main",
      surfaceId: "surface_main",
      paneId: "pane_1",
    });
  });

  test("writes and clears shell binding payload", async () => {
    tempRoot = await mkdtemp(join(tmpdir(), "alan-shell-binding-"));
    const target = {
      filePath: join(tempRoot, "pane", "alan-binding.json"),
      windowId: "window_main",
      spaceId: "space_main",
      surfaceId: "surface_main",
      paneId: "pane_1",
    };

    await writeShellBinding(target, "sess_live", "yielded", true);
    const payload = JSON.parse(await readFile(target.filePath, "utf8"));

    expect(payload.session_id).toBe("sess_live");
    expect(payload.run_status).toBe("yielded");
    expect(payload.pending_yield).toBe(true);
    expect(payload.window_id).toBe("window_main");
    expect(payload.space_id).toBe("space_main");
    expect(payload.surface_id).toBe("surface_main");
    expect(payload.pane_id).toBe("pane_1");
    expect(typeof payload.last_projected_at).toBe("string");

    await clearShellBinding(target);
    await expect(readFile(target.filePath, "utf8")).rejects.toThrow();
  });

  test("serializes overlapping shell binding mutations by call order", async () => {
    tempRoot = await mkdtemp(join(tmpdir(), "alan-shell-binding-"));
    const target = {
      filePath: join(tempRoot, "pane", "alan-binding.json"),
      windowId: "window_main",
      spaceId: "space_main",
      surfaceId: "surface_main",
      paneId: "pane_1",
    };

    await Promise.all([
      writeShellBinding(target, "sess_one", "running", false),
      clearShellBinding(target),
      writeShellBinding(target, "sess_two", "yielded", true),
    ]);

    const payload = JSON.parse(await readFile(target.filePath, "utf8"));
    expect(payload.session_id).toBe("sess_two");
    expect(payload.run_status).toBe("yielded");
    expect(payload.pending_yield).toBe(true);
  });
});
