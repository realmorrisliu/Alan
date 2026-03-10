import type {
  AdaptiveForm,
  AdaptivePresentationHint,
  ConfirmationYieldPayload as ProtocolConfirmationYieldPayload,
  CustomYieldPayload as ProtocolCustomYieldPayload,
  DynamicToolYieldPayload as ProtocolDynamicToolYieldPayload,
  ProtocolStructuredInputKind,
  ProtocolStructuredInputOption,
  ProtocolStructuredInputQuestion,
  StructuredInputYieldPayload as ProtocolStructuredInputYieldPayload,
  YieldKind,
} from "./types";

export type StructuredInputOption = ProtocolStructuredInputOption;
export type StructuredInputKind = ProtocolStructuredInputKind;

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
  presentationHints?: AdaptivePresentationHint[];
}

export interface ConfirmationYieldPayload extends Omit<
  ProtocolConfirmationYieldPayload,
  "details" | "options"
> {
  details?: Record<string, unknown> | null;
  options?: string[];
}

export interface StructuredInputYieldPayload {
  title: string;
  prompt?: string;
  questions: StructuredQuestion[];
}

export interface DynamicToolYieldPayload extends Omit<
  ProtocolDynamicToolYieldPayload,
  "form"
> {
  form?: AdaptiveFormState;
}

export interface CustomYieldPayload extends Omit<
  ProtocolCustomYieldPayload,
  "form"
> {
  form?: AdaptiveFormState;
}

