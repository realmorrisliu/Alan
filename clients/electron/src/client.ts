/**
 * Alan Web Client - Browser-compatible client for Alan agent daemon
 */

import type { 
  EventEnvelope, 
  Submission, 
  Op, 
  Session,
  CreateSessionRequest,
  CreateSessionResponse 
} from './types.js';

type EventHandler<T> = (data: T) => void;

export class AlanWebClient {
  private ws: WebSocket | null = null;
  private baseUrl: string;
  private wsUrl: string;
  private eventHandlers: Map<string, EventHandler<unknown>[]> = new Map();
  private reconnectAttempts = 0;
  private maxReconnectAttempts = 5;
  private reconnectDelay = 1000;

  constructor(url: string) {
    // Convert HTTP URL to WebSocket URL
    this.baseUrl = url.replace(/^ws/, 'http').replace(/\/$/, '');
    this.wsUrl = url.replace(/^http/, 'ws').replace(/\/$/, '');
  }

  // Event emitter implementation
  public on<K extends string>(
    event: K,
    handler: EventHandler<unknown>
  ): void {
    if (!this.eventHandlers.has(event)) {
      this.eventHandlers.set(event, []);
    }
    this.eventHandlers.get(event)!.push(handler);
  }

  public off<K extends string>(
    event: K,
    handler: EventHandler<unknown>
  ): void {
    const handlers = this.eventHandlers.get(event);
    if (handlers) {
      const index = handlers.indexOf(handler);
      if (index > -1) {
        handlers.splice(index, 1);
      }
    }
  }

  private emit<K extends string>(
    event: K,
    ...args: unknown[]
  ): void {
    const handlers = this.eventHandlers.get(event);
    if (handlers) {
      handlers.forEach(handler => (handler as (...args: unknown[]) => void)(...args));
    }
  }

  // HTTP API methods
  public async createSession(request?: CreateSessionRequest): Promise<string> {
    const response = await fetch(`${this.baseUrl}/api/v1/sessions`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request || {}),
    });

    if (!response.ok) {
      throw new Error(`Failed to create session: ${response.statusText}`);
    }

    const data = await response.json() as CreateSessionResponse;
    this.emit('session_created', data.id);
    return data.id;
  }

  public async listSessions(): Promise<Session[]> {
    const response = await fetch(`${this.baseUrl}/api/v1/sessions`);
    
    if (!response.ok) {
      throw new Error(`Failed to list sessions: ${response.statusText}`);
    }

    return await response.json() as Session[];
  }

  public async getSession(sessionId: string): Promise<Session> {
    const response = await fetch(`${this.baseUrl}/api/v1/sessions/${sessionId}`);
    
    if (!response.ok) {
      throw new Error(`Failed to get session: ${response.statusText}`);
    }

    return await response.json() as Session;
  }

  public async submitOperation(sessionId: string, op: Op): Promise<void> {
    const submission: Submission = {
      id: crypto.randomUUID(),
      op,
    };

    const response = await fetch(`${this.baseUrl}/api/v1/sessions/${sessionId}/submit`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(submission),
    });

    if (!response.ok) {
      throw new Error(`Failed to submit operation: ${response.statusText}`);
    }
  }

  // WebSocket methods
  public async connect(): Promise<void> {
    return new Promise((resolve, reject) => {
      try {
        this.ws = new WebSocket(this.wsUrl);

        this.ws.onopen = () => {
          this.reconnectAttempts = 0;
          this.emit('connected');
          resolve();
        };

        this.ws.onmessage = (event) => {
          try {
            const envelope = JSON.parse(event.data) as EventEnvelope;
            this.emit('event', envelope);
          } catch (error) {
            console.error('Failed to parse message:', error);
          }
        };

        this.ws.onclose = () => {
          this.emit('disconnected');
          this.attemptReconnect();
        };

        this.ws.onerror = (error) => {
          this.emit('error', error);
          reject(error);
        };
      } catch (error) {
        reject(error);
      }
    });
  }

  public async connectToSession(sessionId: string): Promise<void> {
    const wsUrl = `${this.wsUrl}/api/v1/sessions/${sessionId}/ws`;
    
    return new Promise((resolve, reject) => {
      try {
        // Close existing connection if any
        if (this.ws) {
          this.ws.close();
        }

        this.ws = new WebSocket(wsUrl);

        this.ws.onopen = () => {
          this.reconnectAttempts = 0;
          this.emit('connected');
          resolve();
        };

        this.ws.onmessage = (event) => {
          try {
            const envelope = JSON.parse(event.data) as EventEnvelope;
            this.emit('event', envelope);
          } catch (error) {
            console.error('Failed to parse message:', error);
          }
        };

        this.ws.onclose = () => {
          this.emit('disconnected');
          this.attemptReconnect();
        };

        this.ws.onerror = (error) => {
          this.emit('error', error);
          reject(error);
        };
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
  }

  private attemptReconnect(): void {
    if (this.reconnectAttempts >= this.maxReconnectAttempts) {
      this.emit('error', new Error('Max reconnection attempts reached'));
      return;
    }

    this.reconnectAttempts++;
    const delay = this.reconnectDelay * Math.pow(2, this.reconnectAttempts - 1);

    setTimeout(() => {
      this.connect().catch(() => {
        // Error handled in connect
      });
    }, delay);
  }

  // Convenience methods
  public async sendMessage(sessionId: string, content: string): Promise<void> {
    await this.submitOperation(sessionId, {
      type: 'input',
      content,
    });
  }

  public async startTask(sessionId: string, input: string): Promise<void> {
    await this.submitOperation(sessionId, {
      type: 'turn',
      input,
    });
  }

  public async resume(
    sessionId: string,
    requestId: string,
    result: unknown
  ): Promise<void> {
    await this.submitOperation(sessionId, {
      type: 'resume',
      request_id: requestId,
      result,
    });
  }

  public async interrupt(sessionId: string): Promise<void> {
    await this.submitOperation(sessionId, {
      type: 'interrupt',
    });
  }
}
