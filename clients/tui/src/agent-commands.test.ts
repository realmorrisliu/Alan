import { describe, expect, test } from "bun:test";
import { parseAgentCommand } from "./agent-commands";

describe("agent command parsing", () => {
  test("parses child-run inspection", () => {
    expect(parseAgentCommand(["child-run-1"])).toEqual({
      kind: "inspect",
      childRunId: "child-run-1",
    });
  });

  test("parses graceful termination with reason", () => {
    expect(parseAgentCommand(["terminate", "child-run-1", "not", "needed"])).toEqual({
      kind: "terminate",
      childRunId: "child-run-1",
      mode: "graceful",
      reason: "not needed",
    });
  });

  test("parses forceful kill with default reason", () => {
    expect(parseAgentCommand(["kill", "child-run-1"])).toEqual({
      kind: "terminate",
      childRunId: "child-run-1",
      mode: "forceful",
      reason: "operator requested forceful child termination",
    });
  });

  test("reports usage for incomplete commands", () => {
    expect(parseAgentCommand([])).toEqual({
      kind: "invalid",
      usage: "Usage: /agent <id>",
    });
    expect(parseAgentCommand(["terminate"])).toEqual({
      kind: "invalid",
      usage: "Usage: /agent terminate <id> [reason]",
    });
  });
});
