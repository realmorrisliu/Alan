import React from "react";
import { Box, Text } from "ink";
import {
  getStructuredAnswer,
  getStructuredOptionCursor,
  moveStructuredOptionCursor,
  moveStructuredQuestion,
  moveStructuredSingleSelection,
  questionAnswerPreview,
  questionValidationError,
  selectStructuredSingleOption,
  structuredFormValidationError,
  toggleStructuredMultiOption,
} from "../structured-input.js";
import { parseSchemaDrivenYieldForm } from "../schema-driven-yield.js";
import type {
  AdaptiveSurfaceDefinition,
  AdaptiveSurfaceEventMessage,
  AdaptiveSurfaceInputContext,
  AdaptiveSurfaceKeyContext,
  AdaptiveSurfaceRenderContext,
} from "./types.js";
import { AdaptiveSurfacePanel } from "./shared.js";
import {
  structuredQuestionControls,
  structuredQuestionPositionLabel,
} from "./structured-input-surface.js";
import type { PendingYield } from "./yield-state.js";

function schemaYieldTitle(pendingYield: PendingYield): string {
  if (pendingYield.kind === "dynamic_tool") {
    return "Action required: dynamic tool";
  }
  return "Action required: custom yield";
}

function dynamicToolContextRows(
  payload: unknown,
): Array<{ label: string; value: string }> {
  const data =
    payload && typeof payload === "object" && !Array.isArray(payload)
      ? (payload as Record<string, unknown>)
      : null;
  if (!data || typeof data.tool_name !== "string") {
    return [];
  }

  return [
    { label: "tool", value: data.tool_name },
    {
      label: "arguments",
      value: JSON.stringify(data.arguments ?? {}, null, 2),
    },
  ];
}

function renderSchemaDrivenSurface({
  pendingYield,
  schemaForm,
}: AdaptiveSurfaceRenderContext) {
  const activeQuestion = schemaForm?.activeQuestion ?? null;
  const formState = schemaForm?.formState ?? null;
  const questions = schemaForm?.questions ?? [];
  const formError =
    formState && questions.length > 0
      ? structuredFormValidationError(formState, questions)
      : null;
  const contextRows = dynamicToolContextRows(pendingYield.payload);

  return (
    <AdaptiveSurfacePanel
      title={schemaYieldTitle(pendingYield)}
      requestId={pendingYield.requestId}
    >
      <Text>{schemaForm?.title ?? "Provide structured input"}</Text>
      {schemaForm?.prompt ? (
        <Text color="gray">{schemaForm.prompt}</Text>
      ) : null}
      {contextRows.map((row) => (
        <Box key={row.label} flexDirection="column">
          <Text color="gray">{row.label}</Text>
          <Text color={row.label === "tool" ? "cyan" : "gray"}>
            {row.value}
          </Text>
        </Box>
      ))}
      {activeQuestion && formState ? (
        <>
          <Text color="gray">
            {structuredQuestionPositionLabel(
              formState.activeQuestionIndex,
              questions,
            )}{" "}
            | {activeQuestion.required ? "required" : "optional"} |{" "}
            {activeQuestion.kind}
          </Text>
          {questions.map((question, index) => {
            const isActive = index === formState.activeQuestionIndex;
            const answerPreview = questionAnswerPreview(formState, question);
            const error = questionValidationError(formState, question);

            return (
              <Box key={question.id} flexDirection="column">
                <Text color={isActive ? "cyan" : undefined}>
                  {isActive ? "›" : " "} {question.label}
                  {question.required ? " *" : ""}: {answerPreview}
                </Text>
                <Text color="gray">
                  {question.id}: {question.prompt}
                </Text>
                {error && isActive ? <Text color="yellow">{error}</Text> : null}
              </Box>
            );
          })}
          {activeQuestion.helpText ? (
            <Text color="gray">{activeQuestion.helpText}</Text>
          ) : null}
          {activeQuestion.options?.map((option, index) => {
            const answer = getStructuredAnswer(formState, activeQuestion);
            const isSelected = Array.isArray(answer)
              ? answer.includes(option.value)
              : answer === option.value;
            const isCursor =
              getStructuredOptionCursor(formState, activeQuestion) === index;
            const marker =
              activeQuestion.kind === "multi_select"
                ? isSelected
                  ? "[x]"
                  : "[ ]"
                : isSelected
                  ? "(x)"
                  : "( )";

            return (
              <Text key={option.value} color={isCursor ? "cyan" : "gray"}>
                {isCursor ? "›" : " "} {index + 1}. {marker} {option.label}
                {option.description ? ` — ${option.description}` : ""}
              </Text>
            );
          })}
          {formError ? (
            <Text color="yellow">Submit blocked: {formError}</Text>
          ) : (
            <Text color="green">Form ready to submit.</Text>
          )}
          <Text color="gray">{structuredQuestionControls(activeQuestion)}</Text>
          <Text color="gray">Manual fallback: /resume &lt;json-object&gt;</Text>
        </>
      ) : (
        <Text color="gray">
          Loading schema-driven form... /resume &lt;json-object&gt;
        </Text>
      )}
    </AdaptiveSurfacePanel>
  );
}

