import { describe, expect, test } from "bun:test";
import {
  buildConfirmationDetailRows,
  preferredConfirmationActionIndex,
} from "./confirmation-surface";

describe("confirmation surface helpers", () => {
  test("preferred action respects explicit defaults before approve", () => {
    expect(
      preferredConfirmationActionIndex(
        ["reject", "approve", "modify"],
        "modify",
      ),
    ).toBe(2);
    expect(
      preferredConfirmationActionIndex(["reject", "approve", "modify"], "skip"),
    ).toBe(1);
    expect(preferredConfirmationActionIndex(["modify", "reject"])).toBe(0);
  });

  test("detail rows flatten runtime replay tool payloads", () => {
    expect(
      buildConfirmationDetailRows({
        path: "/tmp/file.txt",
        replay_tool_call: {
          call_id: "call-123",
          tool_name: "write_file",
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
      { label: "replay call id", value: "call-123", color: "cyan" },
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

  test("detail rows preserve extra replay tool fields and legacy name fallback", () => {
    expect(
      buildConfirmationDetailRows({
        replay_tool_call: {
          name: "edit_file",
          preview: "diff preview",
          attempts: 2,
        },
      }),
    ).toEqual([
      { label: "replay tool", value: "edit_file", color: "cyan" },
      { label: "replay tool attempts", value: "2" },
      {
        label: "replay tool preview",
        value: "diff preview",
        color: "gray",
      },
    ]);
  });

  test("detail rows prioritize common command and policy fields", () => {
    expect(
      buildConfirmationDetailRows({
        policy: { action: "escalate" },
        command: "rm -rf /tmp/demo",
        diff: "--- before\n+++ after",
      }),
    ).toEqual([
      { label: "command", value: "rm -rf /tmp/demo", color: "cyan" },
      { label: "diff", value: "--- before\n+++ after", color: "gray" },
      { label: "policy.action", value: "escalate" },
    ]);
  });
});
