#!/usr/bin/env bun
/**
 * Alan TUI entrypoint.
 */

import React, { useEffect, useRef, useState } from "react";
import { render, Box, Text, useApp, useInput } from "ink";
import TextInput from "ink-text-input";
import { homedir } from "node:os";
import { AlanClient } from "./client.js";
import {
  isExistingConfigFile,
  resolveConfigPathCandidates,
  selectExistingConfigPath,
  shouldRunFirstTimeSetup,
} from "./config-path.js";
import { detectWorkspaceDirFromCwd } from "./workspace-detect.js";
import type { DaemonStatus, EventEnvelope } from "./types.js";
import { MessageList } from "./components.js";
import { InitWizard } from "./init.js";
import {
  confirmationOptions,
  confirmationSummary,
  normalizeYieldKind,
  structuredPrompt,
  structuredQuestions,
  structuredTitle,
} from "./yield.js";

const AGENTD_URL = process.env.ALAN_AGENTD_URL;
const AUTO_MANAGE = !AGENTD_URL;
const VERBOSE = process.env.ALAN_VERBOSE === "1";
const MAX_EVENT_HISTORY = 2000;

function displayPath(path: string): string {
  const home = homedir();
  if (path === home) {
    return "~";
  }
  const homePrefix = `${home}/`;
  return path.startsWith(homePrefix)
    ? `~/${path.slice(homePrefix.length)}`
    : path;
}

const CONFIG_PATH_CANDIDATES = resolveConfigPathCandidates(
  homedir(),
  process.env,
);
const CONFIG_PATH =
  selectExistingConfigPath(CONFIG_PATH_CANDIDATES, isExistingConfigFile) ??
  CONFIG_PATH_CANDIDATES[0];
const CONFIG_PATH_DISPLAY = displayPath(CONFIG_PATH);
const CONFIG_PATH_HINT =
  CONFIG_PATH_CANDIDATES.length === 1
    ? CONFIG_PATH_DISPLAY
    : `${displayPath(CONFIG_PATH_CANDIDATES[0])}（fallback: ${displayPath(
        CONFIG_PATH_CANDIDATES[1],
      )}）`;

const STARTUP_INFO = {
  mode: AGENTD_URL ? "remote" : ("embedded" as const),
  url: AGENTD_URL || "ws://127.0.0.1:8090",
};

type PendingYieldKind =
  | "confirmation"
  | "structured_input"
  | "dynamic_tool"
  | "custom";

interface PendingYield {
  requestId: string;
  kind: PendingYieldKind;
  payload: unknown;
}

function needsFirstTimeSetup(): boolean {
  if (AGENTD_URL) return false;
  return shouldRunFirstTimeSetup(CONFIG_PATH_CANDIDATES, isExistingConfigFile);
}

function shortId(value: string | null | undefined): string {
  if (!value) return "-";
  return value.slice(0, 8);
}

function parsePendingYieldKind(kind: unknown): PendingYieldKind {
  const normalized = normalizeYieldKind(kind as any);
  if (normalized === "confirmation") return "confirmation";
  if (normalized === "structured_input") return "structured_input";
  if (normalized === "dynamic_tool") return "dynamic_tool";
  return "custom";
}

function parseGovernanceProfile(
  input: string | undefined,
): "autonomous" | "conservative" | null {
  if (!input) return null;
  const value = input.trim().toLowerCase();
  if (value === "autonomous" || value === "conservative") {
    return value;
  }
  return null;
}

function parseStreamingMode(
  input: string | undefined,
): "auto" | "on" | "off" | null {
  if (!input) return null;
  const value = input.trim().toLowerCase();
  if (value === "auto" || value === "on" || value === "off") {
    return value;
  }
  return null;
}

