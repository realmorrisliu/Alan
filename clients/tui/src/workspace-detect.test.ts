import { describe, expect, test } from "bun:test";
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

  test("returns undefined when .alan exists but is not a directory", () => {
    const cwd = "/tmp/repo";
    const detected = detectWorkspaceDirFromCwd(cwd, () => false);
    expect(detected).toBeUndefined();
  });
});
