import type { StructuredQuestion } from "./yield";

export type StructuredAnswerValue = string | string[];

export interface StructuredFormState {
  requestId: string;
  questionSignature: string;
  activeQuestionIndex: number;
  answers: Record<string, StructuredAnswerValue>;
  optionCursorByQuestionId: Record<string, number>;
}

function allowedOptionValues(question: StructuredQuestion): string[] {
  return (question.options ?? []).map((option) => option.value);
}

function formatUnknownOptionError(
  unknownValues: string[],
  allowedValues: string[],
): string {
  const subject =
    unknownValues.length === 1
      ? `Unknown option: ${unknownValues[0]}.`
      : `Unknown options: ${unknownValues.join(", ")}.`;
  return `${subject} Use one of: ${allowedValues.join(", ")}.`;
}

function optionIndexForValue(
  question: StructuredQuestion,
  value: string,
): number {
  const options = question.options ?? [];
  if (options.length === 0) return 0;
  const index = options.findIndex((option) => option.value === value);
  return index >= 0 ? index : 0;
}

function optionLabelForValue(
  question: StructuredQuestion,
  value: string,
): string {
  return (
    question.options?.find((option) => option.value === value)?.label ?? value
  );
}

function wrapIndex(index: number, count: number): number {
  if (count <= 0) return 0;
  return ((index % count) + count) % count;
}

export function structuredQuestionSignature(
  questions: StructuredQuestion[],
): string {
  return JSON.stringify(questions);
}

export function createStructuredFormState(
  requestId: string,
  questions: StructuredQuestion[],
): StructuredFormState {
  const answers: Record<string, StructuredAnswerValue> = {};
  const optionCursorByQuestionId: Record<string, number> = {};

  for (const question of questions) {
    if (question.kind === "multi_select") {
      const defaults = question.defaultValues ?? [];
      answers[question.id] = [...defaults];
      optionCursorByQuestionId[question.id] = optionIndexForValue(
        question,
        defaults[0] ?? question.options?.[0]?.value ?? "",
      );
      continue;
    }

    const defaultValue = question.defaultValue ?? "";
    answers[question.id] = defaultValue;
    optionCursorByQuestionId[question.id] = optionIndexForValue(
      question,
      defaultValue || question.options?.[0]?.value || "",
    );
  }

  return {
    requestId,
    questionSignature: structuredQuestionSignature(questions),
    activeQuestionIndex: 0,
    answers,
    optionCursorByQuestionId,
  };
}

export function shouldReuseStructuredFormState(
  state: StructuredFormState,
  requestId: string,
  questions: StructuredQuestion[],
): boolean {
  return (
    state.requestId === requestId &&
    state.questionSignature === structuredQuestionSignature(questions)
  );
}

export function currentStructuredQuestion(
  state: StructuredFormState,
  questions: StructuredQuestion[],
): StructuredQuestion | null {
  if (questions.length === 0) return null;
  return (
    questions[wrapIndex(state.activeQuestionIndex, questions.length)] ?? null
  );
}

export function getStructuredAnswer(
  state: StructuredFormState,
  question: StructuredQuestion,
): StructuredAnswerValue {
  const answer = state.answers[question.id];
  if (answer !== undefined) {
    return answer;
  }
  return question.kind === "multi_select" ? [] : "";
}

export function getStructuredOptionCursor(
  state: StructuredFormState,
  question: StructuredQuestion,
): number {
  return wrapIndex(
    state.optionCursorByQuestionId[question.id] ?? 0,
    question.options?.length ?? 0,
  );
}

export function setStructuredTextAnswer(
  state: StructuredFormState,
  questionId: string,
  value: string,
): StructuredFormState {
  return {
    ...state,
    answers: {
      ...state.answers,
      [questionId]: value,
    },
  };
}

export function moveStructuredQuestion(
  state: StructuredFormState,
  questions: StructuredQuestion[],
  delta: number,
): StructuredFormState {
  if (questions.length === 0) return state;
  return {
    ...state,
    activeQuestionIndex: wrapIndex(
      state.activeQuestionIndex + delta,
      questions.length,
    ),
  };
}

export function moveStructuredOptionCursor(
  state: StructuredFormState,
  question: StructuredQuestion,
  delta: number,
): StructuredFormState {
  const optionCount = question.options?.length ?? 0;
  if (optionCount === 0) return state;

  return {
    ...state,
    optionCursorByQuestionId: {
      ...state.optionCursorByQuestionId,
      [question.id]: wrapIndex(
        getStructuredOptionCursor(state, question) + delta,
        optionCount,
      ),
    },
  };
}