function renderGenericSurface(context: AdaptiveSurfaceRenderContext) {
  if (context.schemaForm) {
    return renderSchemaDrivenSurface(context);
  }

  return (
    <AdaptiveSurfacePanel
      title={`Action required: ${context.pendingYield.kind}`}
      requestId={context.pendingYield.requestId}
    >
      <Text color="gray">Command: /resume &lt;json-object&gt;</Text>
    </AdaptiveSurfacePanel>
  );
}

function buildGenericAnnouncement(
  pendingYield: AdaptiveSurfaceRenderContext["pendingYield"],
) {
  const schemaForm = parseSchemaDrivenYieldForm(pendingYield.payload);
  const messages: AdaptiveSurfaceEventMessage[] = [
    {
      type: "system_warning",
      message:
        pendingYield.kind === "dynamic_tool"
          ? `Dynamic tool call pending (${pendingYield.requestId}).`
          : `Custom yield pending (${pendingYield.requestId}).`,
    },
  ];

  if (schemaForm) {
    messages.push({
      type: "system_message",
      message: schemaForm.title,
    });
    if (schemaForm.prompt) {
      messages.push({
        type: "system_message",
        message: schemaForm.prompt,
      });
    }
    messages.push({
      type: "system_message",
      message:
        "Use the adaptive form in the Action panel, or /resume <json> for raw structured fallback.",
    });
    return messages;
  }

  messages.push({
    type: "system_message",
    message:
      pendingYield.kind === "dynamic_tool"
        ? "Use /resume <json> to return a custom content payload."
        : "Use /resume <json> to continue with a custom payload.",
  });
  return messages;
}

function genericFooterHint(context: AdaptiveSurfaceRenderContext) {
  if (context.schemaForm) {
    return `${structuredQuestionControls(context.schemaForm.activeQuestion)} | /resume fallback`;
  }
  return "Resolve: /resume <json>";
}

function genericInputLabel({
  schemaForm,
  inputValue,
}: AdaptiveSurfaceInputContext) {
  if (!schemaForm) {
    return "Action";
  }
  return inputValue.startsWith("/") ? "Command" : "Answer / Command";
}

function genericInputPlaceholder({ schemaForm }: AdaptiveSurfaceInputContext) {
  if (!schemaForm) {
    return "Resolve pending yield with command...";
  }
  return schemaForm.activeQuestion?.kind === "text"
    ? `Answer: ${schemaForm.activeQuestion.label} (or /resume fallback)`
    : "Use adaptive controls above, or type /resume <json>";
}

