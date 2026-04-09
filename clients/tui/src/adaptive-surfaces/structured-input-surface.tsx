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
import type { StructuredFormState } from "../structured-input.js";
import type { StructuredQuestion } from "../yield.js";
import {
  questionHasPresentationHint,
  usesMultiSelectKind,
  usesSingleSelectKind,
  usesTextEntryKind,
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

export type StructuredQuestionFallbackMode =
  | "structured_input"
  | "schema_fallback";

function multilineFallbackHint(mode: StructuredQuestionFallbackMode): string {
  return mode === "schema_fallback"
    ? "/resume <json-object> for raw fallback"
    : "/answer or /answers for long text";
}

export function structuredAnswersTemplate(payload: unknown): string {
  const questions = structuredQuestions(payload);
  if (questions.length === 0) return "[]";

  const template = questions.map((q) => ({
    question_id: q.id,
    value: usesMultiSelectKind(q.kind)
      ? (q.defaultValues ??
        q.options?.slice(0, 1).map((option) => option.value) ??
        [])
      : usesSingleSelectKind(q.kind)
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
  mode: StructuredQuestionFallbackMode = "structured_input",
): string {
  if (!question) {
    return "Controls: type / for manual command mode";
  }

  if (usesTextEntryKind(question.kind)) {
    if (questionHasPresentationHint(question, "multiline")) {
      return `Controls: Enter save/submit | Ctrl+N next | Ctrl+P previous | ${multilineFallbackHint(
        mode,
      )}`;
    }
    return "Controls: Enter save/submit | Ctrl+N next | Ctrl+P previous | type / for commands";
  }

  if (usesSingleSelectKind(question.kind)) {
    if (questionHasPresentationHint(question, "toggle")) {
      return "Controls: ←/→ or 1-2 toggle | Enter confirm | Ctrl+N/P move | type / for commands";
    }
    return "Controls: ↑/↓ or ←/→ or 1-9 choose | Enter confirm | Ctrl+N/P move | type / for commands";
  }

  return "Controls: ↑/↓ move | Space toggle | Enter confirm | Ctrl+N/P move | type / for commands";
}

export function structuredQuestionHints(
  question: StructuredQuestion | null,
  mode: StructuredQuestionFallbackMode = "structured_input",
): Array<{ text: string; color: string }> {
  if (!question) {
    return [];
  }

  const hints: Array<{ text: string; color: string }> = [];

  if (question.kind === "integer") {
    hints.push({
      text: "Numeric input: whole numbers only.",
      color: "gray",
    });
  } else if (question.kind === "number") {
    hints.push({
      text: "Numeric input: decimals are allowed.",
      color: "gray",
    });
  }

  if (
    questionHasPresentationHint(question, "toggle") &&
    usesSingleSelectKind(question.kind)
  ) {
    hints.push({
      text: "Toggle hint: use ←/→ to switch the selected option quickly.",
      color: "gray",
    });
  }

  if (questionHasPresentationHint(question, "multiline")) {
    hints.push({
      text:
        mode === "schema_fallback"
          ? "Long text hint: use /resume <json-object> if you need a raw fallback payload."
          : "Long text hint: use /answer or /answers with escaped newlines for multi-line input.",
      color: "gray",
    });
  }

  if (questionHasPresentationHint(question, "dangerous")) {
    hints.push({
      text: "Dangerous input: review this value carefully before submitting.",
      color: "red",
    });
  }

  return hints;
}

export function structuredQuestionToggleSummary(
  question: StructuredQuestion | null,
  formState: StructuredFormState | null,
): string | null {
  if (
    !question ||
    !formState ||
    !usesSingleSelectKind(question.kind) ||
    !questionHasPresentationHint(question, "toggle") ||
    !question.options?.length
  ) {
    return null;
  }

  const answer = getStructuredAnswer(formState, question);
  return question.options
    .map((option) =>
      answer === option.value ? `[${option.label}]` : option.label,
    )
    .join(" / ");
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
          {structuredQuestionHints(activeQuestion).map((hint) => (
            <Text key={hint.text} color={hint.color}>
              {hint.text}
            </Text>
          ))}
          {usesTextEntryKind(activeQuestion.kind) &&
          activeQuestion.placeholder ? (
            <Text color="gray">placeholder: {activeQuestion.placeholder}</Text>
          ) : null}
          {usesMultiSelectKind(activeQuestion.kind) ? (
            <Text color="gray">
              constraint: min=
              {activeQuestion.minSelections ??
                (activeQuestion.required ? 1 : 0)}
              , max={activeQuestion.maxSelections ?? "any"}
            </Text>
          ) : null}
          {structuredQuestionToggleSummary(activeQuestion, formState) ? (
            <Text color="gray">
              Toggle:{" "}
              {structuredQuestionToggleSummary(activeQuestion, formState)}
            </Text>
          ) : null}
          {activeQuestion.options?.map((option, index) => {
            const answer = getStructuredAnswer(formState, activeQuestion);
            const isSelected = Array.isArray(answer)
              ? answer.includes(option.value)
              : answer === option.value;
            const isCursor =
              getStructuredOptionCursor(formState, activeQuestion) === index;
            const marker = usesMultiSelectKind(activeQuestion.kind)
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
  return activeQuestion && usesTextEntryKind(activeQuestion.kind)
    ? `Answer: ${activeQuestion.label} (or /answers fallback)`
    : "Use adaptive controls above, or type /answers <json-array>";
}

function structuredInputFocus({
  structuredInput,
  inputValue,
}: AdaptiveSurfaceInputContext) {
  return (
    !structuredInput?.activeQuestion ||
    usesTextEntryKind(structuredInput.activeQuestion.kind) ||
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

  if (!usesTextEntryKind(activeQuestion.kind) && input === "/") {
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

  if (usesTextEntryKind(activeQuestion.kind)) {
    return false;
  }

  if (
    usesSingleSelectKind(activeQuestion.kind) &&
    (key.leftArrow || input === "h")
  ) {
    setFormState((previous) =>
      previous
        ? moveStructuredSingleSelection(previous, activeQuestion, -1)
        : previous,
    );
    return true;
  }

  if (
    usesSingleSelectKind(activeQuestion.kind) &&
    (key.rightArrow || input === "l")
  ) {
    setFormState((previous) =>
      previous
        ? moveStructuredSingleSelection(previous, activeQuestion, 1)
        : previous,
    );
    return true;
  }

  if (key.upArrow || input === "k") {
    setFormState((previous) =>
      previous
        ? usesSingleSelectKind(activeQuestion.kind)
          ? moveStructuredSingleSelection(previous, activeQuestion, -1)
          : moveStructuredOptionCursor(previous, activeQuestion, -1)
        : previous,
    );
    return true;
  }

  if (key.downArrow || input === "j") {
    setFormState((previous) =>
      previous
        ? usesSingleSelectKind(activeQuestion.kind)
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
      return usesSingleSelectKind(activeQuestion.kind)
        ? selectStructuredSingleOption(previous, activeQuestion, index)
        : toggleStructuredMultiOption(previous, activeQuestion, index);
    });
    return true;
  }

  if (usesMultiSelectKind(activeQuestion.kind) && input === " ") {
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
