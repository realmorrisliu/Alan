import { getStructuredAnswer, type StructuredFormState } from "./structured-input.js";
import type { StructuredInputOption, StructuredQuestion } from "./yield.js";
import { asStringArray } from "./yield.js";

type SchemaPrimitiveType = "string" | "boolean" | "number" | "integer";

interface SchemaDrivenField {
  key: string;
  type: SchemaPrimitiveType;
  question: StructuredQuestion;
}

export interface SchemaDrivenYieldForm {
  title: string;
  prompt?: string;
  questions: StructuredQuestion[];
  fields: SchemaDrivenField[];
}

function asRecord(value: unknown): Record<string, unknown> | null {
  return value && typeof value === "object" && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : null;
}

function asString(value: unknown): string | null {
  return typeof value === "string" ? value : null;
}

function humanizeKey(value: string): string {
  return value
    .split(/[_\s]+/)
    .filter(Boolean)
    .map((part) => part[0].toUpperCase() + part.slice(1))
    .join(" ");
}

function parseStringEnumOptions(value: unknown): StructuredInputOption[] | null {
  if (!Array.isArray(value)) {
    return null;
  }

  const options = value
    .filter((item): item is string => typeof item === "string")
    .map((item) => ({ value: item, label: humanizeKey(item) }));
  return options.length === value.length && options.length > 0 ? options : null;
}

function parseSchemaField(
  key: string,
  rawSchema: unknown,
  requiredKeys: string[],
): SchemaDrivenField | null {
  const schema = asRecord(rawSchema);
  if (!schema) return null;

  const enumOptions = parseStringEnumOptions(schema.enum);
  const explicitType = asString(schema.type);
  const type =
    explicitType === "string" ||
    explicitType === "boolean" ||
    explicitType === "number" ||
    explicitType === "integer"
      ? explicitType
      : enumOptions
        ? "string"
        : null;
  if (!type) {
    return null;
  }

  const label = asString(schema.title) ?? humanizeKey(key);
  const description = asString(schema.description) ?? undefined;
  const defaultValue = schema.default;
  const required = requiredKeys.includes(key);

  const question: StructuredQuestion =
    type === "boolean"
      ? {
          id: key,
          label,
          prompt: description ?? label,
          kind: "single_select",
          required,
          defaultValue:
            typeof defaultValue === "boolean" ? String(defaultValue) : undefined,
          options: [
            { value: "true", label: "Yes" },
            { value: "false", label: "No" },
          ],
        }
      : enumOptions
        ? {
            id: key,
            label,
            prompt: description ?? label,
            kind: "single_select",
            required,
            defaultValue: asString(defaultValue) ?? undefined,
            options: enumOptions,
          }
        : {
            id: key,
            label,
            prompt: description ?? label,
            kind: "text",
            required,
            placeholder:
              type === "number" || type === "integer" ? "0" : undefined,
            defaultValue:
              typeof defaultValue === "string" ||
              typeof defaultValue === "number"
                ? String(defaultValue)
                : undefined,
            helpText:
              type === "number" || type === "integer"
                ? "Enter a numeric value."
                : undefined,
          };

  return {
    key,
    type,
    question,
  };
}

export function parseSchemaDrivenYieldForm(
  payload: unknown,
): SchemaDrivenYieldForm | null {
  const data = asRecord(payload);
  if (!data) {
    return null;
  }

  const schema = asRecord(data.resume_schema ?? data.schema);
  if (!schema || asString(schema.type) !== "object") {
    return null;
  }

  const properties = asRecord(schema.properties);
  if (!properties) {
    return null;
  }

  const requiredKeys = asStringArray(schema.required);
  const fields = Object.entries(properties)
    .map(([key, value]) => parseSchemaField(key, value, requiredKeys))
    .filter((field): field is SchemaDrivenField => field !== null);

  if (fields.length === 0 || fields.length !== Object.keys(properties).length) {
    return null;
  }

  return {
    title:
      asString(data.title) ??
      asString(schema.title) ??
      "Provide structured input",
    prompt: asString(data.prompt) ?? asString(schema.description) ?? undefined,
    questions: fields.map((field) => field.question),
    fields,
  };
}

export function buildSchemaDrivenYieldPayload(
  formState: StructuredFormState,
  form: SchemaDrivenYieldForm,
): { payload: Record<string, unknown> } | { error: string } {
  const payload: Record<string, unknown> = {};

  for (const field of form.fields) {
    const answer = getStructuredAnswer(formState, field.question);

    if (field.type === "boolean") {
      const value = typeof answer === "string" ? answer : "";
      if (!value && !field.question.required) {
        continue;
      }
      payload[field.key] = value === "true";
      continue;
    }

    const value = typeof answer === "string" ? answer.trim() : "";
    if (!value && !field.question.required) {
      continue;
    }

    if (field.type === "integer" || field.type === "number") {
      const parsed = Number(value);
      if (!Number.isFinite(parsed)) {
        return {
          error: `${field.question.label}: enter a valid number.`,
        };
      }
      if (field.type === "integer" && !Number.isInteger(parsed)) {
        return {
          error: `${field.question.label}: enter a whole number.`,
        };
      }
      payload[field.key] = parsed;
      continue;
    }

    payload[field.key] = value;
  }

  return { payload };
}
