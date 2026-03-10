import React from "react";
import { Text, Box } from "ink";
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
import type { StructuredQuestion } from "../yield.js";
import {
  structuredPrompt,
  structuredQuestions,
  structuredTitle,
} from "../yield.js";
import type {
  AdaptiveSurfaceDefinition,
  AdaptiveSurfaceKeyContext,
  AdaptiveSurfaceEventMessage,
  AdaptiveSurfaceInputContext,
  AdaptiveSurfaceRenderContext,
} from "./types.js";
import { AdaptiveSurfacePanel } from "./shared.js";

export function structuredAnswersTemplate(payload: unknown): string {
  const questions = structuredQuestions(payload);
  if (questions.length === 0) return "[]";

  const template = questions.map((q) => ({
    question_id: q.id,
    value:
      q.kind === "multi_select"
        ? (q.defaultValues ??
          q.options?.slice(0, 1).map((option) => option.value) ??
          [])
        : q.kind === "single_select"
          ? (q.defaultValue ?? q.options?.[0]?.value ?? "")
          : (q.defaultValue ?? (q.required ? "<required-value>" : "")),
  }));

  return JSON.stringify(template);
}

export function structuredQuestionPositionLabel(
  index: number,
  questions: StructuredQuestion[],
): string {
  return `Question ${index + 1}/${questions.length}`;
}

export function structuredQuestionControls(
  question: StructuredQuestion | null,
): string {
  if (!question) {
    return "Controls: type / for manual command mode";
  }

  if (question.kind === "text") {
    return "Controls: Enter save/submit | Ctrl+N next | Ctrl+P previous | type / for commands";
  }

  if (question.kind === "single_select") {
    return "Controls: ↑/↓ or 1-9 choose | Enter confirm | Ctrl+N/P move | type / for commands";
  }

  return "Controls: ↑/↓ move | Space toggle | Enter confirm | Ctrl+N/P move | type / for commands";
}

function buildStructuredInputAnnouncement(
  pendingYield: AdaptiveSurfaceRenderContext["pendingYield"],
) {
  const messages: AdaptiveSurfaceEventMessage[] = [
    {
      type: "system_warning",
      message: `Input required (${pendingYield.requestId}).`,
    },
  ];
  const title = structuredTitle(pendingYield.payload);
  const prompt = structuredPrompt(pendingYield.payload);
  const questions = structuredQuestions(pendingYield.payload);

  if (title) {
    messages.push({ type: "system_message", message: `Title: ${title}` });
  }
  if (prompt) {
    messages.push({ type: "system_message", message: prompt });
  }
  messages.push({
    type: "system_message",
    message: `Questions: ${questions.length}`,
  });
  messages.push({
    type: "system_message",
    message:
      "Use the adaptive form in the Action panel, or /answers '<json-array>' for manual fallback.",
  });
  return messages;
}

function renderStructuredInputSurface({
  pendingYield,
  structuredInput,
}: AdaptiveSurfaceRenderContext) {
  const title = structuredTitle(pendingYield.payload);
  const prompt = structuredPrompt(pendingYield.payload);
  const questions = structuredInput?.questions ?? [];
  const activeQuestion = structuredInput?.activeQuestion ?? null;
  const formState = structuredInput?.formState ?? null;
  const formError =
    formState && questions.length > 0
      ? structuredFormValidationError(formState, questions)
      : null;

  return (
    <AdaptiveSurfacePanel
      title="Action required: structured input"
      requestId={pendingYield.requestId}
    >
      {title ? <Text>{title}</Text> : null}
      {prompt ? <Text color="gray">{prompt}</Text> : null}
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
          {activeQuestion.kind === "text" && activeQuestion.placeholder ? (
            <Text color="gray">placeholder: {activeQuestion.placeholder}</Text>
          ) : null}
          {activeQuestion.kind === "multi_select" ? (
            <Text color="gray">
              constraint: min=
              {activeQuestion.minSelections ??
                (activeQuestion.required ? 1 : 0)}
              , max={activeQuestion.maxSelections ?? "any"}
            </Text>
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
          <Text color="gray">
            {`Manual fallback: /answers '${structuredAnswersTemplate(pendingYield.payload)}'`}
          </Text>
        </>
      ) : (
        <Text color="gray">
          Loading structured input form... /answers '
          {structuredAnswersTemplate(pendingYield.payload)}'
        </Text>
      )}
    </AdaptiveSurfacePanel>
  );
}