function parsePartialStreamRecoveryMode(
  input: string | undefined,
): "continue_once" | "off" | null {
  if (!input) return null;
  const value = input.trim().toLowerCase();
  if (value === "continue_once") {
    return "continue_once";
  }
  if (value.startsWith("recovery=")) {
    const mode = value.slice("recovery=".length);
    if (mode === "continue_once" || mode === "off") {
      return mode;
    }
  }
  if (value.startsWith("partial_stream_recovery_mode=")) {
    const mode = value.slice("partial_stream_recovery_mode=".length);
    if (mode === "continue_once" || mode === "off") {
      return mode;
    }
  }
  return null;
}

function structuredAnswersTemplate(payload: unknown): string {
  const questions = structuredQuestions(payload);
  if (questions.length === 0) return "[]";

  const template = questions.map((q) => ({
    question_id: q.id,
    value:
      q.options && q.options.length > 0
        ? q.options[0].value
        : q.required
          ? "<required-value>"
          : "",
  }));

  return JSON.stringify(template);
}

function App() {
  const { exit } = useApp();

  const [needsSetup, setNeedsSetup] = useState(needsFirstTimeSetup());
  const [inputValue, setInputValue] = useState("");
  const [status, setStatus] = useState<"connecting" | "connected" | "error">(
    "connecting",
  );
  const [statusMessage, setStatusMessage] = useState("Starting...");
  const [currentSessionId, setCurrentSessionId] = useState<string | null>(null);
  const [events, setEvents] = useState<EventEnvelope[]>([]);
  const [daemonStatus, setDaemonStatus] = useState<DaemonStatus | null>(null);
  const [pendingYield, setPendingYield] = useState<PendingYield | null>(null);

  const clientRef = useRef<AlanClient | null>(null);
  const sessionIdRef = useRef<string>("");

  useEffect(() => {
    sessionIdRef.current = currentSessionId ?? "";
  }, [currentSessionId]);

  const pushEvent = (event: EventEnvelope) => {
    setEvents((prev) => {
      const base =
        prev.length >= MAX_EVENT_HISTORY
          ? prev.slice(prev.length - MAX_EVENT_HISTORY + 1)
          : prev;
      return [...base, event];
    });
  };

  const addSystemEvent = (type: EventEnvelope["type"], message: string) => {
    pushEvent({
      event_id: crypto.randomUUID(),
      sequence: 0,
      session_id: sessionIdRef.current,
      turn_id: "system",
      item_id: "system",
      timestamp_ms: Date.now(),
      type,
      message,
    } as EventEnvelope);
  };

  const announceYield = (incoming: PendingYield) => {
    if (incoming.kind === "confirmation") {
      const summary = confirmationSummary(incoming.payload);
      const options = confirmationOptions(incoming.payload);

      addSystemEvent(
        "system_warning",
        `Approval required (${incoming.requestId}).`,
      );
      if (summary) {
        addSystemEvent("system_message", summary);
      }
      if (options.length > 0) {
        addSystemEvent("system_message", `Options: ${options.join(", ")}`);
      }
      addSystemEvent(
        "system_message",
        "Use /approve, /reject, or /modify <text>.",
      );
      return;
    }

    if (incoming.kind === "structured_input") {
      const title = structuredTitle(incoming.payload);
      const prompt = structuredPrompt(incoming.payload);
      const questions = structuredQuestions(incoming.payload);

      addSystemEvent(
        "system_warning",
        `Input required (${incoming.requestId}).`,
      );
      if (title) {
        addSystemEvent("system_message", `Title: ${title}`);
      }
      if (prompt) {
        addSystemEvent("system_message", prompt);
      }
      addSystemEvent("system_message", `Questions: ${questions.length}`);
      if (questions.length === 1) {
        addSystemEvent(
          "system_message",
          `Use /answer <value> for ${questions[0].id}.`,
        );
      } else {
        addSystemEvent(
          "system_message",
          `Use /answers '${structuredAnswersTemplate(incoming.payload)}'`,
        );
      }
      return;
    }

    if (incoming.kind === "dynamic_tool") {
      addSystemEvent(
        "system_warning",
        `Dynamic tool call pending (${incoming.requestId}).`,
      );
      addSystemEvent(
        "system_message",
        "Use /resume <json> to return a custom content payload.",
      );
      return;
    }

    addSystemEvent(
      "system_warning",
      `Custom yield pending (${incoming.requestId}).`,
    );
    addSystemEvent(
      "system_message",
      "Use /resume <json> to continue with a custom payload.",
    );
  };

  const handleSetupComplete = () => {
    setNeedsSetup(false);
  };

  useEffect(() => {
    if (needsSetup) {
      return;
    }

    const client = new AlanClient({
      url: STARTUP_INFO.url,
      autoManageDaemon: AUTO_MANAGE,
      verbose: VERBOSE,
    });
    clientRef.current = client;

    client.on("connected", () => {
      setStatus("connected");
      setStatusMessage(
        STARTUP_INFO.mode === "embedded"
          ? "Ready"
          : `Connected to ${STARTUP_INFO.url}`,
      );
    });

    client.on("disconnected", () => {
      setStatus("error");
      setStatusMessage("Disconnected");
      addSystemEvent("system_message", "Disconnected from agent");
    });

    client.on("error", (error: Error) => {
      setStatus("error");
      setStatusMessage(`Error: ${error.message}`);
      addSystemEvent("system_error", error.message);
    });

    client.on("event", (envelope: EventEnvelope) => {
      pushEvent(envelope);

      if (envelope.type === "yield" && envelope.request_id) {
        const incoming: PendingYield = {
          requestId: envelope.request_id,
          kind: parsePendingYieldKind(envelope.kind),
          payload: envelope.payload,
        };
        setPendingYield(incoming);
        announceYield(incoming);
      }
    });

    client.on("session_created", (sessionId: string) => {
      setCurrentSessionId(sessionId);
      addSystemEvent("session_created", sessionId);
    });

    const detectWorkspaceDir = (): string | undefined => {
      const cwd = process.cwd();
      const workspaceDir = detectWorkspaceDirFromCwd(cwd);
      if (workspaceDir) {
        addSystemEvent("system_message", `Detected workspace: ${workspaceDir}`);
        return workspaceDir;
      }

      return undefined;
    };

    const init = async () => {
      try {
        setStatusMessage(
          STARTUP_INFO.mode === "embedded"
            ? "Starting daemon..."
            : "Connecting to agent...",
        );

        await client.connect();

        if (AUTO_MANAGE) {
          const latestStatus = client.getDaemonStatus();
          if (latestStatus) {
            setDaemonStatus(latestStatus);
          }
        }

        try {
          const workspaceDir = detectWorkspaceDir();

          if (workspaceDir) {
            addSystemEvent(
              "system_message",
              `Creating session for workspace: ${workspaceDir}...`,
            );
          } else {
            addSystemEvent(
              "system_message",
              "Auto-creating session on default workspace...",
            );
          }

          const session = await client.createSession({
            workspace_dir: workspaceDir,
          });
          const sessionId = session.session_id;
          setCurrentSessionId(sessionId);
          await client.connectToSession(sessionId);
          addSystemEvent(
            "system_message",
            `Alan ready. Type your request directly or /help. (streaming=${session.streaming_mode}, recovery=${session.partial_stream_recovery_mode})`,
          );
        } catch (error) {
          const msg = (error as Error).message;
          addSystemEvent("system_error", msg);

          if (
            msg.includes("LLM") ||
            msg.includes("llm") ||
            msg.includes("model") ||
            msg.includes("key")
          ) {
            addSystemEvent(
              "system_message",
              "Hint: this looks like an LLM configuration issue.",
            );
            addSystemEvent(
              "system_message",
              `Please check ${CONFIG_PATH_HINT} (or ALAN_CONFIG_PATH).`,
            );
          } else if (
            msg.includes("500") ||
            msg.includes("Internal Server Error")
          ) {
            addSystemEvent(
              "system_message",
              "Hint: daemon internal error, please check daemon logs.",
            );
          }
          addSystemEvent(
            "system_message",
            "Type /new to try again, or /help for commands.",
          );
        }
      } catch (error) {
        const message = (error as Error).message;
        setStatus("error");
        setStatusMessage(`Failed: ${message}`);
        addSystemEvent("system_error", `Connection failed: ${message}`);

        if (STARTUP_INFO.mode === "embedded") {
          addSystemEvent(
            "system_message",
            "Make sure `alan` is installed and on PATH (try: just install).",
          );
        }
      }
    };

    init();

    const cleanup = async () => {
      await client.shutdown();
    };

    const handleExit = () => {
      cleanup().then(() => {
        process.exit(0);
      });
    };

    process.on("SIGINT", handleExit);
    process.on("SIGTERM", handleExit);

    return () => {
      process.off("SIGINT", handleExit);
      process.off("SIGTERM", handleExit);
      void cleanup();
    };
  }, [needsSetup]);

  useInput((input, key) => {
    if (key.ctrl && input === "c") {
      exit();
      return;
    }

    if (key.ctrl && input === "l") {
      setEvents([]);
      addSystemEvent("system_message", "Timeline cleared.");
      return;
    }
  });

  const submitPendingYield = async (content: unknown) => {
    const client = clientRef.current;
    if (!client || !currentSessionId || !pendingYield) {
      addSystemEvent("system_warning", "No pending yield to resume.");
      return;
    }

    try {
      await client.resume(currentSessionId, pendingYield.requestId, content);
      addSystemEvent(
        "system_message",
        `Resumed ${pendingYield.kind} (${pendingYield.requestId}).`,
      );
      setPendingYield(null);
    } catch (error) {
      addSystemEvent(
        "system_error",
        `Failed to resume: ${(error as Error).message}`,
      );
    }
  };

  const handleSubmit = async (text: string) => {
    const client = clientRef.current;
    if (!client) return;

    const trimmed = text.trim();
    if (!trimmed) return;

    setInputValue("");
    addSystemEvent("user_message", trimmed);

    if (trimmed.startsWith("/")) {
      await handleCommand(trimmed, client);
      return;
    }

    if (!currentSessionId) {
      addSystemEvent(
        "system_warning",
        "No active session. Use /new to create one.",
      );
      return;
    }

    if (pendingYield) {
      addSystemEvent(
        "system_warning",
        "Yield is pending. Resolve it first (/approve, /reject, /modify, /answer, /answers, /resume).",
      );
      return;
    }

    try {
      await client.sendMessage(currentSessionId, trimmed);
    } catch (error) {
      addSystemEvent(
        "system_error",
        `Failed to send: ${(error as Error).message}`,
      );
    }
  };

  const handleCommand = async (text: string, client: AlanClient) => {
    const [rawCmd, ...args] = text.slice(1).split(" ");
    const cmd = rawCmd.toLowerCase();

    switch (cmd) {
      case "new": {
        let requestedProfile: "autonomous" | "conservative" | null = null;
        let requestedStreaming: "auto" | "on" | "off" | null = null;
        let requestedRecovery: "continue_once" | "off" | null = null;

        for (const arg of args.filter(Boolean)) {
          const profile = parseGovernanceProfile(arg);
          if (profile) {
            if (requestedProfile && requestedProfile !== profile) {
              addSystemEvent(
                "system_warning",
                "Usage: /new [autonomous|conservative] [auto|on|off] [continue_once|recovery=off]",
              );
              return;
            }
            requestedProfile = profile;
            continue;
          }

          const streaming = parseStreamingMode(arg);
          if (streaming) {
            if (requestedStreaming && requestedStreaming !== streaming) {
              addSystemEvent(
                "system_warning",
                "Usage: /new [autonomous|conservative] [auto|on|off] [continue_once|recovery=off]",
              );
              return;
            }
            requestedStreaming = streaming;
            continue;
          }

          const recovery = parsePartialStreamRecoveryMode(arg);
          if (recovery) {
            if (requestedRecovery && requestedRecovery !== recovery) {
              addSystemEvent(
                "system_warning",
                "Usage: /new [autonomous|conservative] [auto|on|off] [continue_once|recovery=off]",
              );
              return;
            }
            requestedRecovery = recovery;
            continue;
          }

          addSystemEvent(
            "system_warning",
            "Usage: /new [autonomous|conservative] [auto|on|off] [continue_once|recovery=off]",
          );
          return;
        }

        try {
          addSystemEvent("system_message", "Creating new session...");
          const createRequest: {
            governance?: { profile: "autonomous" | "conservative" };
            streaming_mode?: "auto" | "on" | "off";
            partial_stream_recovery_mode?: "continue_once" | "off";
          } = {};
          if (requestedProfile) {
            createRequest.governance = { profile: requestedProfile };
          }
          if (requestedStreaming) {
            createRequest.streaming_mode = requestedStreaming;
          }
          if (requestedRecovery) {
            createRequest.partial_stream_recovery_mode = requestedRecovery;
          }

          const session = await client.createSession(
            Object.keys(createRequest).length > 0 ? createRequest : undefined,
          );
          const sessionId = session.session_id;
          setCurrentSessionId(sessionId);
          setPendingYield(null);
          await client.connectToSession(sessionId);
          addSystemEvent(
            "system_message",
            `Session ready (${shortId(sessionId)}), governance=${session.governance.profile}, streaming=${session.streaming_mode}, recovery=${session.partial_stream_recovery_mode}.`,
          );
        } catch (error) {
          const msg = (error as Error).message;
          addSystemEvent("system_error", msg);

          if (
            msg.includes("LLM") ||
            msg.includes("llm") ||
            msg.includes("model") ||
            msg.includes("key")
          ) {
            addSystemEvent(
              "system_message",
              "Hint: this looks like an LLM configuration issue.",
            );
            addSystemEvent(
              "system_message",
              `Please check ${CONFIG_PATH_HINT} (or ALAN_CONFIG_PATH).`,
            );
          } else if (
            msg.includes("500") ||
            msg.includes("Internal Server Error")
          ) {
            addSystemEvent(
              "system_message",
              "Hint: daemon internal error, please check daemon logs.",
            );
          }
        }
        break;
      }

      case "connect":
        if (!args[0]) {
          addSystemEvent("system_warning", "Usage: /connect <session-id>");
          return;
        }
        try {
          addSystemEvent(
            "system_message",
            `Connecting to session ${shortId(args[0])}...`,
          );
          setCurrentSessionId(args[0]);
          setPendingYield(null);
          await client.connectToSession(args[0]);
          addSystemEvent("system_message", "Connected");
        } catch (error) {
          addSystemEvent(
            "system_error",
            `Failed to connect: ${(error as Error).message}`,
          );
        }
        break;

      case "sessions":
        try {
          const sessions = await client.listSessions();
          addSystemEvent(
            "system_message",
            `Active sessions: ${sessions.length}`,
          );
          sessions.forEach((s) => {
            addSystemEvent(
              "system_message",
              `  ${shortId(s.session_id)} | ${s.active ? "active" : "inactive"} | ${s.governance.profile} | streaming=${s.streaming_mode} | recovery=${s.partial_stream_recovery_mode} | workspace=${s.workspace_id}`,
            );
          });
        } catch (error) {
          addSystemEvent(
            "system_error",
            `Failed to list sessions: ${(error as Error).message}`,
          );
        }
        break;

      case "status":
        if (AUTO_MANAGE) {
          const latestStatus = client.getDaemonStatus() || daemonStatus;
          if (latestStatus) {
            addSystemEvent(
              "system_message",
              `Daemon: ${latestStatus.state}${latestStatus.pid ? ` (pid: ${latestStatus.pid})` : ""}`,
            );
          } else {
            const running = await client.isDaemonRunning();
            addSystemEvent(
              "system_message",
              `Daemon: ${running ? "running" : "stopped"}`,
            );
          }
        } else {
          const running = await client.isDaemonRunning();
          addSystemEvent(
            "system_message",
            `Remote agent: ${running ? "online" : "offline"}`,
          );
        }
        break;

      case "input": {
        if (!currentSessionId) {
          addSystemEvent("system_warning", "No active session.");
          return;
        }

        const message = args.join(" ").trim();
        if (!message) {
          addSystemEvent("system_warning", "Usage: /input <text>");
          return;
        }

        try {
          await client.sendInput(currentSessionId, message);
          addSystemEvent("system_message", "Input appended to active turn.");
        } catch (error) {
          addSystemEvent(
            "system_error",
            `Failed to append input: ${(error as Error).message}`,
          );
        }
        break;
      }

      case "interrupt": {
        if (!currentSessionId) {
          addSystemEvent("system_warning", "No active session.");
          return;
        }

        try {
          await client.interrupt(currentSessionId);
          addSystemEvent("system_message", "Interrupt requested.");
          setPendingYield(null);
        } catch (error) {
          addSystemEvent(
            "system_error",
            `Failed to interrupt: ${(error as Error).message}`,
          );
        }
        break;
      }

      case "compact": {
        if (!currentSessionId) {
          addSystemEvent("system_warning", "No active session.");
          return;
        }

        try {
          await client.compact(currentSessionId);
          addSystemEvent("system_message", "Compaction requested.");
        } catch (error) {
          addSystemEvent(
            "system_error",
            `Failed to compact: ${(error as Error).message}`,
          );
        }
        break;
      }

      case "rollback": {
        if (!currentSessionId) {
          addSystemEvent("system_warning", "No active session.");
          return;
        }

        const turnsRaw = args[0];
        const turns = Number(turnsRaw);
        if (
          !turnsRaw ||
          Number.isNaN(turns) ||
          turns < 1 ||
          !Number.isInteger(turns)
        ) {
          addSystemEvent(
            "system_warning",
            "Usage: /rollback <positive-integer>",
          );
          return;
        }

        try {
          await client.rollback(currentSessionId, turns);
          addSystemEvent(
            "system_message",
            `Rollback requested for ${turns} turn(s).`,
          );
        } catch (error) {
          addSystemEvent(
            "system_error",
            `Failed to rollback: ${(error as Error).message}`,
          );
        }
        break;
      }

      case "approve":
        if (!pendingYield || pendingYield.kind !== "confirmation") {
          addSystemEvent("system_warning", "No pending confirmation.");
          return;
        }
        await submitPendingYield({ choice: "approve" });
        break;

      case "reject":
        if (!pendingYield || pendingYield.kind !== "confirmation") {
          addSystemEvent("system_warning", "No pending confirmation.");
          return;
        }
        await submitPendingYield({ choice: "reject" });
        break;

      case "modify": {
        if (!pendingYield || pendingYield.kind !== "confirmation") {
          addSystemEvent("system_warning", "No pending confirmation.");
          return;
        }
        const modifications = args.join(" ").trim();
        if (!modifications) {
          addSystemEvent("system_warning", "Usage: /modify <text>");
          return;
        }
        await submitPendingYield({ choice: "modify", modifications });
        break;
      }

      case "answer": {
        if (!pendingYield || pendingYield.kind !== "structured_input") {
          addSystemEvent(
            "system_warning",
            "No pending structured input request.",
          );
          return;
        }

        const value = args.join(" ").trim();
        if (!value) {
          addSystemEvent("system_warning", "Usage: /answer <value>");
          return;
        }

        const questions = structuredQuestions(pendingYield.payload);
        if (questions.length !== 1) {
          addSystemEvent(
            "system_warning",
            "This request has multiple questions. Use /answers <json-array>.",
          );
          return;
        }

        await submitPendingYield({
          answers: [{ question_id: questions[0].id, value }],
        });
        break;
      }

      case "answers": {
        if (!pendingYield || pendingYield.kind !== "structured_input") {
          addSystemEvent(
            "system_warning",
            "No pending structured input request.",
          );
          return;
        }

        const payload = args.join(" ").trim();
        if (!payload) {
          addSystemEvent("system_warning", "Usage: /answers <json-array>");
          return;
        }

        try {
          const parsed = JSON.parse(payload);
          const content = Array.isArray(parsed) ? { answers: parsed } : parsed;
          await submitPendingYield(content);
        } catch {
          addSystemEvent(
            "system_warning",
            "Invalid JSON payload for /answers.",
          );
        }
        break;
      }

      case "resume": {
        if (!pendingYield) {
          addSystemEvent("system_warning", "No pending yield.");
          return;
        }

        const payload = args.join(" ").trim();
        if (!payload) {
          addSystemEvent("system_warning", "Usage: /resume <json-object>");
          return;
        }

        try {
          await submitPendingYield(JSON.parse(payload));
        } catch {
          addSystemEvent("system_warning", "Invalid JSON payload for /resume.");
        }
        break;
      }

      case "clear":
        setEvents([]);
        addSystemEvent("system_message", "Timeline cleared.");
        break;

      case "help":
        addSystemEvent("system_message", "Available Commands:");
        addSystemEvent(
          "system_message",
          "  /new [autonomous|conservative] [auto|on|off] [continue_once|recovery=off] - Create a new session",
        );
        addSystemEvent(
          "system_message",
          "  /connect <id>                  - Connect to an existing session",
        );
        addSystemEvent(
          "system_message",
          "  /sessions                      - List active sessions",
        );
        addSystemEvent(
          "system_message",
          "  /status                        - Show daemon status",
        );
        addSystemEvent(
          "system_message",
          "  /input <text>                  - Append input to current turn",
        );
        addSystemEvent(
          "system_message",
          "  /interrupt                     - Interrupt current execution",
        );
        addSystemEvent(
          "system_message",
          "  /compact                       - Trigger context compaction",
        );
        addSystemEvent(
          "system_message",
          "  /rollback <n>                  - Roll back N turns",
        );
        addSystemEvent(
          "system_message",
          "  /approve | /reject | /modify   - Resolve confirmation yield",
        );
        addSystemEvent(
          "system_message",
          "  /answer | /answers             - Resolve structured input yield",
        );
        addSystemEvent(
          "system_message",
          "  /resume <json>                 - Resolve custom/dynamic yield",
        );
        addSystemEvent(
          "system_message",
          "  /clear                         - Clear timeline",
        );
        addSystemEvent(
          "system_message",
          "  /help                          - Show this help",
        );
        addSystemEvent(
          "system_message",
          "  /exit                          - Exit (or Ctrl+C)",
        );
        addSystemEvent(
          "system_message",
          "Keyboard: use terminal native scroll, Ctrl+L clear, Ctrl+C exit",
        );
        break;

      case "exit":
      case "quit":
        exit();
        break;

      default:
        addSystemEvent(
          "system_warning",
          `Unknown command: /${cmd}. Type /help for available commands.`,
        );
    }
  };

  if (needsSetup) {
    return (
      <InitWizard onComplete={handleSetupComplete} configPath={CONFIG_PATH} />
    );
  }

  const getStatusColor = () => {
    switch (status) {
      case "connected":
        return "green";
      case "connecting":
        return "yellow";
      case "error":
        return "red";
      default:
        return "gray";
    }
  };

  const getStatusGlyph = () => {
    switch (status) {
      case "connected":
        return "●";
      case "connecting":
        return "◐";
      case "error":
        return "○";
      default:
        return "○";
    }
  };

  const renderPendingYield = () => {
    if (!pendingYield) {
      return null;
    }

    if (pendingYield.kind === "confirmation") {
      const summary = confirmationSummary(pendingYield.payload);
      const options = confirmationOptions(pendingYield.payload);

      return (
        <Box
          borderStyle="round"
          borderColor="yellow"
          flexDirection="column"
          paddingX={1}
        >
          <Text color="yellow" bold>
            Action required: confirmation
          </Text>
          <Text color="gray">request_id: {pendingYield.requestId}</Text>
          {summary && <Text>{summary}</Text>}
          {options.length > 0 && (
            <Text color="gray">options: {options.join(", ")}</Text>
          )}
          <Text color="gray">
            Commands: /approve | /reject | /modify &lt;text&gt;
          </Text>
        </Box>
      );
    }

    if (pendingYield.kind === "structured_input") {
      const title = structuredTitle(pendingYield.payload);
      const prompt = structuredPrompt(pendingYield.payload);
      const questions = structuredQuestions(pendingYield.payload);

      return (
        <Box
          borderStyle="round"
          borderColor="yellow"
          flexDirection="column"
          paddingX={1}
        >
          <Text color="yellow" bold>
            Action required: structured input
          </Text>
          <Text color="gray">request_id: {pendingYield.requestId}</Text>
          {title && <Text>{title}</Text>}
          {prompt && <Text color="gray">{prompt}</Text>}
          {questions.slice(0, 4).map((q) => (
            <Box key={q.id} flexDirection="column">
              <Text>
                - {q.id}
                {q.required ? " *" : ""}: {q.label}
              </Text>
              {q.options && q.options.length > 0 ? (
                <Text color="gray">
                  options:{" "}
                  {q.options
                    .slice(0, 3)
                    .map((o) => o.value)
                    .join(", ")}
                  {q.options.length > 3 ? " ..." : ""}
                </Text>
              ) : null}
            </Box>
          ))}
          {questions.length > 4 && (
            <Text color="gray">...and {questions.length - 4} more</Text>
          )}
          {questions.length === 1 ? (
            <Text color="gray">Command: /answer &lt;value&gt;</Text>
          ) : (
            <Text color="gray">
              Command: /answers '
              {structuredAnswersTemplate(pendingYield.payload)}'
            </Text>
          )}
        </Box>
      );
    }

    return (
      <Box
        borderStyle="round"
        borderColor="yellow"
        flexDirection="column"
        paddingX={1}
      >
        <Text color="yellow" bold>
          Action required: {pendingYield.kind}
        </Text>
        <Text color="gray">request_id: {pendingYield.requestId}</Text>
        <Text color="gray">Command: /resume &lt;json-object&gt;</Text>
      </Box>
    );
  };

  const pendingLabel = pendingYield
    ? `${pendingYield.kind}:${shortId(pendingYield.requestId)}`
    : "none";

  const footerHint = pendingYield
    ? pendingYield.kind === "confirmation"
      ? "Resolve: /approve | /reject | /modify"
      : pendingYield.kind === "structured_input"
        ? "Resolve: /answer or /answers"
        : "Resolve: /resume <json>"
    : "Enter to send | /help commands | terminal scrollback | Ctrl+C exit";

  return (
    <Box flexDirection="column" width="100%">
      <Box
        borderStyle="round"
        borderColor={getStatusColor()}
        flexDirection="column"
        paddingX={1}
      >
        <Box>
          <Text bold>Alan TUI</Text>
          <Text color="gray"> protocol-first terminal workspace assistant</Text>
        </Box>
        <Text color={getStatusColor()}>
          {getStatusGlyph()} {statusMessage}
        </Text>
        <Text color="gray">
          mode={STARTUP_INFO.mode === "embedded" ? "local" : "remote"} |
          session={shortId(currentSessionId)}
          {currentSessionId ? "..." : ""} | pending={pendingLabel} | events=
          {events.length}
        </Text>
      </Box>

      {renderPendingYield()}

      <Box
        borderStyle="single"
        borderColor="gray"
        flexDirection="column"
        paddingX={1}
      >
        <MessageList events={events} />
      </Box>

      <Box paddingX={1}>
        <Text color="gray">{footerHint}</Text>
      </Box>

      <Box
        borderStyle="round"
        borderColor={pendingYield ? "yellow" : "blue"}
        paddingX={1}
      >
        <Text color={pendingYield ? "yellow" : "cyan"} bold>
          {pendingYield ? "Action" : "Input"}
        </Text>
        <Text> </Text>
        <Text color={pendingYield ? "yellow" : "white"}>{"> "}</Text>
        <TextInput
          value={inputValue}
          onChange={setInputValue}
          onSubmit={handleSubmit}
          placeholder={
            pendingYield
              ? "Resolve pending yield with command..."
              : "Type message or /help"
          }
        />
      </Box>
    </Box>
  );
}

render(<App />);
