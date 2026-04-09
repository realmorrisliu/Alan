import React from "react";
import { Box, Text } from "ink";

export interface SummarySurfacePanelProps {
  title: string;
  children: React.ReactNode;
}

export function SummarySurfacePanel({
  title,
  children,
}: SummarySurfacePanelProps) {
  return (
    <Box
      borderStyle="round"
      borderColor="cyan"
      flexDirection="column"
      paddingX={1}
    >
      <Text color="cyan" bold>
        {title}
      </Text>
      {children}
    </Box>
  );
}