function structuredInputFooterHint({
  structuredInput,
}: AdaptiveSurfaceRenderContext) {
  return structuredQuestionControls(structuredInput?.activeQuestion ?? null);
}

function structuredInputLabel({
  structuredInput,
  pendingYield,
}: AdaptiveSurfaceInputContext) {
  return pendingYield.kind === "structured_input"
    ? "Answer / Command"
    : "Action";
}

function structuredInputPlaceholder({
  structuredInput,
}: AdaptiveSurfaceInputContext) {
  const activeQuestion = structuredInput?.activeQuestion ?? null;
  return activeQuestion?.kind === "text"
    ? `Answer: ${activeQuestion.label} (or /answers fallback)`
    : "Use adaptive controls above, or type /answers <json-array>";
}

function structuredInputFocus({
  structuredInput,
  inputValue,
}: AdaptiveSurfaceInputContext) {
  return (
    !structuredInput?.activeQuestion ||
    structuredInput.activeQuestion.kind === "text" ||
    inputValue.startsWith("/")
  );
}

function handleStructuredInputKey({
  input,
  key,
  inputValue,
  setInputValue,
  addSystemEvent,
  structuredInput,
  structuredInputControls,
}: AdaptiveSurfaceKeyContext) {
  if (
    !structuredInput?.formState ||
    !structuredInput.activeQuestion ||
    !structuredInputControls?.setFormState
  ) {
    return false;
  }

  const { formState, questions, activeQuestion } = structuredInput;
  const { setFormState } = structuredInputControls;

  if (inputValue.startsWith("/")) {
    return false;
  }

  if (activeQuestion.kind !== "text" && input === "/") {
    setInputValue("/");
    return true;
  }

  if (key.ctrl && input === "n") {
    setFormState((previous) =>
      previous ? moveStructuredQuestion(previous, questions, 1) : previous,
    );
    return true;
  }

  if (key.ctrl && input === "p") {
    setFormState((previous) =>
      previous ? moveStructuredQuestion(previous, questions, -1) : previous,
    );
    return true;
  }

  if (key.ctrl && input === "s") {
    structuredInputControls.submitStructuredForm();
    return true;
  }

  if (activeQuestion.kind === "text") {
    return false;
  }

  if (key.upArrow || input === "k") {
    setFormState((previous) =>
      previous
        ? activeQuestion.kind === "single_select"
          ? moveStructuredSingleSelection(previous, activeQuestion, -1)
          : moveStructuredOptionCursor(previous, activeQuestion, -1)
        : previous,
    );
    return true;
  }

  if (key.downArrow || input === "j") {
    setFormState((previous) =>
      previous
        ? activeQuestion.kind === "single_select"
          ? moveStructuredSingleSelection(previous, activeQuestion, 1)
          : moveStructuredOptionCursor(previous, activeQuestion, 1)
        : previous,
    );
    return true;
  }

  if (input >= "1" && input <= "9") {
    const index = Number(input) - 1;
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

  if (activeQuestion.kind === "multi_select" && input === " ") {
    const cursor = getStructuredOptionCursor(formState, activeQuestion);
    const nextState = toggleStructuredMultiOption(
      formState,
      activeQuestion,
      cursor,
    );
    if (nextState === formState) {
      addSystemEvent(
        "system_warning",
        `${activeQuestion.label}: at most ${activeQuestion.maxSelections} selections allowed.`,
      );
      return true;
    }
    setFormState(nextState);
    return true;
  }

  if (key.return) {
    structuredInputControls.confirmActiveQuestion();
    return true;
  }

  return false;
}

export const structuredInputSurface: AdaptiveSurfaceDefinition = {
  kind: "structured_input",
  buildAnnouncement: buildStructuredInputAnnouncement,
  render: renderStructuredInputSurface,
  footerHint: structuredInputFooterHint,
  inputLabel: structuredInputLabel,
  inputPlaceholder: structuredInputPlaceholder,
  inputFocus: structuredInputFocus,
  handleInputKey: handleStructuredInputKey,
};
