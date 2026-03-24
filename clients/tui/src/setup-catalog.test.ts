import { describe, expect, test } from "bun:test";
import {
  ADVANCED_PROVIDER_CATALOG,
  applySetupDefaults,
  buildConfigContent,
  buildHostConfigContent,
  configForSetupSelection,
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
    expect(rendered).toContain('package = "builtin:alan-memory"');
    expect(rendered).not.toContain('bind_address = "127.0.0.1:8090"');
  });

  test("service preset keeps its model default when the model field is left blank", () => {
    const option = requireServicePreset("kimi_coding");
    const rendered = buildConfigContent(option, {
      ...applySetupDefaults(DEFAULT_CONFIG, option),
      openai_chat_completions_compatible_model: "",
    });

    expect(rendered).toContain(
      'openai_chat_completions_compatible_model = "kimi-k2-0905-preview"',
    );
  });

  test("switching to a different service preset resets shared credentials", () => {
    const openrouter = requireServicePreset("openrouter");
    const deepseek = requireServicePreset("deepseek");
    const switched = configForSetupSelection(
      {
        ...applySetupDefaults(DEFAULT_CONFIG, openrouter),
        openai_chat_completions_compatible_api_key: "sk-openrouter",
      },
      openrouter,
      deepseek,
    );

    expect(switched.openai_chat_completions_compatible_api_key).toBe("");
    expect(switched.openai_chat_completions_compatible_base_url).toBe(
      "https://api.deepseek.com/v1",
    );
    expect(switched.openai_chat_completions_compatible_model).toBe(
      "deepseek-chat",
    );
  });

  test("re-entering the same preset preserves its in-progress field values", () => {
    const kimi = requireServicePreset("kimi_coding");
    const preserved = configForSetupSelection(
      {
        ...applySetupDefaults(DEFAULT_CONFIG, kimi),
        openai_chat_completions_compatible_api_key: "sk-kimi",
        openai_chat_completions_compatible_model: "kimi-custom",
      },
      kimi,
      kimi,
    );

    expect(preserved.openai_chat_completions_compatible_api_key).toBe(
      "sk-kimi",
    );
    expect(preserved.openai_chat_completions_compatible_model).toBe(
      "kimi-custom",
    );
    expect(preserved.openai_chat_completions_compatible_base_url).toBe(
      "https://api.moonshot.cn/v1",
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

  test("host config content uses canonical split host file shape", () => {
    const rendered = buildHostConfigContent();

    expect(rendered).toContain("# Alan Host Configuration");
    expect(rendered).toContain('bind_address = "127.0.0.1:8090"');
    expect(rendered).toContain('daemon_url = "http://127.0.0.1:8090"');
  });
});
