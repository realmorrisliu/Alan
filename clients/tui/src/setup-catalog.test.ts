import { describe, expect, test } from "bun:test";
import {
  ADVANCED_PROVIDER_CATALOG,
  applySetupDefaults,
  buildConfigContent,
  DEFAULT_CONFIG,
  isConfigurableSetupOption,
  SERVICE_CATALOG,
} from "./setup-catalog.js";

function requireServicePreset(key: string) {
  const option = SERVICE_CATALOG.find((entry) => entry.key === key);
  expect(option).toBeDefined();
  expect(option && isConfigurableSetupOption(option)).toBe(true);
  if (!option || !isConfigurableSetupOption(option)) {
    throw new Error(`Missing configurable service preset: ${key}`);
  }
  return option;
}

function requireAdvancedPreset(key: string) {
  const option = ADVANCED_PROVIDER_CATALOG.find((entry) => entry.key === key);
  expect(option).toBeDefined();
  if (!option) {
    throw new Error(`Missing advanced preset: ${key}`);
  }
  return option;
}

describe("service-first setup catalog", () => {
  test("includes popular services and an explicit advanced handoff", () => {
    const keys = SERVICE_CATALOG.map((entry) => entry.key);
    expect(keys).toContain("kimi_coding");
    expect(keys).toContain("deepseek");
    expect(keys).toContain("openrouter");
    expect(keys).toContain("advanced_custom");
  });

  test("OpenAI API Platform preset maps to OpenAI Responses without exposing base URL", () => {
    const option = requireServicePreset("openai_api_platform");
    expect(option.provider).toBe("openai_responses");
    expect(option.fields.map((field) => field.key)).toEqual([
      "openai_responses_api_key",
      "openai_responses_model",
    ]);
  });

  test("OpenRouter preset writes canonical compatible config with OpenRouter defaults", () => {
    const option = requireServicePreset("openrouter");
    const config = applySetupDefaults(DEFAULT_CONFIG, option);
    const rendered = buildConfigContent(option, config);

    expect(option.provider).toBe("openai_chat_completions_compatible");
    expect(option.fields.some((field) => field.key.includes("base_url"))).toBe(
      false,
    );
    expect(rendered).toContain(
      'llm_provider = "openai_chat_completions_compatible"',
    );
    expect(rendered).toContain(
      'openai_chat_completions_compatible_base_url = "https://openrouter.ai/api/v1"',
    );
    expect(rendered).toContain(
      'openai_chat_completions_compatible_model = "openai/gpt-5.2"',
    );
  });

  test("advanced compatible setup exposes base URL for manual endpoint control", () => {
    const option = requireAdvancedPreset(
      "advanced_openai_chat_completions_compatible",
    );
    const rendered = buildConfigContent(
      option,
      applySetupDefaults(DEFAULT_CONFIG, option),
    );

    expect(option.fields.map((field) => field.key)).toEqual([
      "openai_chat_completions_compatible_base_url",
      "openai_chat_completions_compatible_api_key",
      "openai_chat_completions_compatible_model",
    ]);
    expect(rendered).toContain(
      "# Custom OpenAI Chat Completions API-compatible Configuration",
    );
  });
});
