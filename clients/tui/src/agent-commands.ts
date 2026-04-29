import type { ChildRunTerminationMode } from "./types.js";

export type AgentCommand =
  | {
      kind: "inspect";
      childRunId: string;
    }
  | {
      kind: "terminate";
      childRunId: string;
      mode: ChildRunTerminationMode;
      reason: string;
    }
  | {
      kind: "invalid";
      usage: string;
    };

export function parseAgentCommand(args: string[]): AgentCommand {
  const action = args[0];
  if (!action) {
    return {
      kind: "invalid",
      usage: "Usage: /agent <id>",
    };
  }

  if (action === "terminate" || action === "kill") {
    const childRunId = args[1];
    if (!childRunId) {
      return {
        kind: "invalid",
        usage: `Usage: /agent ${action} <id> [reason]`,
      };
    }
    const mode = action === "kill" ? "forceful" : "graceful";
    const fallbackReason =
      mode === "forceful"
        ? "operator requested forceful child termination"
        : "operator requested child termination";
    return {
      kind: "terminate",
      childRunId,
      mode,
      reason: args.slice(2).join(" ").trim() || fallbackReason,
    };
  }

  return {
    kind: "inspect",
    childRunId: action,
  };
}
