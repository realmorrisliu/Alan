/**
 * Alan Client - WebSocket and HTTP client for Alan agent daemon
 *
 * 支持两种模式：
 * 1. 自动模式（默认）：TUI 自动启动并管理 agentd 进程
 * 2. 远程模式：连接到已有的 agentd 实例
 */

import { WebSocket } from "ws";
import { DaemonManager, ensureDaemon, getDaemon } from "./daemon.js";
import type {
  EventEnvelope,
  Submission,
  Op,
  SessionListResponse,
  SessionListItem,
  SessionReadResponse,
  CreateSessionRequest,
  CreateSessionResponse,
  ClientEvents,
} from "./types";

type EventHandler<T> = (data: T) => void;

export interface AlanClientOptions {
  /**
   * agentd URL，默认自动启动本地 agentd
   * 可以设置为远程 URL，如 ws://remote-server:8090
   */
  url?: string;
  /** 自动管理 agentd 进程（仅在连接本地 agentd 时有效） */
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
   * 确保 agentd 在运行（本地模式）
   */
  async ensureDaemon(): Promise<void> {
    if (this.isRemote || !this.options.autoManageDaemon) {
      return;
    }

    this.daemon = getDaemon({ verbose: this.options.verbose });
    const status = await this.daemon.start();

    if (this.options.verbose) {
      console.log(
        `[Alan] agentd ${status.state}${status.pid ? ` (pid: ${status.pid})` : ""}`,
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

  // HTTP API methods
  public async createSession(request?: CreateSessionRequest): Promise<string> {
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
    return data.session_id;
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
      `${this.baseUrl}/api/v1/sessions/${sessionId}`,
    );

    if (!response.ok) {
      throw new Error(`Failed to get session: ${response.statusText}`);
    }

    return (await response.json()) as SessionReadResponse;
  }

  public async submitOperation(sessionId: string, op: Op): Promise<void> {
    await this.ensureDaemon();

    const submission: Submission = {
      id: crypto.randomUUID(),
      op,
    };

    const response = await fetch(
      `${this.baseUrl}/api/v1/sessions/${sessionId}/submit`,
      {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(submission),
      },
    );

    if (!response.ok) {
      throw new Error(`Failed to submit operation: ${response.statusText}`);
    }
  }

  /**
   * 连接到 agentd（HTTP 健康检查）
   * 不建立 WebSocket 连接，WebSocket 在 connectToSession 时建立
   */
  public async connect(): Promise<void> {
    // 如果是本地模式，先确保 agentd 启动
    if (!this.isRemote && this.options.autoManageDaemon) {
      await this.ensureDaemon();
    }

    // 等待 agentd 就绪（健康检查）
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

    throw new Error("Failed to connect to agentd: health check timeout");
  }

  public async connectToSession(sessionId: string): Promise<void> {
    // 如果是本地模式，先确保 agentd 启动
    if (!this.isRemote && this.options.autoManageDaemon) {
      await this.ensureDaemon();
    }

    this.currentSessionId = sessionId;
    const wsUrl = `${this.wsUrl}/api/v1/sessions/${sessionId}/ws`;

    return new Promise((resolve, reject) => {
      try {
        // Close existing connection if any
        if (this.ws) {
          this.ws.close();
          this.ws = null;
        }

        this.ws = new WebSocket(wsUrl);

        this.ws.on("open", () => {
          this.reconnectAttempts = 0;
          this.emit("connected");
          resolve();
        });

        this.ws.on("message", (data: Buffer) => {
          try {
            const envelope = JSON.parse(data.toString()) as EventEnvelope;
            this.emit("event", envelope);
          } catch (error) {
            console.error("Failed to parse message:", error);
          }
        });

        this.ws.on("close", () => {
          this.emit("disconnected");
          this.attemptReconnect();
        });

        this.ws.on("error", (error: Error) => {
          this.emit("error", error);
          reject(error);
        });
      } catch (error) {
        reject(error);
      }
    });
  }

  public disconnect(): void {
    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }
    this.currentSessionId = null;
  }

  /**
   * 完全关闭（停止 agentd 如果是自动管理的）
   */
  async shutdown(): Promise<void> {
    this.disconnect();

    if (this.daemon && !this.isRemote) {
      await this.daemon.stop();
    }
  }

  private attemptReconnect(): void {
    if (!this.currentSessionId) return; // 没有活跃 session 时不重连

    if (this.reconnectAttempts >= this.maxReconnectAttempts) {
      this.emit("error", new Error("Max reconnection attempts reached"));
      return;
    }

    this.reconnectAttempts++;
    const delay = this.reconnectDelay * Math.pow(2, this.reconnectAttempts - 1);

    setTimeout(() => {
      // 使用保存的 sessionId 重连
      if (this.currentSessionId) {
        this.connectToSession(this.currentSessionId).catch(() => {
          // Error handled in connectToSession
        });
      }
    }, delay);
  }

  // Convenience methods
  public async sendMessage(sessionId: string, content: string): Promise<void> {
    await this.submitOperation(sessionId, {
      type: "user_input",
      content,
    });
  }

  public async startTask(sessionId: string, input: string): Promise<void> {
    await this.submitOperation(sessionId, {
      type: "start_task",
      input,
    });
  }

  public async confirmCheckpoint(
    sessionId: string,
    checkpointId: string,
    choice: "approve" | "modify" | "reject",
    modifications?: string,
  ): Promise<void> {
    await this.submitOperation(sessionId, {
      type: "confirm",
      checkpoint_id: checkpointId,
      choice,
      modifications,
    });
  }

  /**
   * 检查 agentd 是否可用
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
   * 获取 agentd 状态（如果是自动管理的）
   */
  getDaemonStatus() {
    return this.daemon?.getStatus();
  }
}
