/**
 * Event Renderer - Renders Alan protocol events to the terminal
 */

import type { EventEnvelope, Event } from './types';

export class EventRenderer {
  private chatBox: { log: (text: string) => void };

  constructor(chatBox: { log: (text: string) => void }) {
    this.chatBox = chatBox;
  }

  public renderEvent(envelope: EventEnvelope): void {
    const { event } = envelope;

    switch (event.type) {
      case 'turn_started':
        this.renderTurnStarted();
        break;
      case 'turn_completed':
        this.renderTurnCompleted();
        break;
      case 'thinking':
        this.renderThinking(event.message || '');
        break;
      case 'thinking_complete':
        this.renderThinkingComplete();
        break;
      case 'reasoning_delta':
        this.renderReasoningDelta(event.chunk || '', event.is_final || false);
        break;
      case 'message_delta':
        this.renderMessageDelta(event.content || '');
        break;
      case 'message_delta_chunk':
        this.renderMessageDeltaChunk(event.chunk || '', event.is_final || false);
        break;
      case 'tool_call_started':
        this.renderToolCallStarted(event.tool_name || '', event.call_id || '');
        break;
      case 'tool_call_completed':
        this.renderToolCallCompleted(
          event.tool_name || '',
          event.call_id || '',
          event.success || false
        );
        break;
      case 'confirmation_required':
        this.renderConfirmationRequired(
          event.checkpoint_type || '',
          event.summary || '',
          event.options || []
        );
        break;
      case 'task_completed':
        this.renderTaskCompleted(event.summary || '');
        break;
      case 'error':
        this.renderError(event.message || '', event.recoverable || false);
        break;
      case 'plan_updated':
        this.renderPlanUpdated(event.items || [], event.explanation);
        break;
      case 'skills_loaded':
        this.renderSkillsLoaded(event.skill_ids || [], event.auto_selected || false);
        break;
      default:
        // Silently ignore unknown event types
        break;
    }
  }

  private renderTurnStarted(): void {
    this.chatBox.log('');
    this.chatBox.log('{grey-fg}───────────────────────────────{/grey-fg}');
  }

  private renderTurnCompleted(): void {
    this.chatBox.log('{grey-fg}───────────────────────────────{/grey-fg}');
    this.chatBox.log('');
  }

  private renderThinking(message: string): void {
    this.chatBox.log(`{cyan-fg}🤔 {italic}${message}{/italic}{/cyan-fg}`);
  }

  private renderThinkingComplete(): void {
    // No-op, thinking is complete
  }

  private renderReasoningDelta(chunk: string, isFinal: boolean): void {
    // Render reasoning chunks inline (could accumulate in a buffer for live display)
    if (isFinal) {
      this.chatBox.log('');
    }
  }

  private renderMessageDelta(content: string): void {
    this.chatBox.log(`{bold}{blue-fg}Alan:{/blue-fg}{/bold} ${this.escapeTags(content)}`);
  }

  private renderMessageDeltaChunk(chunk: string, isFinal: boolean): void {
    // For streaming chunks, we could accumulate them
    // For now, just log when complete
    if (isFinal) {
      this.chatBox.log('');
    }
  }

  private renderToolCallStarted(toolName: string, callId: string): void {
    this.chatBox.log(`{yellow-fg}🔧 Using tool: ${toolName}{/yellow-fg}`);
  }

  private renderToolCallCompleted(toolName: string, callId: string, success: boolean): void {
    const icon = success ? '✓' : '✗';
    const color = success ? 'green' : 'red';
    this.chatBox.log(`{${color}-fg}${icon} Tool ${toolName} ${success ? 'succeeded' : 'failed'}{/${color}-fg}`);
  }

  private renderConfirmationRequired(type: string, summary: string, options: string[]): void {
    this.chatBox.log('');
    this.chatBox.log(`{bold}{magenta-fg}⚠️  Confirmation Required: ${this.formatCheckpointType(type)}{/magenta-fg}{/bold}`);
    this.chatBox.log(`{magenta-fg}${this.escapeTags(summary)}{/magenta-fg}`);
    this.chatBox.log(`{magenta-fg}Options: ${options.join(', ')}{/magenta-fg}`);
    this.chatBox.log('');
  }

  private renderTaskCompleted(summary: string): void {
    this.chatBox.log('');
    this.chatBox.log(`{bold}{green-fg}✓ Task Completed{/green-fg}{/bold}`);
    this.chatBox.log(`{green-fg}${this.escapeTags(summary)}{/green-fg}`);
    this.chatBox.log('');
  }

  private renderError(message: string, recoverable: boolean): void {
    const severity = recoverable ? '⚠️' : '❌';
    this.chatBox.log(`{red-fg}${severity} Error: ${this.escapeTags(message)}{/red-fg}`);
  }

  private renderPlanUpdated(items: Array<{ id: string; content: string; status: string }>, explanation?: string): void {
    if (explanation) {
      this.chatBox.log(`{grey-fg}📋 ${this.escapeTags(explanation)}{/grey-fg}`);
    }
    
    items.forEach(item => {
      const icon = this.getPlanItemIcon(item.status);
      this.chatBox.log(`  ${icon} ${this.escapeTags(item.content)}`);
    });
  }

  private renderSkillsLoaded(skillIds: string[], autoSelected: boolean): void {
    const prefix = autoSelected ? '🤖 Auto-selected skills:' : '📚 Loaded skills:';
    this.chatBox.log(`{cyan-fg}${prefix} ${skillIds.join(', ')}{/cyan-fg}`);
  }

  private getPlanItemIcon(status: string): string {
    switch (status) {
      case 'completed':
        return '{green-fg}✓{/green-fg}';
      case 'in_progress':
        return '{yellow-fg}▶{/yellow-fg}';
      case 'pending':
        return '{grey-fg}○{/grey-fg}';
      default:
        return '{grey-fg}○{/grey-fg}';
    }
  }

  private formatCheckpointType(type: string): string {
    return type
      .split('_')
      .map(word => word.charAt(0).toUpperCase() + word.slice(1))
      .join(' ');
  }

  private escapeTags(text: string): string {
    // Escape curly braces that aren't blessed tags
    return text
      .replace(/{/g, '{open}')
      .replace(/}/g, '{close}')
      .replace(/\{open\}/g, '{')
      .replace(/\{close\}/g, '}');
  }
}
