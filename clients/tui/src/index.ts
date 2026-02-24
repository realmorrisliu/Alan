#!/usr/bin/env bun
/**
 * Alan TUI - Terminal User Interface for Alan Agent Runtime
 * 
 * A terminal-based interface for interacting with the Alan agent daemon.
 * Features:
 * - Real-time WebSocket event streaming
 * - Session management
 * - Interactive chat interface
 * - Tool call visualization
 */

import blessed from 'blessed';
import { AlanClient } from './client';
import { EventRenderer } from './renderer';
import type { EventEnvelope } from './types';

const AGENTD_URL = process.env.AGENTD_URL || 'ws://localhost:8090';

class AlanTUI {
  private screen: blessed.Widgets.Screen;
  private chatBox: blessed.Widgets.Log;
  private inputBox: blessed.Widgets.Textbox;
  private statusBar: blessed.Widgets.Box;
  private client: AlanClient;
  private renderer: EventRenderer;
  private currentSessionId: string | null = null;

  constructor() {
    this.screen = blessed.screen({
      smartCSR: true,
      title: 'Alan Agent TUI',
    });

    this.setupUI();
    this.client = new AlanClient(AGENTD_URL);
    this.renderer = new EventRenderer(this.chatBox);
    this.setupEventHandlers();
  }

  private setupUI(): void {
    // Chat history box
    this.chatBox = blessed.log({
      top: 0,
      left: 0,
      width: '100%',
      height: '90%',
      border: { type: 'line' },
      style: {
        border: { fg: 'cyan' },
      },
      scrollable: true,
      alwaysScroll: true,
      tags: true,
      label: ' {bold}Alan Agent{/bold} ',
    });

    // Status bar
    this.statusBar = blessed.box({
      bottom: 1,
      left: 0,
      width: '100%',
      height: 1,
      content: '{center}Connecting...{/center}',
      tags: true,
      style: {
        fg: 'black',
        bg: 'cyan',
      },
    });

    // Input box
    this.inputBox = blessed.textbox({
      bottom: 0,
      left: 0,
      width: '100%',
      height: 1,
      inputOnFocus: true,
      style: {
        fg: 'white',
        bg: 'blue',
      },
    });

    this.screen.append(this.chatBox);
    this.screen.append(this.statusBar);
    this.screen.append(this.inputBox);

    // Key bindings
    this.screen.key(['escape', 'q', 'C-c'], () => {
      this.client.disconnect();
      process.exit(0);
    });

    this.screen.key(['C-l'], () => {
      this.chatBox.setContent('');
      this.screen.render();
    });

    this.inputBox.key('enter', () => {
      this.handleInput();
    });

    this.inputBox.focus();
  }

  private setupEventHandlers(): void {
    this.client.on('connected', () => {
      this.updateStatus('Connected');
      this.addSystemMessage('Connected to Alan agent daemon');
    });

    this.client.on('disconnected', () => {
      this.updateStatus('Disconnected');
      this.addSystemMessage('Disconnected from daemon');
    });

    this.client.on('error', (error: Error) => {
      this.updateStatus(`Error: ${error.message}`);
      this.addSystemMessage(`Error: ${error.message}`, 'error');
    });

    this.client.on('event', (envelope: EventEnvelope) => {
      this.renderer.renderEvent(envelope);
      this.screen.render();
    });

    this.client.on('session_created', (sessionId: string) => {
      this.currentSessionId = sessionId;
      this.addSystemMessage(`Session created: ${sessionId.slice(0, 8)}...`);
    });
  }

  private async handleInput(): Promise<void> {
    const text = this.inputBox.getValue().trim();
    if (!text) return;

    this.inputBox.clearValue();
    this.screen.render();

    // Add user message to chat
    this.addUserMessage(text);

    // Handle commands
    if (text.startsWith('/')) {
      await this.handleCommand(text);
      return;
    }

    // Send to agent
    if (!this.currentSessionId) {
      this.addSystemMessage('No active session. Use /new to create one.', 'warning');
      return;
    }

    try {
      await this.client.sendMessage(this.currentSessionId, text);
    } catch (error) {
      this.addSystemMessage(`Failed to send: ${(error as Error).message}`, 'error');
    }
  }

  private async handleCommand(text: string): Promise<void> {
    const [cmd, ...args] = text.slice(1).split(' ');

    switch (cmd) {
      case 'new':
        try {
          const sessionId = await this.client.createSession();
          this.currentSessionId = sessionId;
          await this.client.connectToSession(sessionId);
        } catch (error) {
          this.addSystemMessage(`Failed to create session: ${(error as Error).message}`, 'error');
        }
        break;

      case 'connect':
        if (!args[0]) {
          this.addSystemMessage('Usage: /connect <session-id>', 'warning');
          return;
        }
        try {
          this.currentSessionId = args[0];
          await this.client.connectToSession(args[0]);
        } catch (error) {
          this.addSystemMessage(`Failed to connect: ${(error as Error).message}`, 'error');
        }
        break;

      case 'sessions':
        try {
          const sessions = await this.client.listSessions();
          this.addSystemMessage(`Active sessions: ${sessions.length}`);
          sessions.forEach(s => {
            this.chatBox.log(`  ${s.id.slice(0, 8)}... ${s.status}`);
          });
        } catch (error) {
          this.addSystemMessage(`Failed to list sessions: ${(error as Error).message}`, 'error');
        }
        break;

      case 'help':
        this.showHelp();
        break;

      default:
        this.addSystemMessage(`Unknown command: /${cmd}. Type /help for available commands.`, 'warning');
    }
  }

  private showHelp(): void {
    this.chatBox.log('{bold}Available Commands:{/bold}');
    this.chatBox.log('  /new              - Create a new session');
    this.chatBox.log('  /connect <id>     - Connect to an existing session');
    this.chatBox.log('  /sessions         - List active sessions');
    this.chatBox.log('  /help             - Show this help message');
    this.chatBox.log('  Ctrl+C or q       - Quit');
    this.chatBox.log('  Ctrl+L            - Clear screen');
    this.chatBox.log('');
    this.chatBox.log('Simply type your message to chat with the agent.');
  }

  private addUserMessage(text: string): void {
    this.chatBox.log(`{bold}{green-fg}You:{/green-fg}{/bold} ${text}`);
  }

  private addSystemMessage(text: string, type: 'info' | 'warning' | 'error' = 'info'): void {
    const colors = {
      info: 'cyan',
      warning: 'yellow',
      error: 'red',
    };
    this.chatBox.log(`{${colors[type]}-fg}[System]{/${colors[type]}-fg} ${text}`);
  }

  private updateStatus(status: string): void {
    this.statusBar.setContent(`{center}Alan | ${status} | ${this.currentSessionId ? this.currentSessionId.slice(0, 8) + '...' : 'No Session'}{/center}`);
    this.screen.render();
  }

  public async start(): Promise<void> {
    this.chatBox.log('{bold}{cyan-fg}Welcome to Alan Agent TUI{/cyan-fg}{/bold}');
    this.chatBox.log('Type /help for available commands');
    this.chatBox.log('');

    try {
      await this.client.connect();
    } catch (error) {
      this.addSystemMessage(`Connection failed: ${(error as Error).message}`, 'error');
      this.addSystemMessage('Make sure agentd is running on ' + AGENTD_URL, 'warning');
    }

    this.screen.render();
  }
}

// Start the TUI
const tui = new AlanTUI();
tui.start().catch(console.error);
