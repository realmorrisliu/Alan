/**
 * Alan Client - WebSocket and HTTP client for Alan agent daemon
 *
 * 支持两种模式：
 * 1. 自动模式（默认）：TUI 自动启动并管理 daemon 进程
 * 2. 远程模式：连接到已有的 daemon 实例
 */

import { WebSocket } from "ws";
import { DaemonManager, getDaemon } from "./daemon.js";
import type {
  ContentPart,
  EventEnvelope,
  Op,
  SessionListResponse,
  SessionListItem,
  SessionReadResponse,
  CreateSessionRequest,
  CreateSessionResponse,
  ClientEvents,
} from "./types";

type EventHandler<T> = (data: T) => void;

interface ReadEventsResponse {
  gap: boolean;
  oldest_event_id?: string | null;
  latest_event_id?: string | null;
  events: EventEnvelope[];
}

function toResumeContent(contentInput: unknown): ContentPart[] {
  if (contentInput === null || contentInput === undefined) {
    return [];
  }

  if (typeof contentInput === "string") {
    return [{ type: "text", text: contentInput }];
  }

  return [{ type: "structured", data: contentInput }];
}

export interface AlanClientOptions {
  /**
   * daemon URL，默认自动启动本地 daemon
   * 可以设置为远程 URL，如 ws://remote-server:8090
   */
  url?: string;
  /** 自动管理 daemon 进程（仅在连接本地 daemon 时有效） */
  autoManageDaemon?: boolean;
  /** 是否显示详细日志 */
  verbose?: boolean;
}

export class AlanClient {
  private ws: WebSocket | null = null;
  private baseUrl: string;
  private wsUrl: string;
  private eventHandlers: Map<string, EventHandler<unknown>[]> = new Map();
  private reconnectAttempts = 0;
  private maxReconnectAttempts = 5;
  private reconnectDelay = 1000;
  private options: Required<AlanClientOptions>;
  private daemon: DaemonManager | null = null;
  private isRemote = false;
  private currentSessionId: string | null = null;
  private reconnectTimer: NodeJS.Timeout | null = null;
  private reconnectEnabled = true;
  private connectionVersion = 0;
  private lastEventId: string | null = null;
  private seenEventIds: string[] = [];
  private seenEventSet: Set<string> = new Set();
  private maxSeenEventIds = 4096;
  private maxReplayEvents = 20000;

  constructor(options: AlanClientOptions = {}) {
    this.options = {
      url: options.url ?? "ws://127.0.0.1:8090",
      autoManageDaemon: options.autoManageDaemon ?? true,
      verbose: options.verbose ?? false,
    };

    // 判断是否为远程连接（非 localhost/127.0.0.1）
    this.isRemote =
      !this.options.url.includes("127.0.0.1") &&
      !this.options.url.includes("localhost");

    // Convert HTTP URL to WebSocket URL
    this.baseUrl = this.options.url.replace(/^ws/, "http").replace(/\/$/, "");
    this.wsUrl = this.options.url.replace(/^http/, "ws").replace(/\/$/, "");
  }

  /**
   * 确保 daemon 在运行（本地模式）
   */
  async ensureDaemon(): Promise<void> {
    if (this.isRemote || !this.options.autoManageDaemon) {
      return;
    }

    this.daemon = getDaemon({ verbose: this.options.verbose });
    const status = await this.daemon.start();

    if (this.options.verbose) {
      console.log(
        `[Alan] daemon ${status.state}${status.pid ? ` (pid: ${status.pid})` : ""}`,
      );
    }
  }

  // Event emitter implementation
  public on<K extends keyof ClientEvents>(
    event: K,
    handler: ClientEvents[K],
  ): void {
    if (!this.eventHandlers.has(event)) {
      this.eventHandlers.set(event, []);
    }
    this.eventHandlers.get(event)!.push(handler as EventHandler<unknown>);
  }

