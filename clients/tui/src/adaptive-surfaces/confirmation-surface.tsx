import React from "react";
import { Box, Text } from "ink";
import {
  confirmationActionOptions,
  confirmationDefaultOption,
  confirmationDetails,
  confirmationIsDangerous,
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
    key === "call_id" ||
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

function detailRowPriority(label: string): number {
  if (label === "command") return 0;
  if (label === "path") return 1;
  if (label === "tool" || label === "replay tool") return 2;
  if (label === "call id" || label === "replay call id") return 3;
  if (label === "arguments") return 4;
  if (label === "diff") return 5;
  if (label.startsWith("policy.")) return 6;
  if (label.startsWith("replay tool ")) return 7;
  return 20;
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
      const replayToolName =
        typeof value.tool_name === "string"
          ? value.tool_name
          : typeof value.name === "string"
            ? value.name
            : null;

      if (replayToolName) {
        rows.push({
          label: "replay tool",
          value: replayToolName,
          color: "cyan",
        });
      }
      if (typeof value.call_id === "string") {
        rows.push({
          label: "replay call id",
          value: value.call_id,
          color: "cyan",
        });
      }
      if ("arguments" in value) {
        rows.push({
          label: "arguments",
          value: formatDetailValue(value.arguments),
          color: "gray",
        });
      }
      for (const [nestedKey, nestedValue] of Object.entries(value)) {
        if (
          nestedKey === "tool_name" ||
          nestedKey === "name" ||
          nestedKey === "call_id" ||
          nestedKey === "arguments"
        ) {
          continue;
        }
        const color = detailRowColor(nestedKey);
        rows.push({
          label: `replay tool ${nestedKey.replace(/_/g, " ")}`,
          value: formatDetailValue(nestedValue),
          ...(color ? { color } : {}),
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

  return rows.sort((left, right) => {
    const priorityDelta =
      detailRowPriority(left.label) - detailRowPriority(right.label);
    if (priorityDelta !== 0) {
      return priorityDelta;
    }
    return left.label.localeCompare(right.label);
  });
}

export function preferredConfirmationActionIndex(
  options: string[],
  defaultOption?: string | null,
): number {
  if (defaultOption) {
    const defaultIndex = options.findIndex((option) => option === defaultOption);
    if (defaultIndex >= 0) {
      return defaultIndex;
    }
  }
  const approveIndex = options.findIndex((option) => option === "approve");
  return approveIndex >= 0 ? approveIndex : 0;
}

function confirmationActionStyle(
  option: string,
  isActive: boolean,
  dangerousConfirmation: boolean,
): { color: string; backgroundColor?: string } {
  const dangerousAction = dangerousConfirmation && option === "approve";

  if (isActive && dangerousAction) {
    return { color: "white", backgroundColor: "red" };
  }
  if (isActive) {
    return { color: "black", backgroundColor: "yellow" };
  }
  if (dangerousAction) {
    return { color: "red" };
  }
  return { color: "yellow" };
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
  const defaultOption = confirmationDefaultOption(pendingYield.payload);
  const dangerousConfirmation = confirmationIsDangerous(pendingYield.payload);
  const detailRows = buildConfirmationDetailRows(
    confirmationDetails(pendingYield.payload),
  );
  const actionIndex =
    confirmation?.actionIndex ??
    preferredConfirmationActionIndex(options, defaultOption);

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
          const style = confirmationActionStyle(
            option,
            isActive,
            dangerousConfirmation,
          );
          return (
            <Box key={option} marginRight={1}>
              <Text bold color={style.color} backgroundColor={style.backgroundColor}>
                {actionShortcut(option, index)} {humanizeAction(option)}
              </Text>
            </Box>
          );
        })}
      </Box>
      {defaultOption ? (
        <Text color="gray">Default action: {humanizeAction(defaultOption)}</Text>
      ) : null}
      {dangerousConfirmation ? (
        <Text color="red">
          Approving will allow a dangerous action. Review details carefully.
        </Text>
      ) : null}
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
