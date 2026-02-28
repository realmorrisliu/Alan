#!/usr/bin/env bun
/**
 * Alan TUI entrypoint.
 */

import React, { useEffect, useRef, useState } from "react";
import { render, Box, Text, useApp, useInput } from "ink";
import TextInput from "ink-text-input";
import { existsSync } from "node:fs";
import { join } from "node:path";
import { homedir } from "node:os";
import { AlanClient } from "./client.js";
import type { DaemonStatus, EventEnvelope, YieldKind } from "./types.js";
import { MessageList } from "./components.js";
import { InitWizard } from "./init.js";
import {
  confirmationOptions,
  confirmationSummary,
  structuredPrompt,
  structuredQuestions,
  structuredTitle,
} from "./yield.js";

const AGENTD_URL = process.env.ALAN_AGENTD_URL;
const AUTO_MANAGE = !AGENTD_URL;
const VERBOSE = process.env.ALAN_VERBOSE === "1";
const MAX_EVENT_HISTORY = 2000;

const STARTUP_INFO = {
  mode: AGENTD_URL ? "remote" : ("embedded" as const),
  url: AGENTD_URL || "ws://127.0.0.1:8090",
};

interface PendingYield {
  requestId: string;
  kind: YieldKind;
  payload: unknown;
}

function needsFirstTimeSetup(): boolean {
  if (AGENTD_URL) return false;
  const configPath = join(homedir(), ".alan", "config.toml");
  return !existsSync(configPath);
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
  const [scrollOffset, setScrollOffset] = useState(0);
  const [messageRowCount, setMessageRowCount] = useState(0);

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

  const terminalRows = process.stdout.rows || 24;
  const reservedRows = pendingYield ? 16 : 11;
  const messageViewportRows = Math.max(terminalRows - reservedRows, 6);
  const maxScrollOffset = Math.max(0, messageRowCount - messageViewportRows);

  useEffect(() => {
    setScrollOffset((prev) => Math.min(prev, maxScrollOffset));
  }, [maxScrollOffset]);

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
          'Use /answers <json-array> with [{"question_id":"...","value":"..."}].',
        );
      }
      return;
    }

    addSystemEvent(
      "system_warning",
      `Dynamic tool call pending (${incoming.requestId}).`,
    );
    addSystemEvent(
      "system_message",
      "Use /resume <json> to return a custom content payload.",
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

      if (envelope.type === "yield" && envelope.request_id && envelope.kind) {
        const incoming: PendingYield = {
          requestId: envelope.request_id,
          kind: envelope.kind,
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

    const detectWorkspaceDir = async (): Promise<string | undefined> => {
      const cwd = process.cwd();
      try {
        const { existsSync } = await import("node:fs");
        const { join } = await import("node:path");

        if (existsSync(join(cwd, ".alan"))) {
          addSystemEvent("system_message", `Detected workspace: ${cwd}`);
          return cwd;
        }
      } catch {
        // ignore
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
          const workspaceDir = await detectWorkspaceDir();

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

          const sessionId = await client.createSession({
            workspace_dir: workspaceDir,
          });
          setCurrentSessionId(sessionId);
          await client.connectToSession(sessionId);
          addSystemEvent(
            "system_message",
            "Alan agent ready. Type your request directly.",
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
            addSystemEvent("system_message", "提示: 看起来是 LLM 配置问题");
            addSystemEvent("system_message", "  请检查 ~/.alan/config.toml");
          } else if (
            msg.includes("500") ||
            msg.includes("Internal Server Error")
          ) {
            addSystemEvent(
              "system_message",
              "提示: daemon 内部错误，请检查 daemon 日志",
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

    const pageStep = Math.max(1, Math.floor(messageViewportRows * 0.8));
    if (key.pageUp || (key.ctrl && input === "u")) {
      setScrollOffset((prev) => Math.min(maxScrollOffset, prev + pageStep));
      return;
    }
    if (key.pageDown || (key.ctrl && input === "d")) {
      setScrollOffset((prev) => Math.max(0, prev - pageStep));
      return;
    }
    if (key.shift && key.upArrow) {
      setScrollOffset((prev) => Math.min(maxScrollOffset, prev + 1));
      return;
    }
    if (key.shift && key.downArrow) {
      setScrollOffset((prev) => Math.max(0, prev - 1));
      return;
    }
    if (key.home) {
      setScrollOffset(maxScrollOffset);
      return;
    }
    if (key.end) {
      setScrollOffset(0);
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
        "Yield is pending. Resolve it first (/approve, /reject, /modify, /answer, /answers).",
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
      case "new":
        try {
          addSystemEvent("system_message", "Creating new session...");
          const sessionId = await client.createSession();
          setCurrentSessionId(sessionId);
          setPendingYield(null);
          await client.connectToSession(sessionId);
          addSystemEvent("system_message", "Session created and connected");
        } catch (error) {
          const msg = (error as Error).message;
          addSystemEvent("system_error", msg);

          if (
            msg.includes("LLM") ||
            msg.includes("llm") ||
            msg.includes("model") ||
            msg.includes("key")
          ) {
            addSystemEvent("system_message", "提示: 看起来是 LLM 配置问题");
            addSystemEvent("system_message", "  请检查 ~/.alan/config.toml");
          } else if (
            msg.includes("500") ||
            msg.includes("Internal Server Error")
          ) {
            addSystemEvent(
              "system_message",
              "提示: daemon 内部错误，请检查 daemon 日志",
            );
          }
        }
        break;

      case "connect":
        if (!args[0]) {
          addSystemEvent("system_warning", "Usage: /connect <session-id>");
          return;
        }
        try {
          addSystemEvent(
            "system_message",
            `Connecting to session ${args[0].slice(0, 8)}...`,
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
              `  ${s.session_id.slice(0, 8)}... ${s.active ? "active" : "inactive"}`,
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

      case "help":
        addSystemEvent("system_message", "Available Commands:");
        addSystemEvent(
          "system_message",
          "  /new               - Create a new session",
        );
        addSystemEvent(
          "system_message",
          "  /connect <id>      - Connect to an existing session",
        );
        addSystemEvent(
          "system_message",
          "  /sessions          - List active sessions",
        );
        addSystemEvent(
          "system_message",
          "  /status            - Show daemon status",
        );
        addSystemEvent(
          "system_message",
          "  /approve           - Approve pending confirmation",
        );
        addSystemEvent(
          "system_message",
          "  /reject            - Reject pending confirmation",
        );
        addSystemEvent(
          "system_message",
          "  /modify <text>     - Modify pending confirmation",
        );
        addSystemEvent(
          "system_message",
          "  /answer <value>    - Reply to single structured question",
        );
        addSystemEvent(
          "system_message",
          "  /answers <json>    - Reply to multi-question structured input",
        );
        addSystemEvent(
          "system_message",
          "  /resume <json>     - Generic yield resume payload",
        );
        addSystemEvent(
          "system_message",
          "  /help              - Show this help",
        );
        addSystemEvent(
          "system_message",
          "  /exit              - Exit (or press Ctrl+C)",
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
    return <InitWizard onComplete={handleSetupComplete} />;
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
            Pending confirmation
          </Text>
          <Text>request_id: {pendingYield.requestId}</Text>
          {summary && <Text>{summary}</Text>}
          {options.length > 0 && (
            <Text color="gray">options: {options.join(", ")}</Text>
          )}
          <Text color="gray">
            Use /approve, /reject, or /modify &lt;text&gt;
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
            Pending structured input
          </Text>
          <Text>request_id: {pendingYield.requestId}</Text>
          {title && <Text>{title}</Text>}
          {prompt && <Text color="gray">{prompt}</Text>}
          {questions.slice(0, 3).map((q) => (
            <Text key={q.id}>
              - {q.id}
              {q.required ? " *" : ""}: {q.label}
            </Text>
          ))}
          {questions.length > 3 && (
            <Text color="gray">...and {questions.length - 3} more</Text>
          )}
          <Text color="gray">
            Use /answer &lt;value&gt; (single) or /answers &lt;json-array&gt;
          </Text>
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
          Pending dynamic tool call
        </Text>
        <Text>request_id: {pendingYield.requestId}</Text>
        <Text color="gray">Use /resume &lt;json-object&gt;</Text>
      </Box>
    );
  };

  const scrollStatus =
    maxScrollOffset > 0
      ? ` | Scroll: ${scrollOffset === 0 ? "bottom" : `-${scrollOffset}`} (PgUp/PgDn, Shift+↑/↓)`
      : "";

  return (
    <Box flexDirection="column" height={process.stdout.rows || 24} width="100%">
      <Box borderStyle="single" borderColor="cyan" paddingX={1}>
        <Text bold>Alan Agent</Text>
        <Text color="gray">
          {" "}
          {STARTUP_INFO.mode === "embedded" ? "(local)" : "(remote)"}
        </Text>
      </Box>

      {renderPendingYield()}

      <Box flexGrow={1} flexDirection="column" paddingX={1}>
        <MessageList
          events={events}
          maxRows={messageViewportRows}
          scrollOffset={scrollOffset}
          onRowCountChange={(count) =>
            setMessageRowCount((prev) => (prev === count ? prev : count))
          }
        />
      </Box>

      <Box backgroundColor={getStatusColor()} paddingX={1}>
        <Text color="black">
          {status === "connected" ? "●" : status === "connecting" ? "◐" : "○"}{" "}
          {statusMessage}
          {currentSessionId
            ? ` | Session: ${currentSessionId.slice(0, 8)}...`
            : ""}
          {scrollStatus}
        </Text>
      </Box>

      <Box backgroundColor="blue" paddingX={1}>
        <Text color="white" bold>
          {"> "}{" "}
        </Text>
        <TextInput
          value={inputValue}
          onChange={setInputValue}
          onSubmit={handleSubmit}
          placeholder={
            pendingYield
              ? "Resolve pending yield first..."
              : "Type message or /help"
          }
        />
      </Box>
    </Box>
  );
}

render(<App />);
