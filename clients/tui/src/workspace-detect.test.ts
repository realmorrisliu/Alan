import { describe, expect, test } from "bun:test";
import { mkdtempSync, mkdirSync, rmSync, symlinkSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { detectWorkspaceDirFromCwd } from "./workspace-detect.js";

describe("workspace detection", () => {
  test("returns cwd when .alan is a directory", () => {
    const cwd = "/tmp/repo";
    const detected = detectWorkspaceDirFromCwd(
      cwd,
      (path) => path === "/tmp/repo/.alan",
    );
    expect(detected).toBe(cwd);
  });

  test("returns parent when cwd is .alan directory", () => {
    const cwd = "/tmp/repo/.alan";
    const detected = detectWorkspaceDirFromCwd(
      cwd,
      (path) => path === "/tmp/repo/.alan",
    );
    expect(detected).toBe("/tmp/repo");
  });

  test("ignores the global Alan home directory when cwd is home", () => {
    const detected = detectWorkspaceDirFromCwd(
      "/Users/test",
      (path) => path === "/Users/test/.alan",
      "/Users/test/.alan",
    );
    expect(detected).toBeUndefined();
  });

  test("ignores the global Alan home directory when cwd is .alan", () => {
    const detected = detectWorkspaceDirFromCwd(
      "/Users/test/.alan",
      (path) => path === "/Users/test/.alan",
      "/Users/test/.alan",
    );
    expect(detected).toBeUndefined();
  });

  test.skipIf(process.platform === "win32")(
    "ignores symlinked global Alan home when cwd uses canonical spelling",
    () => {
      const root = mkdtempSync(join(tmpdir(), "alan-workspace-detect-"));
      try {
        const realHome = join(root, "real-home");
        const linkedHome = join(root, "linked-home");
        mkdirSync(realHome);
        symlinkSync(realHome, linkedHome);
        mkdirSync(join(realHome, ".alan"));

        const detected = detectWorkspaceDirFromCwd(
          realHome,
          (path) => path === join(realHome, ".alan"),
          join(linkedHome, ".alan"),
        );

        expect(detected).toBeUndefined();
      } finally {
        rmSync(root, { force: true, recursive: true });
      }
    },
  );

  test("returns undefined when .alan exists but is not a directory", () => {
    const cwd = "/tmp/repo";
    const detected = detectWorkspaceDirFromCwd(cwd, () => false);
    expect(detected).toBeUndefined();
  });
});
