import React from "react";
import { Text } from "ink";
import { confirmationOptions, confirmationSummary } from "../yield.js";
import type {
  AdaptiveSurfaceDefinition,
  AdaptiveSurfaceEventMessage,
  AdaptiveSurfaceInputContext,
  AdaptiveSurfaceRenderContext,
} from "./types.js";
import { AdaptiveSurfacePanel } from "./shared.js";

function renderConfirmationSurface({
  pendingYield,
}: AdaptiveSurfaceRenderContext) {
  const summary = confirmationSummary(pendingYield.payload);
  const options = confirmationOptions(pendingYield.payload);

  return (
    <AdaptiveSurfacePanel
      title="Action required: confirmation"
      requestId={pendingYield.requestId}
    >
      {summary ? <Text>{summary}</Text> : null}
      {options.length > 0 ? (
        <Text color="gray">options: {options.join(", ")}</Text>
      ) : null}
      <Text color="gray">
        Commands: /approve | /reject | /modify &lt;text&gt;
      </Text>
    </AdaptiveSurfacePanel>
  );
}

function buildConfirmationAnnouncement(
  pendingYield: AdaptiveSurfaceRenderContext["pendingYield"],
) {
  const messages: AdaptiveSurfaceEventMessage[] = [
    {
      type: "system_warning",
      message: `Approval required (${pendingYield.requestId}).`,
    },
  ];
  const summary = confirmationSummary(pendingYield.payload);
  const options = confirmationOptions(pendingYield.payload);

  if (summary) {
    messages.push({ type: "system_message", message: summary });
  }
  if (options.length > 0) {
    messages.push({
      type: "system_message",
      message: `Options: ${options.join(", ")}`,
    });
  }
  messages.push({
    type: "system_message",
    message: "Use /approve, /reject, or /modify <text>.",
  });
  return messages;
}

export const confirmationSurface: AdaptiveSurfaceDefinition = {
  kind: "confirmation",
  buildAnnouncement: buildConfirmationAnnouncement,
  render: renderConfirmationSurface,
  footerHint() {
    return "Resolve: /approve | /reject | /modify";
  },
  inputLabel(_: AdaptiveSurfaceInputContext) {
    return "Action";
  },
  inputPlaceholder(_: AdaptiveSurfaceInputContext) {
    return "Resolve pending yield with command...";
  },
};
