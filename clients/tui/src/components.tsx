import React from 'react';
import { Box, Text } from 'ink';
import type { EventEnvelope } from './types';

export interface MessageListProps {
  events: EventEnvelope[];
}

export function MessageList({ events }: MessageListProps) {
  return (
    <Box flexDirection="column" width="100%">
      {events.map((envelope) => {
        // Use the event envelope event_id as a unique key
        const key = envelope.event_id;

        // Handle undefined type
        const eventType = envelope.type;
        if (!eventType) {
          return null;
        }

        switch (eventType as string) {
          case 'turn_started':
            return (
              <Box key={key} marginY={1}>
                <Text color="gray">───────────────────────────────</Text>
              </Box>
            );

          case 'turn_completed':
            return (
              <Box key={key} marginY={1}>
                <Text color="gray">───────────────────────────────</Text>
              </Box>
            );

          case 'thinking_delta':
            if (envelope.is_final) return null;
            return (
              <Box key={key}>
                <Text color="cyan">🤔 </Text>
                <Text color="cyan" italic>{envelope.chunk || 'Thinking...'}</Text>
              </Box>
            );

          case 'text_delta': {
            const chunk = envelope.chunk || '';
            if (!chunk) return null;
            return (
              <Box key={key}>
                <Text color="blue" bold>Alan: </Text>
                <Text>{chunk}</Text>
              </Box>
            );
          }

          case 'tool_call_started':
            return (
              <Box key={key}>
                <Text color="yellow">🔧 Using tool: {envelope.tool_name}</Text>
              </Box>
            );

          case 'tool_call_completed': {
            const success = envelope.success ?? true;
            return (
              <Box key={key}>
                <Text color={success ? "green" : "red"}>
                  {success ? '✓' : '✗'} Tool {envelope.tool_name} {success ? 'succeeded' : 'failed'}
                </Text>
              </Box>
            );
          }

          case 'task_completed':
            return (
              <Box key={key} marginY={1} flexDirection="column">
                <Text color="green" bold>✓ Task Completed</Text>
                <Text color="green">{envelope.summary}</Text>
              </Box>
            );

          case 'error':
            return (
              <Box key={key}>
                <Text color="red">
                  {envelope.recoverable ? '⚠️' : '❌'} Error: {envelope.message}
                </Text>
              </Box>
            );

          case 'session_created':
            // Custom client-side synthesized event
            return (
              <Box key={key}>
                <Text color="cyan">[System] Session created: {envelope.message?.slice(0, 8)}...</Text>
              </Box>
            );

          case 'system_message':
            // Custom client-side synthesized event
            return (
              <Box key={key}>
                <Text color="cyan">[System] {envelope.message}</Text>
              </Box>
            );

          case 'system_error':
            // Custom client-side synthesized event
            return (
              <Box key={key}>
                <Text color="red">[System] Error: {envelope.message}</Text>
              </Box>
            );

          case 'system_warning':
            // Custom client-side synthesized event
            return (
              <Box key={key}>
                <Text color="yellow">[System] Warning: {envelope.message}</Text>
              </Box>
            );

          case 'user_message':
            // Custom client-side synthesized event
            return (
              <Box key={key}>
                <Text color="green" bold>You: </Text>
                <Text>{envelope.message}</Text>
              </Box>
            );

          default:
            return null;
        }
      })}
    </Box>
  );
}