export function moveStructuredSingleSelection(
  state: StructuredFormState,
  question: StructuredQuestion,
  delta: number,
): StructuredFormState {
  const optionCount = question.options?.length ?? 0;
  if (optionCount === 0) return state;

  const nextIndex = wrapIndex(
    getStructuredOptionCursor(state, question) + delta,
    optionCount,
  );
  return selectStructuredSingleOption(state, question, nextIndex);
}

export function selectStructuredSingleOption(
  state: StructuredFormState,
  question: StructuredQuestion,
  index: number,
): StructuredFormState {
  const option = question.options?.[index];
  if (!option) return state;

  return {
    ...state,
    answers: {
      ...state.answers,
      [question.id]: option.value,
    },
    optionCursorByQuestionId: {
      ...state.optionCursorByQuestionId,
      [question.id]: index,
    },
  };
}

export function toggleStructuredMultiOption(
  state: StructuredFormState,
  question: StructuredQuestion,
  index: number,
): StructuredFormState {
  const option = question.options?.[index];
  if (!option) return state;

  const answer = getStructuredAnswer(state, question);
  const selected = Array.isArray(answer) ? answer : [];
  const isSelected = selected.includes(option.value);
  const nextSelected = isSelected
    ? selected.filter((value) => value !== option.value)
    : [...selected, option.value];
  const maxSelections = question.maxSelections;

  if (
    !isSelected &&
    maxSelections !== undefined &&
    nextSelected.length > maxSelections
  ) {
    return state;
  }

  return {
    ...state,
    answers: {
      ...state.answers,
      [question.id]: nextSelected,
    },
    optionCursorByQuestionId: {
      ...state.optionCursorByQuestionId,
      [question.id]: index,
    },
  };
}

export function questionValidationError(
  state: StructuredFormState,
  question: StructuredQuestion,
): string | null {
  const answer = getStructuredAnswer(state, question);
  const allowedValues = allowedOptionValues(question);

  if (question.kind === "multi_select") {
    const selected = Array.isArray(answer) ? answer : [];
    const unknownValues = selected.filter(
      (value) =>
        allowedValues.length > 0 && !allowedValues.includes(value),
    );
    if (unknownValues.length > 0) {
      return formatUnknownOptionError(unknownValues, allowedValues);
    }
    const minSelections = question.minSelections ?? (question.required ? 1 : 0);
    if (selected.length < minSelections) {
      return minSelections === 1
        ? "Select at least one option."
        : `Select at least ${minSelections} options.`;
    }
    if (
      question.maxSelections !== undefined &&
      selected.length > question.maxSelections
    ) {
      return `Select at most ${question.maxSelections} options.`;
    }
    return null;
  }

  const value = typeof answer === "string" ? answer.trim() : "";
  if (question.required && value.length === 0) {
    return question.kind === "text" ? "Answer required." : "Select one option.";
  }
  if (
    question.kind === "single_select" &&
    value.length > 0 &&
    allowedValues.length > 0 &&
    !allowedValues.includes(value)
  ) {
    return formatUnknownOptionError([value], allowedValues);
  }
  return null;
}

export function structuredFormValidationError(
  state: StructuredFormState,
  questions: StructuredQuestion[],
): string | null {
  for (const question of questions) {
    const error = questionValidationError(state, question);
    if (error) {
      return `${question.label}: ${error}`;
    }
  }
  return null;
}

export function buildStructuredResumePayload(
  state: StructuredFormState,
  questions: StructuredQuestion[],
): { answers: Array<{ question_id: string; value: StructuredAnswerValue }> } {
  const answers: Array<{ question_id: string; value: StructuredAnswerValue }> =
    [];

  for (const question of questions) {
    const value = getStructuredAnswer(state, question);
    if (question.kind === "multi_select") {
      const selected = Array.isArray(value) ? value : [];
      if (selected.length > 0) {
        answers.push({ question_id: question.id, value: selected });
      }
      continue;
    }

    const textValue = typeof value === "string" ? value.trim() : "";
    if (textValue.length > 0) {
      answers.push({ question_id: question.id, value: textValue });
    }
  }

  return { answers };
}

export function questionAnswerPreview(
  state: StructuredFormState,
  question: StructuredQuestion,
): string {
  const answer = getStructuredAnswer(state, question);

  if (question.kind === "multi_select") {
    const selected = Array.isArray(answer) ? answer : [];
    if (selected.length === 0) return "No selection";
    return selected
      .map((value) => optionLabelForValue(question, value))
      .join(", ");
  }

  const value = typeof answer === "string" ? answer.trim() : "";
  if (!value) {
    return question.kind === "text"
      ? question.placeholder || "Not answered"
      : "No selection";
  }

  return question.kind === "single_select"
    ? optionLabelForValue(question, value)
    : value;
}