export interface AdaptiveFormState {
  fields: StructuredQuestion[];
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

function parsePresentationHints(
  value: unknown,
): AdaptivePresentationHint[] | undefined {
  const hints = asStringArray(value).filter(
    (hint): hint is AdaptivePresentationHint =>
      hint === "radio" ||
      hint === "toggle" ||
      hint === "searchable" ||
      hint === "multiline" ||
      hint === "compact" ||
      hint === "dangerous",
  );
  return hints.length > 0 ? hints : undefined;
}

function parseStructuredInputKind(
  value: unknown,
  hasOptions: boolean,
): StructuredInputKind | null {
  if (
    value === "text" ||
    value === "boolean" ||
    value === "number" ||
    value === "integer" ||
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

function defaultBooleanOptions(): StructuredInputOption[] {
  return [
    { value: "true", label: "Yes" },
    { value: "false", label: "No" },
  ];
}

function parseStructuredOptions(
  value: unknown,
  kind: StructuredInputKind | null,
): StructuredInputOption[] {
  if (!Array.isArray(value)) {
    return kind === "boolean" ? defaultBooleanOptions() : [];
  }

  const options = value
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

  if (kind === "boolean" && options.length === 0) {
    return defaultBooleanOptions();
  }
  return options;
}

export function usesTextEntryKind(kind: StructuredInputKind): boolean {
  return kind === "text" || kind === "number" || kind === "integer";
}

export function usesSingleSelectKind(kind: StructuredInputKind): boolean {
  return kind === "single_select" || kind === "boolean";
}

export function usesMultiSelectKind(kind: StructuredInputKind): boolean {
  return kind === "multi_select";
}

function parseStructuredQuestion(value: unknown): StructuredQuestion | null {
  const question = asRecord(value);
  if (!question) return null;

  const id = asString(question.id);
  const label = asString(question.label);
  const prompt = asString(question.prompt);
  const kindCandidate = parseStructuredInputKind(
    question.kind,
    Array.isArray(question.options),
  );
  const options = parseStructuredOptions(question.options, kindCandidate);
  const kind = parseStructuredInputKind(question.kind, options.length > 0);
  const required =
    typeof question.required === "boolean" ? question.required : undefined;
  const placeholder = asString(question.placeholder) || undefined;
  const helpText = asString(question.help_text) || undefined;
  const defaultValue = asString(question.default) || undefined;
  const defaultValues = asStringArray(question.defaults);
  const minSelections = asNumber(question.min_selected) ?? undefined;
  const maxSelections = asNumber(question.max_selected) ?? undefined;
  const presentationHints = parsePresentationHints(question.presentation_hints);

  if (!id || !label || !prompt || !kind) {
    return null;
  }

  if (
    (usesSingleSelectKind(kind) || usesMultiSelectKind(kind)) &&
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
    presentationHints,
  };
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

export function parseConfirmationPayload(
  payload: unknown,
): ConfirmationYieldPayload | null {
  const data = asRecord(payload);
  const summary = data ? asString(data.summary) : null;
  if (!data || !summary) {
    return null;
  }

  return {
    checkpoint_type: asString(data.checkpoint_type) ?? "confirmation",
    summary,
    details: asRecord(data.details),
    options: asStringArray(data.options),
    default_option: asString(data.default_option) ?? undefined,
    presentation_hints: parsePresentationHints(data.presentation_hints),
  };
}

export function parseStructuredInputPayload(
  payload: unknown,
): StructuredInputYieldPayload | null {
  const data = asRecord(payload);
  const title = data ? asString(data.title) : null;
  if (!data || !title || !Array.isArray(data.questions)) {
    return null;
  }

  return {
    title,
    prompt: asString(data.prompt) ?? undefined,
    questions: data.questions
      .map(parseStructuredQuestion)
      .filter((item): item is StructuredQuestion => item !== null),
  };
}

export function parseAdaptiveForm(payload: unknown): AdaptiveFormState | null {
  const data = asRecord(payload);
  if (!data || !Array.isArray(data.fields)) {
    return null;
  }

  const fields = data.fields
    .map(parseStructuredQuestion)
    .filter((item): item is StructuredQuestion => item !== null);
  if (fields.length === 0) {
    return null;
  }

  return { fields };
}

export function parseDynamicToolYieldPayload(
  payload: unknown,
): DynamicToolYieldPayload | null {
  const data = asRecord(payload);
  const toolName = data ? asString(data.tool_name) : null;
  const title = data ? asString(data.title) : null;
  if (!data || !toolName || !title) {
    return null;
  }

  return {
    tool_name: toolName,
    arguments: data.arguments,
    title,
    prompt: asString(data.prompt) ?? undefined,
    form: parseAdaptiveForm(data.form) ?? undefined,
  };
}

export function parseCustomYieldPayload(
  payload: unknown,
): CustomYieldPayload | null {
  const data = asRecord(payload);
  if (!data) {
    return null;
  }

  return {
    title: asString(data.title) ?? undefined,
    prompt: asString(data.prompt) ?? undefined,
    details: asRecord(data.details) ?? undefined,
    form: parseAdaptiveForm(data.form) ?? undefined,
  };
}

export function confirmationSummary(payload: unknown): string | null {
  return parseConfirmationPayload(payload)?.summary ?? null;
}

export function confirmationOptions(payload: unknown): string[] {
  return parseConfirmationPayload(payload)?.options ?? [];
}

export function confirmationActionOptions(payload: unknown): string[] {
  const options = confirmationOptions(payload);
  return options.length > 0 ? options : ["approve", "modify", "reject"];
}

export function confirmationDetails(
  payload: unknown,
): Record<string, unknown> | null {
  return parseConfirmationPayload(payload)?.details ?? null;
}

export function structuredTitle(payload: unknown): string | null {
  return parseStructuredInputPayload(payload)?.title ?? null;
}

export function structuredPrompt(payload: unknown): string | null {
  return parseStructuredInputPayload(payload)?.prompt ?? null;
}

export function structuredQuestions(payload: unknown): StructuredQuestion[] {
  const data = asRecord(payload);
  if (!data || !Array.isArray(data.questions)) {
    return [];
  }

  return data.questions
    .map(parseStructuredQuestion)
    .filter((item): item is StructuredQuestion => item !== null);
}
