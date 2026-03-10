import {
  getStructuredAnswer,
  type StructuredFormState,
} from "./structured-input.js";
import type { StructuredQuestion } from "./yield.js";
import {
  parseCustomYieldPayload,
  parseDynamicToolYieldPayload,
  usesMultiSelectKind,
} from "./yield.js";

export interface SchemaDrivenYieldForm {
  title: string;
  prompt?: string;
  questions: StructuredQuestion[];
}

export function parseSchemaDrivenYieldForm(
  payload: unknown,
): SchemaDrivenYieldForm | null {
  const dynamic = parseDynamicToolYieldPayload(payload);
  if (dynamic?.form?.fields.length) {
    return {
      title: dynamic.title,
      prompt: dynamic.prompt,
      questions: dynamic.form.fields,
    };
  }

  const custom = parseCustomYieldPayload(payload);
  if (custom?.form?.fields.length) {
    return {
      title: custom.title ?? "Provide structured input",
      prompt: custom.prompt,
      questions: custom.form.fields,
    };
  }

  return null;
}

export function buildSchemaDrivenYieldPayload(
  formState: StructuredFormState,
  form: SchemaDrivenYieldForm,
): { payload: Record<string, unknown> } | { error: string } {
  const payload: Record<string, unknown> = {};

  for (const question of form.questions) {
    const answer = getStructuredAnswer(formState, question);

    if (usesMultiSelectKind(question.kind)) {
      const selected = Array.isArray(answer) ? answer : [];
      if (selected.length > 0) {
        payload[question.id] = selected;
      }
      continue;
    }

    const value = typeof answer === "string" ? answer.trim() : "";
    if (!value && !question.required) {
      continue;
    }

    if (question.kind === "boolean") {
      payload[question.id] = value === "true";
      continue;
    }

    if (question.kind === "integer" || question.kind === "number") {
      const parsed = Number(value);
      if (!Number.isFinite(parsed)) {
        return {
          error: `${question.label}: enter a valid number.`,
        };
      }
      if (question.kind === "integer" && !Number.isInteger(parsed)) {
        return {
          error: `${question.label}: enter a whole number.`,
        };
      }
      payload[question.id] = parsed;
      continue;
    }

    payload[question.id] = value;
  }

  return { payload };
}
