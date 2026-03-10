import type { YieldKind } from "./types";

export interface StructuredInputOption {
  value: string;
  label: string;
  description?: string;
}

export type StructuredInputKind = "text" | "single_select" | "multi_select";

export interface StructuredQuestion {
  id: string;
  label: string;
  prompt: string;
  kind: StructuredInputKind;
  required?: boolean;
  placeholder?: string;
  helpText?: string;
  defaultValue?: string;
  defaultValues?: string[];
  minSelections?: number;
  maxSelections?: number;
  options?: StructuredInputOption[];
}

function asRecord(value: unknown): Record<string, unknown> | null {
  if (value && typeof value === "object" && !Array.isArray(value)) {
    return value as Record<string, unknown>;
  }
  return null;
}

export function asString(value: unknown): string | null {
  return typeof value === "string" ? value : null;
}

function asNumber(value: unknown): number | null {
  return typeof value === "number" && Number.isFinite(value) ? value : null;
}

export function asStringArray(value: unknown): string[] {
  if (!Array.isArray(value)) return [];
  return value.filter((item): item is string => typeof item === "string");
}

function parseStructuredInputKind(
  value: unknown,
  hasOptions: boolean,
): StructuredInputKind | null {
  if (
    value === "text" ||
    value === "single_select" ||
    value === "multi_select"
  ) {
    return value;
  }

  if (value === undefined) {
    return hasOptions ? "single_select" : "text";
  }

  return null;
}

function parseStructuredOptions(value: unknown): StructuredInputOption[] {
  if (!Array.isArray(value)) return [];

  return value
    .map((item) => {
      const raw = asRecord(item);
      if (!raw) return null;

      const parsed: StructuredInputOption = {
        value: asString(raw.value) || "",
        label: asString(raw.label) || "",
      };

      const description = asString(raw.description);
      if (description) {
        parsed.description = description;
      }

      if (!parsed.value || !parsed.label) {
        return null;
      }

      return parsed;
    })
    .filter((item): item is StructuredInputOption => item !== null);
}

export function normalizeYieldKind(
  kind: YieldKind | undefined,
): "confirmation" | "structured_input" | "dynamic_tool" | "custom" | null {
  if (!kind) {
    return null;
  }

  if (typeof kind === "string") {
    if (kind === "confirmation") return "confirmation";
    if (kind === "structured_input") return "structured_input";
    if (kind === "dynamic_tool") return "dynamic_tool";
    return "custom";
  }

  if (typeof kind === "object" && kind !== null && "custom" in kind) {
    return "custom";
  }

  return null;
}

export function confirmationSummary(payload: unknown): string | null {
  const data = asRecord(payload);
  if (!data) return null;
  return asString(data.summary);
}

export function confirmationOptions(payload: unknown): string[] {
  const data = asRecord(payload);
  if (!data) return [];
  return asStringArray(data.options);
}

export function structuredTitle(payload: unknown): string | null {
  const data = asRecord(payload);
  if (!data) return null;
  return asString(data.title);
}

export function structuredPrompt(payload: unknown): string | null {
  const data = asRecord(payload);
  if (!data) return null;
  return asString(data.prompt);
}

export function structuredQuestions(payload: unknown): StructuredQuestion[] {
  const data = asRecord(payload);
  if (!data || !Array.isArray(data.questions)) return [];

  return data.questions
    .map((item) => {
      const question = asRecord(item);
      if (!question) return null;

      const id = asString(question.id);
      const label = asString(question.label);
      const prompt = asString(question.prompt);
      const required =
        typeof question.required === "boolean" ? question.required : undefined;
      const options = parseStructuredOptions(question.options);
      const kind = parseStructuredInputKind(question.kind, options.length > 0);
      const placeholder = asString(question.placeholder) || undefined;
      const helpText = asString(question.help_text) || undefined;
      const defaultValue = asString(question.default) || undefined;
      const defaultValues = asStringArray(question.defaults);
      const minSelections = asNumber(question.min_selected) ?? undefined;
      const maxSelections = asNumber(question.max_selected) ?? undefined;

      if (!id || !label || !prompt || !kind) {
        return null;
      }

      if (
        (kind === "single_select" || kind === "multi_select") &&
        options.length === 0
      ) {
        return null;
      }

      return {
        id,
        label,
        prompt,
        kind,
        required,
        placeholder,
        helpText,
        defaultValue,
        defaultValues: defaultValues.length > 0 ? defaultValues : undefined,
        minSelections,
        maxSelections,
        options: options.length > 0 ? options : undefined,
      } as StructuredQuestion;
    })
    .filter((item): item is StructuredQuestion => item !== null);
}
