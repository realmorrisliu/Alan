import React from "react";
import { Box, Text } from "ink";

export interface AdaptiveSurfacePanelProps {
  title: string;
  requestId: string;
  children: React.ReactNode;
}

export function AdaptiveSurfacePanel({
  title,
  requestId,
  children,
}: AdaptiveSurfacePanelProps) {
  return (
    <Box flexDirection="column" paddingX={1} marginY={1}>
      <Text color="yellow" bold>
        {title}
      </Text>
      <Text color="gray">request_id: {requestId}</Text>
      {children}
    </Box>
  );
}
