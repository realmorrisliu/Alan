/**
 * Alan Electron - Renderer Process
 * 
 * This script runs in the renderer process (the web page).
 * It handles the UI and communicates with the main process via the exposed API.
 */

import { AlanWebClient } from './client.js';
import type { EventEnvelope, Session } from './types.js';

// DOM Elements
const chatContainer = document.getElementById('chat-container') as HTMLDivElement;
const messageInput = document.getElementById('message-input') as HTMLTextAreaElement;
const sendButton = document.getElementById('send-button') as HTMLButtonElement;
const sessionList = document.getElementById('session-list') as HTMLUListElement;
const newSessionButton = document.getElementById('new-session-button') as HTMLButtonElement;
const statusBar = document.getElementById('status-bar') as HTMLDivElement;
const thinkingIndicator = document.getElementById('thinking-indicator') as HTMLDivElement;

// State
let client: AlanWebClient | null = null;
let currentSessionId: string | null = null;
let isThinking = false;

// Initialize
async function init(): Promise<void> {
  try {
    const agentdUrl = await window.electronAPI.getAgentdUrl();
    client = new AlanWebClient(agentdUrl);
    
    setupEventHandlers();
    setupUIHandlers();
    
    // Load existing sessions
    await loadSessions();
    
    updateStatus('Ready');
  } catch (error) {
    console.error('Failed to initialize:', error);
    updateStatus('Error: Failed to connect');
  }
}

function setupEventHandlers(): void {
  if (!client) return;

  client.on('connected', () => {
    updateStatus('Connected');
  });

  client.on('disconnected', () => {
    updateStatus('Disconnected');
  });

  client.on('error', (error: Error) => {
    updateStatus(`Error: ${error.message}`);
  });

  client.on('event', (envelope: EventEnvelope) => {
    handleEvent(envelope);
  });

  client.on('session_created', (sessionId: string) => {
    currentSessionId = sessionId;
    addSystemMessage(`Session created: ${sessionId.slice(0, 8)}...`);
    loadSessions();
  });

  // Menu event handlers
  window.electronAPI.onMenuNewSession(() => {
    createNewSession();
  });

  window.electronAPI.onMenuCloseSession(() => {
    if (currentSessionId) {
      closeSession(currentSessionId);
    }
  });
}

function setupUIHandlers(): void {
  // Send message
  sendButton.addEventListener('click', sendMessage);
  
  messageInput.addEventListener('keydown', (e) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      sendMessage();
    }
  });

  // New session
  newSessionButton.addEventListener('click', createNewSession);
}

async function loadSessions(): Promise<void> {
  if (!client) return;

  try {
    const sessions = await client.listSessions();
    renderSessionList(sessions);
  } catch (error) {
    console.error('Failed to load sessions:', error);
  }
}

function renderSessionList(sessions: Session[]): void {
  sessionList.innerHTML = '';
  
  sessions.forEach(session => {
    const li = document.createElement('li');
    li.className = `session-item ${session.id === currentSessionId ? 'active' : ''}`;
    li.innerHTML = `
      <div class="session-name">Session ${session.id.slice(0, 8)}...</div>
      <div class="session-status ${session.status}">${session.status}</div>
    `;
    li.addEventListener('click', () => connectToSession(session.id));
    sessionList.appendChild(li);
  });
}

async function createNewSession(): Promise<void> {
  if (!client) return;

  try {
    updateStatus('Creating session...');
    const sessionId = await client.createSession();
    currentSessionId = sessionId;
    await client.connectToSession(sessionId);
    chatContainer.innerHTML = ''; // Clear chat
    updateStatus('Connected to new session');
  } catch (error) {
    updateStatus(`Failed to create session: ${(error as Error).message}`);
  }
}

async function connectToSession(sessionId: string): Promise<void> {
  if (!client) return;

  try {
    currentSessionId = sessionId;
    await client.connectToSession(sessionId);
    chatContainer.innerHTML = ''; // Clear chat
    updateStatus(`Connected to session ${sessionId.slice(0, 8)}...`);
    loadSessions();
  } catch (error) {
    updateStatus(`Failed to connect: ${(error as Error).message}`);
  }
}

async function closeSession(sessionId: string): Promise<void> {
  if (!client) return;

  try {
    // TODO: Implement close session API
    currentSessionId = null;
    chatContainer.innerHTML = '';
    await loadSessions();
    updateStatus('Session closed');
  } catch (error) {
    updateStatus(`Failed to close session: ${(error as Error).message}`);
  }
}

async function sendMessage(): Promise<void> {
  if (!client || !currentSessionId) {
    addSystemMessage('No active session. Create or select a session first.');
    return;
  }

  const text = messageInput.value.trim();
  if (!text) return;

  // Add user message to UI
  addUserMessage(text);
  messageInput.value = '';

  try {
    await client.sendMessage(currentSessionId, text);
  } catch (error) {
    addSystemMessage(`Failed to send: ${(error as Error).message}`);
  }
}

