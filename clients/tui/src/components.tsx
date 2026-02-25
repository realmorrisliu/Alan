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
        const { event } = envelope;

        // Use the event envelope event_id as a unique key
        const key = envelope.event_id;

        // Handle undefined event
        if (!event) {
          return null;
        }

        switch (event.type as string) {
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

          case 'thinking':
            return (
              <Box key={key}>
                <Text color="cyan">🤔 </Text>
                <Text color="cyan" italic>{event.message || 'Thinking...'}</Text>
              </Box>
            );

          case 'message_delta':
            return (
              <Box key={key}>
                <Text color="blue" bold>Alan: </Text>
                <Text>{event.content}</Text>
              </Box>
            );

          case 'tool_call_started':
            return (
              <Box key={key}>
                <Text color="yellow">🔧 Using tool: {event.tool_name}</Text>
              </Box>
            );

          case 'tool_call_completed': {
            const success = event.success ?? true;
            return (
              <Box key={key}>
                <Text color={success ? "green" : "red"}>
                  {success ? '✓' : '✗'} Tool {event.tool_name} {success ? 'succeeded' : 'failed'}
                </Text>
              </Box>
            );
          }

          case 'task_completed':
            return (
              <Box key={key} marginY={1} flexDirection="column">
                <Text color="green" bold>✓ Task Completed</Text>
                <Text color="green">{event.summary}</Text>
              </Box>
            );

          case 'error':
            return (
              <Box key={key}>
                <Text color="red">
                  {event.recoverable ? '⚠️' : '❌'} Error: {event.message}
                </Text>
              </Box>
            );

          case 'session_created':
            // Custom client-side synthesized event
            return (
              <Box key={key}>
                <Text color="cyan">[System] Session created: {event.message?.slice(0, 8)}...</Text>
              </Box>
            );

          case 'system_message':
            // Custom client-side synthesized event
            return (
              <Box key={key}>
                <Text color="cyan">[System] {event.message}</Text>
              </Box>
            );

          case 'system_error':
            // Custom client-side synthesized event
            return (
              <Box key={key}>
                <Text color="red">[System] Error: {event.message}</Text>
              </Box>
            );

          case 'system_warning':
            // Custom client-side synthesized event
            return (
              <Box key={key}>
                <Text color="yellow">[System] Warning: {event.message}</Text>
              </Box>
            );

          case 'user_message':
            // Custom client-side synthesized event
            return (
              <Box key={key}>
                <Text color="green" bold>You: </Text>
                <Text>{event.message}</Text>
              </Box>
            );

          default:
            return null;
        }
      })}
    </Box>
  );
}
