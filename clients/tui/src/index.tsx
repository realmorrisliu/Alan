#!/usr/bin/env bun
/**
 * Alan TUI - 完整的终端交互工具
 * 
 * 参考 pi-mono 设计：TUI 自动管理后端，用户只需运行 `alan` 即可开始工作
 */

import React, { useState, useEffect, useRef } from 'react';
import { render, Box, Text, useInput, useApp } from 'ink';
import TextInput from 'ink-text-input';
import { existsSync } from 'node:fs';
import { join } from 'node:path';
import { homedir } from 'node:os';
import { AlanClient } from './client.js';
import type { EventEnvelope, DaemonStatus } from './types.js';
import { MessageList } from './components.js';
import { InitWizard } from './init.js';

// 配置
const AGENTD_URL = process.env.ALAN_AGENTD_URL;
const AUTO_MANAGE = !AGENTD_URL; // 如果没有设置 URL，自动管理 agentd
const VERBOSE = process.env.ALAN_VERBOSE === '1';

// 启动模式信息
const STARTUP_INFO = {
  mode: AGENTD_URL ? 'remote' : 'embedded' as const,
  url: AGENTD_URL || 'ws://127.0.0.1:8090',
};

// 检查是否需要首次启动设置
function needsFirstTimeSetup(): boolean {
  // 远程模式不需要本地配置
  if (AGENTD_URL) return false;

  const configPath = join(homedir(), '.alan', 'config.toml');
  return !existsSync(configPath);
}