function handleEvent(envelope: EventEnvelope): void {
  const { event } = envelope;

  switch (event.type) {
    case 'thinking':
      showThinking(event.message || 'Thinking...');
      break;
    case 'thinking_complete':
      hideThinking();
      break;
    case 'message_delta':
      addAssistantMessage(event.content || '');
      hideThinking();
      break;
    case 'tool_call_started':
      addToolCallMessage(event.tool_name || '', 'started');
      break;
    case 'tool_call_completed':
      addToolCallMessage(event.tool_name || '', event.success ? 'completed' : 'failed');
      break;
    case 'confirmation_required':
      addConfirmationMessage(event.checkpoint_type || '', event.summary || '', event.options || []);
      break;
    case 'task_completed':
      addSystemMessage(`Task completed: ${event.summary}`);
      break;
    case 'error':
      addErrorMessage(event.message || 'Unknown error', event.recoverable || false);
      break;
    case 'plan_updated':
      updatePlan(event.items || []);
      break;
  }
}

// UI Helper functions
function addUserMessage(text: string): void {
  const messageDiv = document.createElement('div');
  messageDiv.className = 'message user-message';
  messageDiv.innerHTML = `
    <div class="message-header">You</div>
    <div class="message-content">${escapeHtml(text)}</div>
  `;
  chatContainer.appendChild(messageDiv);
  scrollToBottom();
}

function addAssistantMessage(text: string): void {
  // Check if we already have an assistant message we should append to
  const lastMessage = chatContainer.lastElementChild;
  if (lastMessage?.classList.contains('assistant-message')) {
    const content = lastMessage.querySelector('.message-content');
    if (content) {
      content.textContent += text;
      scrollToBottom();
      return;
    }
  }

  const messageDiv = document.createElement('div');
  messageDiv.className = 'message assistant-message';
  messageDiv.innerHTML = `
    <div class="message-header">Alan</div>
    <div class="message-content">${escapeHtml(text)}</div>
  `;
  chatContainer.appendChild(messageDiv);
  scrollToBottom();
}

function addSystemMessage(text: string): void {
  const messageDiv = document.createElement('div');
  messageDiv.className = 'message system-message';
  messageDiv.innerHTML = `<div class="message-content">${escapeHtml(text)}</div>`;
  chatContainer.appendChild(messageDiv);
  scrollToBottom();
}

function addToolCallMessage(toolName: string, status: 'started' | 'completed' | 'failed'): void {
  const messageDiv = document.createElement('div');
  messageDiv.className = `message tool-message ${status}`;
  const icon = status === 'started' ? '🔧' : status === 'completed' ? '✓' : '✗';
  messageDiv.innerHTML = `
    <div class="message-content">${icon} ${escapeHtml(toolName)} ${status}</div>
  `;
  chatContainer.appendChild(messageDiv);
  scrollToBottom();
}

function addConfirmationMessage(type: string, summary: string, options: string[]): void {
  const messageDiv = document.createElement('div');
  messageDiv.className = 'message confirmation-message';
  messageDiv.innerHTML = `
    <div class="message-header">Confirmation Required</div>
    <div class="message-content">
      <p><strong>${escapeHtml(type)}</strong></p>
      <p>${escapeHtml(summary)}</p>
      <div class="confirmation-options">
        ${options.map(opt => `<button class="confirm-btn" data-choice="${opt}">${opt}</button>`).join('')}
      </div>
    </div>
  `;
  
  // Add event listeners to buttons
  messageDiv.querySelectorAll('.confirm-btn').forEach(btn => {
    btn.addEventListener('click', async (e) => {
      const choice = (e.target as HTMLElement).dataset.choice as string;
      if (currentSessionId && client) {
        // Map button text to confirm choice
        const confirmChoice = choice.toLowerCase() as 'approve' | 'modify' | 'reject';
        await client.confirmCheckpoint(currentSessionId, 'checkpoint-id', confirmChoice);
      }
    });
  });
  
  chatContainer.appendChild(messageDiv);
  scrollToBottom();
}

function addErrorMessage(message: string, recoverable: boolean): void {
  const messageDiv = document.createElement('div');
  messageDiv.className = 'message error-message';
  messageDiv.innerHTML = `
    <div class="message-content">
      ${recoverable ? '⚠️' : '❌'} ${escapeHtml(message)}
    </div>
  `;
  chatContainer.appendChild(messageDiv);
  scrollToBottom();
}

function updatePlan(items: Array<{ id: string; content: string; status: string }>): void {
  // TODO: Implement plan visualization
  console.log('Plan updated:', items);
}

function showThinking(message: string): void {
  isThinking = true;
  thinkingIndicator.textContent = `🤔 ${message}`;
  thinkingIndicator.style.display = 'block';
}

function hideThinking(): void {
  isThinking = false;
  thinkingIndicator.style.display = 'none';
}

function updateStatus(message: string): void {
  statusBar.textContent = message;
}

function scrollToBottom(): void {
  chatContainer.scrollTop = chatContainer.scrollHeight;
}

function escapeHtml(text: string): string {
  const div = document.createElement('div');
  div.textContent = text;
  return div.innerHTML;
}

// Start the application
init().catch(console.error);
