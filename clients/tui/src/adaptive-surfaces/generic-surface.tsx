import React from "react";
import { Text } from "ink";
import type {
  AdaptiveSurfaceDefinition,
  AdaptiveSurfaceInputContext,
  AdaptiveSurfaceRenderContext,
} from "./types.js";
import { AdaptiveSurfacePanel } from "./shared.js";

function renderGenericSurface({ pendingYield }: AdaptiveSurfaceRenderContext) {
  return (
    <AdaptiveSurfacePanel
      title={`Action required: ${pendingYield.kind}`}
      requestId={pendingYield.requestId}
    >
      <Text color="gray">Command: /resume &lt;json-object&gt;</Text>
    </AdaptiveSurfacePanel>
  );
}

function buildGenericAnnouncement(
  pendingYield: AdaptiveSurfaceRenderContext["pendingYield"],
) {
  if (pendingYield.kind === "dynamic_tool") {
    return [
      {
        type: "system_warning" as const,
        message: `Dynamic tool call pending (${pendingYield.requestId}).`,
      },
      {
        type: "system_message" as const,
        message: "Use /resume <json> to return a custom content payload.",
      },
    ];
  }

  return [
    {
      type: "system_warning" as const,
      message: `Custom yield pending (${pendingYield.requestId}).`,
    },
    {
      type: "system_message" as const,
      message: "Use /resume <json> to continue with a custom payload.",
    },
  ];
}

export const dynamicToolSurface: AdaptiveSurfaceDefinition = {
  kind: "dynamic_tool",
  buildAnnouncement: buildGenericAnnouncement,
  render: renderGenericSurface,
  footerHint() {
    return "Resolve: /resume <json>";
  },
  inputLabel(_: AdaptiveSurfaceInputContext) {
    return "Action";
  },
  inputPlaceholder(_: AdaptiveSurfaceInputContext) {
    return "Resolve pending yield with command...";
  },
};

export const customYieldSurface: AdaptiveSurfaceDefinition = {
  kind: "custom",
  buildAnnouncement: buildGenericAnnouncement,
  render: renderGenericSurface,
  footerHint() {
    return "Resolve: /resume <json>";
  },
  inputLabel(_: AdaptiveSurfaceInputContext) {
    return "Action";
  },
  inputPlaceholder(_: AdaptiveSurfaceInputContext) {
    return "Resolve pending yield with command...";
  },
};
