import React from "react";
import { Box, Text } from "ink";
import {
  confirmationActionOptions,
  confirmationDetails,
  confirmationSummary,
} from "../yield.js";
import type {
  AdaptiveSurfaceDefinition,
  AdaptiveSurfaceEventMessage,
  AdaptiveSurfaceInputContext,
  AdaptiveSurfaceKeyContext,
  AdaptiveSurfaceRenderContext,
} from "./types.js";
import { AdaptiveSurfacePanel } from "./shared.js";

interface ConfirmationDetailRow {
  label: string;
  value: string;
  color?: string;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

function humanizeAction(action: string): string {
  return action
    .split(/[_\s]+/)
    .filter(Boolean)
    .map((part) => part[0].toUpperCase() + part.slice(1))
    .join(" ");
}

function actionShortcut(action: string, index: number): string {
  if (action === "approve") return "A";
  if (action === "modify") return "M";
  if (action === "reject") return "R";
  return String(index + 1);
}

function formatDetailValue(value: unknown): string {
  if (value === null || value === undefined) {
    return "null";
  }
  if (typeof value === "string") {
    return value;
  }
  if (
    typeof value === "number" ||
    typeof value === "boolean" ||
    typeof value === "bigint"
  ) {
    return String(value);
  }
  if (Array.isArray(value)) {
    return value.every(
      (item) =>
        typeof item === "string" ||
        typeof item === "number" ||
        typeof item === "boolean",
    )
      ? value.join(", ")
      : JSON.stringify(value, null, 2);
  }

  return JSON.stringify(value, null, 2);
}

function detailRowColor(key: string): string | undefined {
  if (
    key === "command" ||
    key === "tool" ||
    key === "tool_name" ||
    key.endsWith("_path") ||
    key === "path"
  ) {
    return "cyan";
  }
  if (key === "diff" || key === "arguments" || key.includes("preview")) {
    return "gray";
  }
  return undefined;
}

export function buildConfirmationDetailRows(
  details: Record<string, unknown> | null,
): ConfirmationDetailRow[] {
  if (!details) {
    return [];
  }

  const rows: ConfirmationDetailRow[] = [];

  for (const [key, value] of Object.entries(details)) {
    if (isRecord(value) && key === "replay_tool_call") {
      if (typeof value.name === "string") {
        rows.push({ label: "replay tool", value: value.name, color: "cyan" });
      }
      if ("arguments" in value) {
        rows.push({
          label: "arguments",
          value: formatDetailValue(value.arguments),
          color: "gray",
        });
      }
      continue;
    }

    if (isRecord(value)) {
      for (const [nestedKey, nestedValue] of Object.entries(value)) {
        const color = detailRowColor(`${key}.${nestedKey}`);
        rows.push({
          label: `${key}.${nestedKey}`,
          value: formatDetailValue(nestedValue),
          ...(color ? { color } : {}),
        });
      }
      continue;
    }

    const color = detailRowColor(key);
    rows.push({
      label: key.replace(/_/g, " "),
      value: formatDetailValue(value),
      ...(color ? { color } : {}),
    });
  }

  return rows;
}

export function preferredConfirmationActionIndex(options: string[]): number {
  const approveIndex = options.findIndex((option) => option === "approve");
  return approveIndex >= 0 ? approveIndex : 0;
}

function executeConfirmationAction(
  action: string,
  context: AdaptiveSurfaceKeyContext,
): void {
  if (action === "approve") {
    context.submitPendingYield({ choice: "approve" });
    return;
  }
  if (action === "reject") {
    context.submitPendingYield({ choice: "reject" });
    return;
  }
  if (action === "modify") {
    context.setInputValue("/modify ");
    return;
  }

  context.submitPendingYield({ choice: action });
}

function renderConfirmationSurface({
  pendingYield,
  confirmation,
}: AdaptiveSurfaceRenderContext) {
  const summary = confirmationSummary(pendingYield.payload);
  const options = confirmationActionOptions(pendingYield.payload);
  const detailRows = buildConfirmationDetailRows(
    confirmationDetails(pendingYield.payload),
  );
  const actionIndex =
    confirmation?.actionIndex ?? preferredConfirmationActionIndex(options);

  return (
    <AdaptiveSurfacePanel
      title="Action required: confirmation"
      requestId={pendingYield.requestId}
    >
      {summary ? <Text bold>{summary}</Text> : null}
      {detailRows.length > 0 ? (
        <>
          <Text color="gray">Details</Text>
          {detailRows.map((row) => (
            <Box key={`${row.label}:${row.value}`} flexDirection="column">
              <Text color="gray">{row.label}</Text>
              <Text color={row.color}>{row.value}</Text>
            </Box>
          ))}
        </>
      ) : null}
      <Box marginTop={1} flexWrap="wrap">
        {options.map((option, index) => {
          const isActive = index === actionIndex;
          return (
            <Box key={option} marginRight={1}>
              <Text
                bold
                color={isActive ? "black" : "yellow"}
                backgroundColor={isActive ? "yellow" : undefined}
              >
                {actionShortcut(option, index)} {humanizeAction(option)}
              </Text>
            </Box>
          );
        })}
      </Box>
      <Text color="gray">
        Enter confirm | ←/→ select | A approve | M modify | R reject | slash
        commands still work
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
  const options = confirmationActionOptions(pendingYield.payload);

  if (summary) {
    messages.push({ type: "system_message", message: summary });
  }
  messages.push({
    type: "system_message",
    message: `Options: ${options.join(", ")}`,
  });
  messages.push({
    type: "system_message",
    message:
      "Use Enter/arrow shortcuts in the Action panel, or /approve, /reject, /modify <text>.",
  });
  return messages;
}

function confirmationFooterHint({
  confirmation,
}: AdaptiveSurfaceRenderContext) {
  const activeAction =
    confirmation?.options[confirmation.actionIndex] ?? "approve";
  return `Confirm: Enter ${activeAction} | ←/→ select | A approve | M modify | R reject`;
}

function confirmationLabel({ inputValue }: AdaptiveSurfaceInputContext) {
  return inputValue.startsWith("/modify") ? "Modify" : "Action";
}

function confirmationPlaceholder({ inputValue }: AdaptiveSurfaceInputContext) {
  return inputValue.startsWith("/modify")
    ? "Describe required modifications..."
    : "Use the confirmation card or /approve /reject /modify";
}

function handleConfirmationKey(context: AdaptiveSurfaceKeyContext) {
  if (!context.confirmation || !context.confirmationControls) {
    return false;
  }

  const { options, actionIndex } = context.confirmation;
  if (context.inputValue.startsWith("/")) {
    return false;
  }

  if (context.key.leftArrow || context.input === "h") {
    context.confirmationControls.setActionIndex(
      (previous) => (previous - 1 + options.length) % options.length,
    );
    return true;
  }

  if (context.key.rightArrow || context.input === "l") {
    context.confirmationControls.setActionIndex(
      (previous) => (previous + 1) % options.length,
    );
    return true;
  }

  if (context.input >= "1" && context.input <= "9") {
    const index = Number(context.input) - 1;
    if (index < options.length) {
      context.confirmationControls.setActionIndex(index);
    }
    return true;
  }

  const shortcut =
    context.input === "a"
      ? "approve"
      : context.input === "m"
        ? "modify"
        : context.input === "r"
          ? "reject"
          : null;
  if (shortcut && options.includes(shortcut)) {
    executeConfirmationAction(shortcut, context);
    return true;
  }

  if (context.key.return && options[actionIndex]) {
    executeConfirmationAction(options[actionIndex], context);
    return true;
  }

  return false;
}

export const confirmationSurface: AdaptiveSurfaceDefinition = {
  kind: "confirmation",
  buildAnnouncement: buildConfirmationAnnouncement,
  render: renderConfirmationSurface,
  footerHint: confirmationFooterHint,
  inputLabel: confirmationLabel,
  inputPlaceholder: confirmationPlaceholder,
  handleInputKey: handleConfirmationKey,
};
