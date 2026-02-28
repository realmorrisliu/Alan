export interface StructuredQuestion {
  id: string;
  label: string;
  prompt: string;
  required?: boolean;
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

export function asStringArray(value: unknown): string[] {
  if (!Array.isArray(value)) return [];
  return value.filter((item): item is string => typeof item === "string");
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

      if (!id || !label || !prompt) {
        return null;
      }

      return { id, label, prompt, required } as StructuredQuestion;
    })
    .filter((item): item is StructuredQuestion => item !== null);
}