function genericInputFocus({
  schemaForm,
  inputValue,
}: AdaptiveSurfaceInputContext) {
  if (!schemaForm) {
    return true;
  }

  return (
    !schemaForm.activeQuestion ||
    schemaForm.activeQuestion.kind === "text" ||
    inputValue.startsWith("/")
  );
}

function handleSchemaDrivenKey(context: AdaptiveSurfaceKeyContext) {
  const schemaForm = context.schemaForm;
  if (
    !schemaForm?.formState ||
    !schemaForm.activeQuestion ||
    !context.schemaFormControls
  ) {
    return false;
  }

  const { formState, questions, activeQuestion } = schemaForm;
  const { setFormState } = context.schemaFormControls;

  if (context.inputValue.startsWith("/")) {
    return false;
  }

  if (activeQuestion.kind !== "text" && context.input === "/") {
    context.setInputValue("/");
    return true;
  }

  if (context.key.ctrl && context.input === "n") {
    setFormState((previous) =>
      previous ? moveStructuredQuestion(previous, questions, 1) : previous,
    );
    return true;
  }

  if (context.key.ctrl && context.input === "p") {
    setFormState((previous) =>
      previous ? moveStructuredQuestion(previous, questions, -1) : previous,
    );
    return true;
  }

  if (context.key.ctrl && context.input === "s") {
    context.schemaFormControls.submitForm();
    return true;
  }

  if (activeQuestion.kind === "text") {
    return false;
  }

  if (context.key.upArrow || context.input === "k") {
    setFormState((previous) =>
      previous
        ? activeQuestion.kind === "single_select"
          ? moveStructuredSingleSelection(previous, activeQuestion, -1)
          : moveStructuredOptionCursor(previous, activeQuestion, -1)
        : previous,
    );
    return true;
  }

  if (context.key.downArrow || context.input === "j") {
    setFormState((previous) =>
      previous
        ? activeQuestion.kind === "single_select"
          ? moveStructuredSingleSelection(previous, activeQuestion, 1)
          : moveStructuredOptionCursor(previous, activeQuestion, 1)
        : previous,
    );
    return true;
  }

  if (context.input >= "1" && context.input <= "9") {
    const index = Number(context.input) - 1;
    if (index >= (activeQuestion.options?.length ?? 0)) {
      return true;
    }
    setFormState((previous) => {
      if (!previous) return previous;
      return activeQuestion.kind === "single_select"
        ? selectStructuredSingleOption(previous, activeQuestion, index)
        : toggleStructuredMultiOption(previous, activeQuestion, index);
    });
    return true;
  }

  if (activeQuestion.kind === "multi_select" && context.input === " ") {
    const cursor = getStructuredOptionCursor(formState, activeQuestion);
    const nextState = toggleStructuredMultiOption(
      formState,
      activeQuestion,
      cursor,
    );
    if (nextState === formState) {
      context.addSystemEvent(
        "system_warning",
        `${activeQuestion.label}: at most ${activeQuestion.maxSelections} selections allowed.`,
      );
      return true;
    }
    setFormState(nextState);
    return true;
  }

  if (context.key.return) {
    context.schemaFormControls.confirmActiveQuestion();
    return true;
  }

  return false;
}

export const dynamicToolSurface: AdaptiveSurfaceDefinition = {
  kind: "dynamic_tool",
  buildAnnouncement: buildGenericAnnouncement,
  render: renderGenericSurface,
  footerHint: genericFooterHint,
  inputLabel: genericInputLabel,
  inputPlaceholder: genericInputPlaceholder,
  inputFocus: genericInputFocus,
  handleInputKey: handleSchemaDrivenKey,
};

export const customYieldSurface: AdaptiveSurfaceDefinition = {
  kind: "custom",
  buildAnnouncement: buildGenericAnnouncement,
  render: renderGenericSurface,
  footerHint: genericFooterHint,
  inputLabel: genericInputLabel,
  inputPlaceholder: genericInputPlaceholder,
  inputFocus: genericInputFocus,
  handleInputKey: handleSchemaDrivenKey,
};
