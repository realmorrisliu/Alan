import { describe, expect, test } from "bun:test";
import {
  buildConfirmationDetailRows,
  preferredConfirmationActionIndex,
} from "./confirmation-surface";

describe("confirmation surface helpers", () => {
  test("preferred action focuses approve when available", () => {
    expect(
      preferredConfirmationActionIndex(["reject", "approve", "modify"]),
    ).toBe(1);
    expect(preferredConfirmationActionIndex(["modify", "reject"])).toBe(0);
  });

  test("detail rows flatten common nested payloads", () => {
    expect(
      buildConfirmationDetailRows({
        path: "/tmp/file.txt",
        replay_tool_call: {
          name: "write_file",
          arguments: { path: "/tmp/file.txt", content: "hello" },
        },
        policy: {
          action: "escalate",
          capability: "write",
        },
      }),
    ).toEqual([
      { label: "path", value: "/tmp/file.txt", color: "cyan" },
      { label: "replay tool", value: "write_file", color: "cyan" },
      {
        label: "arguments",
        value: JSON.stringify(
          { path: "/tmp/file.txt", content: "hello" },
          null,
          2,
        ),
        color: "gray",
      },
      { label: "policy.action", value: "escalate" },
      { label: "policy.capability", value: "write" },
    ]);
  });
});