  public off<K extends keyof ClientEvents>(
    event: K,
    handler: ClientEvents[K],
  ): void {
    const handlers = this.eventHandlers.get(event);
    if (handlers) {
      const index = handlers.indexOf(handler as EventHandler<unknown>);
      if (index > -1) {
        handlers.splice(index, 1);
      }
    }
  }

  private emit<K extends keyof ClientEvents>(
    event: K,
    ...args: Parameters<ClientEvents[K]>
  ): void {
    const handlers = this.eventHandlers.get(event);
    if (handlers) {
      handlers.forEach((handler) =>
        (handler as (...args: unknown[]) => void)(...args),
      );
    }
  }

  private resetReplayState(): void {
    this.lastEventId = null;
    this.seenEventIds = [];
    this.seenEventSet.clear();
  }

  private rememberEventId(eventId: string): void {
    if (this.seenEventSet.has(eventId)) return;
    this.seenEventSet.add(eventId);
    this.seenEventIds.push(eventId);
    if (this.seenEventIds.length > this.maxSeenEventIds) {
      const removed = this.seenEventIds.shift();
      if (removed) {
        this.seenEventSet.delete(removed);
      }
    }
  }

  private emitEnvelope(envelope: EventEnvelope): void {
    const eventId = envelope.event_id;
    if (eventId && this.seenEventSet.has(eventId)) {
      return;
    }
    if (eventId) {
      this.rememberEventId(eventId);
      this.lastEventId = eventId;
    }
    this.emit("event", envelope);
  }

  private async replayMissedEvents(
    sessionId: string,
    version: number,
  ): Promise<void> {
    if (!this.lastEventId) {
      return;
    }

    let afterEventId: string | null = this.lastEventId;
    let replayedEvents = 0;
    const pageLimit = 200;
    while (afterEventId) {
      if (version !== this.connectionVersion) {
        return;
      }

      const params = new URLSearchParams({
        limit: String(pageLimit),
        after_event_id: afterEventId,
      });
      const response = await fetch(
        `${this.baseUrl}/api/v1/sessions/${sessionId}/events/read?${params.toString()}`,
      );
      if (!response.ok) {
        throw new Error(`Failed to replay missed events: ${response.statusText}`);
      }
      const page = (await response.json()) as ReadEventsResponse;
      if (!Array.isArray(page.events)) {
        throw new Error("Failed to replay missed events: malformed read-events page");
      }

      if (page.gap) {
        const oldest = page.oldest_event_id ?? "unknown";
        const latest = page.latest_event_id ?? "unknown";
        this.emit(
          "error",
          new Error(
            `Event replay gap detected (oldest=${oldest}, latest=${latest}). Some recent events may have been evicted from server buffer.`,
          ),
        );
        if (page.events.length === 0) {
          throw new Error("Event replay gap detected but no replayable events were returned");
        }
      }

      if (page.events.length === 0) {
        return;
      }

      for (const envelope of page.events) {
        this.emitEnvelope(envelope);
      }
      replayedEvents += page.events.length;
      if (replayedEvents > this.maxReplayEvents) {
        throw new Error(
          `Replay exceeded safety limit (${this.maxReplayEvents} events); stopping to avoid unbounded catch-up loop.`,
        );
      }

      const pageLastEventId = page.events[page.events.length - 1]?.event_id ?? null;
      if (!pageLastEventId) {
        throw new Error("Failed to replay missed events: replay page contains event without event_id");
      }
      if (
        pageLastEventId === afterEventId ||
        page.events.length < pageLimit ||
        (page.latest_event_id !== undefined &&
          page.latest_event_id !== null &&
          pageLastEventId === page.latest_event_id)
      ) {
        return;
      }
      afterEventId = pageLastEventId;
    }
  }

  // HTTP API methods
  public async createSession(
    request?: CreateSessionRequest,
  ): Promise<CreateSessionResponse> {
    await this.ensureDaemon();

    const response = await fetch(`${this.baseUrl}/api/v1/sessions`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(request || {}),
    });