function App() {
  const { exit } = useApp();
  const [needsSetup, setNeedsSetup] = useState(needsFirstTimeSetup());
  const [inputValue, setInputValue] = useState('');
  const [status, setStatus] = useState<'connecting' | 'connected' | 'error'>('connecting');
  const [statusMessage, setStatusMessage] = useState('Starting...');
  const [currentSessionId, setCurrentSessionId] = useState<string | null>(null);
  const [events, setEvents] = useState<EventEnvelope[]>([]);
  const [daemonStatus, setDaemonStatus] = useState<DaemonStatus | null>(null);

  // Use ref to keep a persistent client around
  const clientRef = useRef<AlanClient | null>(null);

  // Helper to add fake system events for the UI
  // Note: Event fields are flattened (matching server-side EventEnvelope with #[serde(flatten)])
  const addSystemEvent = (type: string, message: string) => {
    setEvents(prev => [...prev, {
      event_id: crypto.randomUUID(),
      sequence: 0,
      session_id: currentSessionId || '',
      turn_id: 'system',
      item_id: 'system',
      timestamp_ms: Date.now(),
      type: type as any,
      message
    }]);
  };

  // 初始化向导完成
  const handleSetupComplete = () => {
    setNeedsSetup(false);
  };

  useEffect(() => {
    // 如果需要首次设置，不启动客户端
    if (needsSetup) {
      return;
    }

    const client = new AlanClient({
      url: STARTUP_INFO.url,
      autoManageDaemon: AUTO_MANAGE,
      verbose: VERBOSE,
    });
    clientRef.current = client;

    // 设置事件监听器
    client.on('connected', () => {
      setStatus('connected');
      setStatusMessage(STARTUP_INFO.mode === 'embedded' ? 'Ready' : `Connected to ${STARTUP_INFO.url}`);
    });

    client.on('disconnected', () => {
      setStatus('error');
      setStatusMessage('Disconnected');
      addSystemEvent('system_message', 'Disconnected from agent');
    });

    client.on('error', (error: Error) => {
      setStatus('error');
      setStatusMessage(`Error: ${error.message}`);
      addSystemEvent('system_error', error.message);
    });

    client.on('event', (envelope: EventEnvelope) => {
      setEvents(prev => [...prev, envelope]);
    });

    client.on('session_created', (sessionId: string) => {
      setCurrentSessionId(sessionId);
      addSystemEvent('session_created', sessionId);
    });

    // 检测当前目录的 workspace
    const detectWorkspaceDir = async (): Promise<string | undefined> => {
      const cwd = process.cwd();

      // 检查当前目录是否有 .alan 子目录
      try {
        const { existsSync } = await import('node:fs');
        const { join } = await import('node:path');

        if (existsSync(join(cwd, '.alan'))) {
          addSystemEvent('system_message', `Detected workspace: ${cwd}`);
          return cwd;
        }
      } catch {
        // ignore
      }

      return undefined;
    };

    // 初始化连接
    const init = async () => {
      try {
        setStatusMessage(STARTUP_INFO.mode === 'embedded'
          ? 'Starting agent daemon...'
          : 'Connecting to agent...'
        );

        await client.connect();

        // 如果是自动模式，更新 daemon 状态
        if (AUTO_MANAGE) {
          const daemonStatus = client.getDaemonStatus();
          if (daemonStatus) {
            setDaemonStatus(daemonStatus);
          }
        }

        try {
          // 检测 workspace
          const workspaceDir = await detectWorkspaceDir();

          if (workspaceDir) {
            addSystemEvent('system_message', `Creating session for workspace: ${workspaceDir}...`);
          } else {
            addSystemEvent('system_message', 'Auto-creating session on default workspace...');
          }

          const sessionId = await client.createSession({ workspace_dir: workspaceDir });
          await client.connectToSession(sessionId);
          addSystemEvent('system_message', `Alan agent ready. You can type your request directly.`);
        } catch (error) {
          const msg = (error as Error).message;
          addSystemEvent('system_error', msg); // Removed redundant "Failed to auto-create session: " prefix since we throw it with that prefix

          if (msg.includes('LLM') || msg.includes('llm') || msg.includes('model') || msg.includes('key')) {
            addSystemEvent('system_message', '提示: 看起来是 LLM 配置问题');
            addSystemEvent('system_message', '  请检查 ~/.alan/config.toml');
          } else if (msg.includes('500') || msg.includes('Internal Server Error')) {
            addSystemEvent('system_message', '提示: daemon 内部错误，请检查 agentd 日志');
          }
          addSystemEvent('system_message', 'Type /new to try again, or /help for commands.');
        }

      } catch (error) {
        const message = (error as Error).message;
        setStatus('error');
        setStatusMessage(`Failed: ${message}`);
        addSystemEvent('system_error', `Connection failed: ${message}`);

        if (STARTUP_INFO.mode === 'embedded') {
          addSystemEvent('system_message',
            'Make sure you have built agentd: cargo build --release -p alan-agentd'
          );
        }
      }
    };

    init();

    // 优雅退出处理
    const cleanup = async () => {
      await client.shutdown();
    };

    // 监听进程信号
    const handleExit = () => {
      cleanup().then(() => {
        process.exit(0);
      });
    };

    process.on('SIGINT', handleExit);
    process.on('SIGTERM', handleExit);

    return () => {
      process.off('SIGINT', handleExit);
      process.off('SIGTERM', handleExit);
      cleanup();
    };
  }, [needsSetup]); // 依赖 needsSetup，完成后重新运行

  // Handle generic app-wide keys (e.g. exit on ctrl+c)
  useInput((input, key) => {
    if (key.ctrl && input === 'c') {
      exit();
    }
  });

  const handleSubmit = async (text: string) => {
    const client = clientRef.current;
    if (!client) return;

    const trimmed = text.trim();
    if (!trimmed) return;

    setInputValue('');
    addSystemEvent('user_message', trimmed);

    if (trimmed.startsWith('/')) {
      await handleCommand(trimmed, client);
      return;
    }

    if (!currentSessionId) {
      addSystemEvent('system_warning', 'No active session. Use /new to create one.');
      return;
    }

    try {
      await client.sendMessage(currentSessionId, trimmed);
    } catch (error) {
      addSystemEvent('system_error', `Failed to send: ${(error as Error).message}`);
    }
  };

  const handleCommand = async (text: string, client: AlanClient) => {
    const [cmd, ...args] = text.slice(1).split(' ');

    switch (cmd) {
      case 'new':
        try {
          addSystemEvent('system_message', 'Creating new session...');
          const sessionId = await client.createSession();
          setCurrentSessionId(sessionId);
          await client.connectToSession(sessionId);
          addSystemEvent('system_message', `Session created and connected`);
        } catch (error) {
          const msg = (error as Error).message;
          addSystemEvent('system_error', msg);

          if (msg.includes('LLM') || msg.includes('llm') || msg.includes('model') || msg.includes('key')) {
            addSystemEvent('system_message', '提示: 看起来是 LLM 配置问题');
            addSystemEvent('system_message', '  请检查 ~/.alan/config.toml');
          } else if (msg.includes('500') || msg.includes('Internal Server Error')) {
            addSystemEvent('system_message', '提示: daemon 内部错误，请检查 agentd 日志');
          }
        }
        break;

      case 'connect':
        if (!args[0]) {
          addSystemEvent('system_warning', 'Usage: /connect <session-id>');
          return;
        }
        try {
          addSystemEvent('system_message', `Connecting to session ${args[0].slice(0, 8)}...`);
          setCurrentSessionId(args[0]);
          await client.connectToSession(args[0]);
          addSystemEvent('system_message', 'Connected');
        } catch (error) {
          addSystemEvent('system_error', `Failed to connect: ${(error as Error).message}`);
        }
        break;

      case 'sessions':
        try {
          const sessions = await client.listSessions();
          addSystemEvent('system_message', `Active sessions: ${sessions.length}`);
          sessions.forEach(s => {
            addSystemEvent('system_message', `  ${s.session_id.slice(0, 8)}... ${s.active ? 'active' : 'inactive'}`);
          });
        } catch (error) {
          addSystemEvent('system_error', `Failed to list sessions: ${(error as Error).message}`);
        }
        break;

      case 'status':
        if (AUTO_MANAGE) {
          const status = client.getDaemonStatus();
          if (status) {
            addSystemEvent('system_message', `Daemon: ${status.state}${status.pid ? ` (pid: ${status.pid})` : ''}`);
          }
        } else {
          const running = await client.isDaemonRunning();
          addSystemEvent('system_message', `Remote agent: ${running ? 'online' : 'offline'}`);
        }
        break;

      case 'help':
        addSystemEvent('system_message', 'Available Commands:');
        addSystemEvent('system_message', '  /new           - Create a new session');
        addSystemEvent('system_message', '  /connect <id>  - Connect to an existing session');
        addSystemEvent('system_message', '  /sessions      - List active sessions');
        addSystemEvent('system_message', '  /status        - Show daemon status');
        addSystemEvent('system_message', '  /help          - Show this help');
        addSystemEvent('system_message', '  /exit          - Exit (or press Ctrl+C)');
        break;

      case 'exit':
      case 'quit':
        exit();
        break;

      default:
        addSystemEvent('system_warning', `Unknown command: /${cmd}. Type /help for available commands.`);
    }
  };

  // 如果需要首次设置，显示初始化向导
  if (needsSetup) {
    return <InitWizard onComplete={handleSetupComplete} />;
  }

  // 状态栏颜色
  const getStatusColor = () => {
    switch (status) {
      case 'connected': return 'green';
      case 'connecting': return 'yellow';
      case 'error': return 'red';
      default: return 'gray';
    }
  };

  return (
    <Box flexDirection="column" height={process.stdout.rows || 24} width="100%">
      {/* Title */}
      <Box borderStyle="single" borderColor="cyan" paddingX={1}>
        <Text bold>Alan Agent</Text>
        <Text color="gray"> {STARTUP_INFO.mode === 'embedded' ? '(local)' : '(remote)'}</Text>
      </Box>

      {/* Message List */}
      <Box flexGrow={1} flexDirection="column" paddingX={1} overflowY="hidden">
        <MessageList events={events.slice(-15)} />
      </Box>

      {/* Status Bar */}
      <Box backgroundColor={getStatusColor()} paddingX={1}>
        <Text color="black">
          {status === 'connected' ? '●' : status === 'connecting' ? '◐' : '○'} {statusMessage}
          {currentSessionId ? ` | Session: ${currentSessionId.slice(0, 8)}...` : ''}
        </Text>
      </Box>

      {/* Input Box */}
      <Box backgroundColor="blue" paddingX={1}>
        <Text color="white" bold>{'> '} </Text>
        <TextInput
          value={inputValue}
          onChange={setInputValue}
          onSubmit={handleSubmit}
        />
      </Box>
    </Box>
  );
}

// Start the app
render(<App />);