    if (!response.ok) {
      let errorMsg = response.statusText;
      try {
        const errData = await response.json();
        if (errData && (errData as any).error) {
          errorMsg = (errData as any).error;
        }
      } catch (e) {
        // ignore JSON parse error
      }
      throw new Error(`Failed to create session: ${errorMsg}`);
    }

    const data = (await response.json()) as CreateSessionResponse;
    this.emit("session_created", data.session_id);
    return data;
  }

  public async listSessions(): Promise<SessionListItem[]> {
    await this.ensureDaemon();

    const response = await fetch(`${this.baseUrl}/api/v1/sessions`);

    if (!response.ok) {
      throw new Error(`Failed to list sessions: ${response.statusText}`);
    }

    const data = (await response.json()) as SessionListResponse;
    return data.sessions;
  }

  public async getSession(sessionId: string): Promise<SessionReadResponse> {
    await this.ensureDaemon();

    const response = await fetch(
      `${this.baseUrl}/api/v1/sessions/${sessionId}/read`,
    );

    if (!response.ok) {
      throw new Error(`Failed to get session: ${response.statusText}`);
    }

    return (await response.json()) as SessionReadResponse;
  }

  public async submitOperation(sessionId: string, op: Op): Promise<void> {
    await this.ensureDaemon();

    const response = await fetch(
      `${this.baseUrl}/api/v1/sessions/${sessionId}/submit`,
      {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ op }),
      },
    );

    if (!response.ok) {
      throw new Error(`Failed to submit operation: ${response.statusText}`);
    }
  }

  /**
   * 连接到 daemon（HTTP 健康检查）
   * 不建立 WebSocket 连接，WebSocket 在 connectToSession 时建立
   */
  public async connect(): Promise<void> {
    // 如果是本地模式，先确保 daemon 启动
    if (!this.isRemote && this.options.autoManageDaemon) {
      await this.ensureDaemon();
    }

    // 等待 daemon 就绪（健康检查）
    const startTime = Date.now();
    const timeout = 10000;
    const checkInterval = 200;

    while (Date.now() - startTime < timeout) {
      if (await this.isDaemonRunning()) {
        this.reconnectAttempts = 0;
        this.emit("connected");
        return;
      }
      await new Promise((r) => setTimeout(r, checkInterval));
    }

    throw new Error("Failed to connect to daemon: health check timeout");
  }

  public async connectToSession(sessionId: string): Promise<void> {
    // 如果是本地模式，先确保 daemon 启动
    if (!this.isRemote && this.options.autoManageDaemon) {
      await this.ensureDaemon();
    }

    // Switching session should not trigger reconnect from an old socket close.
    this.reconnectEnabled = false;
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }

    if (this.ws) {
      this.ws.removeAllListeners();
      this.ws.close();
      this.ws = null;
    }

    const previousSessionId = this.currentSessionId;
    if (previousSessionId !== sessionId) {
      this.resetReplayState();
    }
    this.currentSessionId = sessionId;
    const version = ++this.connectionVersion;
    const wsUrl = `${this.wsUrl}/api/v1/sessions/${sessionId}/ws`;

    return new Promise((resolve, reject) => {
      let ready = false;
      let settled = false;
      const queuedEnvelopes: EventEnvelope[] = [];
      const resolveOnce = () => {
        if (settled) return;
        settled = true;
        resolve();
      };
      const rejectOnce = (error: Error) => {
        if (settled) return;
        settled = true;
        reject(error);
      };

      try {
        this.ws = new WebSocket(wsUrl);

        this.ws.on("open", () => {
          if (version !== this.connectionVersion) return;
          void (async () => {
            try {
              await this.replayMissedEvents(sessionId, version);
            } catch (error) {
              const replayError = error as Error;
              this.emit("error", replayError);
              rejectOnce(replayError);
              if (version === this.connectionVersion) {
                this.ws?.close();
              }
              return;
            }

            if (version !== this.connectionVersion) {
              return;
            }
            ready = true;
            this.reconnectEnabled = true;
            this.reconnectAttempts = 0;
            for (const envelope of queuedEnvelopes.splice(0)) {
              this.emitEnvelope(envelope);
            }
            this.emit("connected");
            resolveOnce();
          })();
        });

        this.ws.on("message", (data: Buffer) => {
          if (version !== this.connectionVersion) return;
          try {
            const envelope = JSON.parse(data.toString()) as EventEnvelope;
            if (ready) {
              this.emitEnvelope(envelope);
            } else {
              queuedEnvelopes.push(envelope);
            }
          } catch (error) {
            console.error("Failed to parse message:", error);
          }
        });

        this.ws.on("close", () => {
          if (version !== this.connectionVersion) return;
          this.emit("disconnected");
          if (!ready) {
            rejectOnce(new Error("WebSocket closed before initialization completed"));
          }
          this.attemptReconnect(version);
        });

        this.ws.on("error", (error: Error) => {
          if (version !== this.connectionVersion) return;
          this.emit("error", error);
          rejectOnce(error);
        });
      } catch (error) {
        rejectOnce(error as Error);
      }
    });
  }

  public disconnect(): void {
    this.reconnectEnabled = false;
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
    this.connectionVersion++;
    if (this.ws) {
      this.ws.removeAllListeners();
      this.ws.close();
      this.ws = null;
    }
    this.currentSessionId = null;
  }

  /**
   * 完全关闭（自动管理模式下停止 daemon）
   */
  async shutdown(): Promise<void> {
    this.disconnect();

    if (this.daemon && !this.isRemote) {
      await this.daemon.stop();
    }
  }

  private attemptReconnect(version: number): void {
    if (!this.reconnectEnabled) return;
    if (!this.currentSessionId) return; // 没有活跃 session 时不重连
    if (version !== this.connectionVersion) return;

    if (this.reconnectAttempts >= this.maxReconnectAttempts) {
      this.emit("error", new Error("Max reconnection attempts reached"));
      return;
    }

    this.reconnectAttempts++;
    const delay = this.reconnectDelay * Math.pow(2, this.reconnectAttempts - 1);

    this.reconnectTimer = setTimeout(() => {
      // 使用保存的 sessionId 重连
      if (this.currentSessionId && version === this.connectionVersion) {
        this.connectToSession(this.currentSessionId).catch(() => {
          // Error handled in connectToSession
        });
      }
    }, delay);
  }

  // Convenience methods
  public async sendMessage(sessionId: string, content: string): Promise<void> {
    await this.submitOperation(sessionId, {
      type: "turn",
      parts: [{ type: "text", text: content }],
    });
  }

  public async startTask(sessionId: string, input: string): Promise<void> {
    await this.submitOperation(sessionId, {
      type: "turn",
      parts: [{ type: "text", text: input }],
    });
  }

  public async sendInput(sessionId: string, content: string): Promise<void> {
    await this.submitOperation(sessionId, {
      type: "input",
      parts: [{ type: "text", text: content }],
    });
  }

  public async resume(
    sessionId: string,
    requestId: string,
    content: unknown,
  ): Promise<void> {
    await this.submitOperation(sessionId, {
      type: "resume",
      request_id: requestId,
      content: toResumeContent(content),
    });
  }

  public async interrupt(sessionId: string): Promise<void> {
    await this.submitOperation(sessionId, {
      type: "interrupt",
    });
  }

  public async compact(sessionId: string): Promise<void> {
    await this.submitOperation(sessionId, {
      type: "compact",
    });
  }

  public async rollback(sessionId: string, turns: number): Promise<void> {
    await this.submitOperation(sessionId, {
      type: "rollback",
      turns,
    });
  }

  /**
   * 检查 daemon 是否可用
   */
  async isDaemonRunning(): Promise<boolean> {
    try {
      const response = await fetch(`${this.baseUrl}/health`, {
        signal: AbortSignal.timeout(1000),
      });
      return response.ok;
    } catch {
      return false;
    }
  }

  /**
   * 获取 daemon 状态（如果是自动管理的）
   */
  getDaemonStatus() {
    return this.daemon?.getStatus();
  }
}
