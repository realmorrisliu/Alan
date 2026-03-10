/**
 * First-time setup wizard for Alan
 *
 * Runs when config.toml doesn't exist
 */

import React, { useState } from "react";
import { Box, Text, useInput } from "ink";
import TextInput from "ink-text-input";
import { chmodSync, mkdirSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";
import { homedir } from "node:os";

interface InitWizardProps {
  onComplete: () => void;
  configPath: string;
}

type Provider =
  | "google_gemini_generate_content"
  | "openai_responses"
  | "openai_chat_completions"
  | "openai_chat_completions_compatible"
  | "anthropic_messages";
type WizardStep = "welcome" | "provider" | "config" | "done";

interface ConfigValues {
  google_gemini_generate_content_location: string;
  google_gemini_generate_content_project_id: string;
  google_gemini_generate_content_model: string;
  openai_responses_base_url: string;
  openai_responses_api_key: string;
  openai_responses_model: string;
  openai_chat_completions_base_url: string;
  openai_chat_completions_api_key: string;
  openai_chat_completions_model: string;
  openai_chat_completions_compatible_base_url: string;
  openai_chat_completions_compatible_api_key: string;
  openai_chat_completions_compatible_model: string;
  anthropic_messages_base_url: string;
  anthropic_messages_api_key: string;
  anthropic_messages_model: string;
}

const DEFAULT_CONFIG: ConfigValues = {
  google_gemini_generate_content_location: "us-central1",
  google_gemini_generate_content_project_id: "",
  google_gemini_generate_content_model: "gemini-2.0-flash",
  openai_responses_base_url: "https://api.openai.com/v1",
  openai_responses_api_key: "",
  openai_responses_model: "gpt-5.4",
  openai_chat_completions_base_url: "https://api.openai.com/v1",
  openai_chat_completions_api_key: "",
  openai_chat_completions_model: "gpt-5.4",
  openai_chat_completions_compatible_base_url: "https://api.openai.com/v1",
  openai_chat_completions_compatible_api_key: "",
  openai_chat_completions_compatible_model: "qwen3.5-plus",
  anthropic_messages_base_url: "https://api.anthropic.com/v1",
  anthropic_messages_api_key: "",
  anthropic_messages_model: "claude-3-5-sonnet-latest",
};

function displayPath(path: string): string {
  const home = homedir();
  if (path === home) return "~";
  const homePrefix = `${home}/`;
  return path.startsWith(homePrefix)
    ? `~/${path.slice(homePrefix.length)}`
    : path;
}

export function InitWizard({ onComplete, configPath }: InitWizardProps) {
  const [step, setStep] = useState<WizardStep>("welcome");
  const [selectedProvider, setSelectedProvider] = useState<Provider>(
    "google_gemini_generate_content",
  );
  const [config, setConfig] = useState<ConfigValues>(DEFAULT_CONFIG);

  // Cursor for provider selection
  const [providerCursor, setProviderCursor] = useState(0);

  // For config input: which field we're currently editing
  const [configFieldIndex, setConfigFieldIndex] = useState(0);
  // Current input value for the active field
  const [inputValue, setInputValue] = useState("");

  const providers: { key: Provider; name: string; desc: string }[] = [
    {
      key: "google_gemini_generate_content",
      name: "Google Gemini GenerateContent API",
      desc: "Gemini via Google Cloud / Vertex AI",
    },
    {
      key: "openai_responses",
      name: "OpenAI Responses API",
      desc: "Official OpenAI Responses API",
    },
    {
      key: "openai_chat_completions",
      name: "OpenAI Chat Completions API",
      desc: "Official OpenAI Chat Completions API",
    },
    {
      key: "openai_chat_completions_compatible",
      name: "OpenAI Chat Completions API-compatible",
      desc: "Compatible providers that mirror OpenAI chat/completions",
    },
    {
      key: "anthropic_messages",
      name: "Anthropic Messages API",
      desc: "Anthropic-native Messages API",
    },
  ];

  const getConfigFields = (
    provider: Provider,
  ): {
    key: keyof ConfigValues;
    label: string;
    placeholder: string;
    hint?: string;
  }[] => {
    switch (provider) {
      case "google_gemini_generate_content":
        return [
          {
            key: "google_gemini_generate_content_location",
            label: "Location",
            placeholder: "us-central1",
            hint: "e.g., us-central1, asia-northeast1",
          },
          {
            key: "google_gemini_generate_content_project_id",
            label: "Project ID",
            placeholder: "your-gcp-project-id",
            hint: "Your Google Cloud project ID",
          },
          {
            key: "google_gemini_generate_content_model",
            label: "Model",
            placeholder: "gemini-2.0-flash",
            hint: "e.g., gemini-2.0-flash, gemini-2.5-pro",
          },
        ];
      case "openai_responses":
        return [
          {
            key: "openai_responses_base_url",
            label: "Base URL",
            placeholder: "https://api.openai.com/v1",
            hint: "API endpoint URL",
          },
          {
            key: "openai_responses_api_key",
            label: "API Key",
            placeholder: "sk-...",
            hint: "Your API key",
          },
          {
            key: "openai_responses_model",
            label: "Model",
            placeholder: "gpt-5.4",
            hint: "e.g., gpt-5.4, gpt-5",
          },
        ];
      case "openai_chat_completions":
        return [
          {
            key: "openai_chat_completions_base_url",
            label: "Base URL",
            placeholder: "https://api.openai.com/v1",
            hint: "API endpoint URL",
          },
          {
            key: "openai_chat_completions_api_key",
            label: "API Key",
            placeholder: "sk-...",
            hint: "Your API key",
          },
          {
            key: "openai_chat_completions_model",
            label: "Model",
            placeholder: "gpt-5.4",
            hint: "e.g., gpt-5.4, gpt-5",
          },
        ];
      case "openai_chat_completions_compatible":
        return [
          {
            key: "openai_chat_completions_compatible_base_url",
            label: "Base URL",
            placeholder: "https://api.openai.com/v1",
            hint: "API endpoint URL",
          },
          {
            key: "openai_chat_completions_compatible_api_key",
            label: "API Key",
            placeholder: "sk-...",
            hint: "Your API key",
          },
          {
            key: "openai_chat_completions_compatible_model",
            label: "Model",
            placeholder: "qwen3.5-plus",
            hint: "e.g., qwen3.5-plus, kimi-k2",
          },
        ];
      case "anthropic_messages":
        return [
          {
            key: "anthropic_messages_base_url",
            label: "Base URL",
            placeholder: "https://api.anthropic.com/v1",
            hint: "API endpoint URL",
          },
          {
            key: "anthropic_messages_api_key",
            label: "API Key",
            placeholder: "sk-ant-...",
            hint: "Your API key",
          },
          {
            key: "anthropic_messages_model",
            label: "Model",
            placeholder: "claude-3-5-sonnet-latest",
            hint: "e.g., claude-3-5-sonnet-latest, claude-3-7-sonnet-latest",
          },
        ];
    }
  };

  const saveConfig = () => {
    // Use flat format to match Config struct field names
    let configContent = `# Alan Agent Daemon Configuration
# Generated by alan init wizard

# Server configuration
bind_address = "127.0.0.1:8090"

# LLM Provider
llm_provider = "${selectedProvider}"
`;

    if (selectedProvider === "google_gemini_generate_content") {
      configContent += `
# Google Gemini GenerateContent API Configuration
google_gemini_generate_content_project_id = "${config.google_gemini_generate_content_project_id || ""}"
google_gemini_generate_content_location = "${config.google_gemini_generate_content_location || "us-central1"}"
google_gemini_generate_content_model = "${config.google_gemini_generate_content_model || "gemini-2.0-flash"}"
`;
    } else if (selectedProvider === "openai_responses") {
      configContent += `
# OpenAI Responses API Configuration
openai_responses_api_key = "${config.openai_responses_api_key || ""}"
openai_responses_base_url = "${config.openai_responses_base_url || "https://api.openai.com/v1"}"
openai_responses_model = "${config.openai_responses_model || "gpt-5.4"}"
`;
    } else if (selectedProvider === "openai_chat_completions") {
      configContent += `
# OpenAI Chat Completions API Configuration
openai_chat_completions_api_key = "${config.openai_chat_completions_api_key || ""}"
openai_chat_completions_base_url = "${config.openai_chat_completions_base_url || "https://api.openai.com/v1"}"
openai_chat_completions_model = "${config.openai_chat_completions_model || "gpt-5.4"}"
`;
    } else if (selectedProvider === "openai_chat_completions_compatible") {
      configContent += `
# OpenAI Chat Completions API-compatible Configuration
openai_chat_completions_compatible_api_key = "${config.openai_chat_completions_compatible_api_key || ""}"
openai_chat_completions_compatible_base_url = "${config.openai_chat_completions_compatible_base_url || "https://api.openai.com/v1"}"
openai_chat_completions_compatible_model = "${config.openai_chat_completions_compatible_model || "qwen3.5-plus"}"
`;
    } else if (selectedProvider === "anthropic_messages") {
      configContent += `
# Anthropic Messages API Configuration
anthropic_messages_api_key = "${config.anthropic_messages_api_key || ""}"
anthropic_messages_base_url = "${config.anthropic_messages_base_url || "https://api.anthropic.com/v1"}"
anthropic_messages_model = "${config.anthropic_messages_model || "claude-3-5-sonnet-latest"}"
`;
    }

    configContent += `
# Runtime Configuration
llm_request_timeout_secs = 180
tool_timeout_secs = 30

# Memory Configuration
[memory]
enabled = true
strict_workspace = true
`;

    mkdirSync(dirname(configPath), { recursive: true });
    writeFileSync(configPath, configContent, { mode: 0o600 });
    chmodSync(configPath, 0o600);
  };

  useInput((input, key) => {
    if (step === "welcome") {
      if (key.return) {
        setStep("provider");
      }
      return;
    }

    if (step === "provider") {
      if (key.upArrow) {
        setProviderCursor((c) => (c > 0 ? c - 1 : providers.length - 1));
      } else if (key.downArrow) {
        setProviderCursor((c) => (c < providers.length - 1 ? c + 1 : 0));
      } else if (key.return) {
        const selected = providers[providerCursor].key;
        setSelectedProvider(selected);
        // Initialize input with current config value
        const fields = getConfigFields(selected);
        setConfigFieldIndex(0);
        setInputValue(String(config[fields[0].key] || ""));
        setStep("config");
      }
      return;
    }

    if (step === "config") {
      const fields = getConfigFields(selectedProvider);
      const currentField = fields[configFieldIndex];

      if (key.escape) {
        // Go back to provider selection
        setStep("provider");
        setInputValue("");
        return;
      }

      if (key.return) {
        // Save current field value
        const newConfig = { ...config, [currentField.key]: inputValue };
        setConfig(newConfig);

        if (configFieldIndex < fields.length - 1) {
          // Move to next field
          const nextIndex = configFieldIndex + 1;
          setConfigFieldIndex(nextIndex);
          setInputValue(String(newConfig[fields[nextIndex].key] || ""));
        } else {
          // All fields completed, save and done
          saveConfig();
          setStep("done");
          setTimeout(onComplete, 2000);
        }
      }
      return;
    }
  });

  if (step === "welcome") {
    return (
      <Box flexDirection="column" padding={1}>
        <Text bold color="cyan">
          Welcome to Alan!
        </Text>
        <Text> </Text>
        <Text>Alan is an AI assistant that runs in your terminal.</Text>
        <Text> </Text>
        <Text>To get started, we need to configure your LLM provider.</Text>
        <Text> </Text>
        <Text color="gray">Press Enter to continue...</Text>
      </Box>
    );
  }

  if (step === "provider") {
    return (
      <Box flexDirection="column" padding={1}>
        <Text bold>Select your LLM provider:</Text>
        <Text> </Text>
        {providers.map((p, i) => (
          <Box key={p.key} flexDirection="column" marginBottom={1}>
            <Text color={providerCursor === i ? "green" : "white"}>
              {providerCursor === i ? "> " : "  "}
              {p.name}
            </Text>
            <Text color="gray"> {p.desc}</Text>
          </Box>
        ))}
        <Text> </Text>
        <Text color="gray">↑↓ to select, Enter to confirm</Text>
      </Box>
    );
  }

  if (step === "config") {
    const fields = getConfigFields(selectedProvider);
    const currentField = fields[configFieldIndex];
    const providerName =
      providers.find((p) => p.key === selectedProvider)?.name || "";

    return (
      <Box flexDirection="column" padding={1}>
        <Text bold>Configure {providerName}</Text>
        <Text color="gray">
          Step {configFieldIndex + 1} of {fields.length}
        </Text>
        <Text> </Text>

        {/* Show completed fields */}
        {fields.slice(0, configFieldIndex).map((field, idx) => (
          <Box key={field.key} flexDirection="row">
            <Text color="green">✓ </Text>
            <Text>{field.label}: </Text>
            <Text color="cyan">
              {field.key.includes("api_key") && config[field.key]
                ? "*".repeat(String(config[field.key]).length)
                : String(config[field.key] || "")}
            </Text>
          </Box>
        ))}

        {/* Current input field */}
        <Box flexDirection="column">
          <Box flexDirection="row">
            <Text color="yellow">→ </Text>
            <Text bold>{currentField.label}: </Text>
            <TextInput
              value={inputValue}
              onChange={setInputValue}
              placeholder={currentField.placeholder}
              mask={currentField.key.includes("api_key") ? "*" : undefined}
            />
          </Box>
          {currentField.hint && <Text color="gray"> {currentField.hint}</Text>}
        </Box>

        {/* Show remaining fields */}
        {fields.slice(configFieldIndex + 1).map((field) => (
          <Box key={field.key} flexDirection="row">
            <Text color="gray">○ {field.label}</Text>
          </Box>
        ))}

        <Text> </Text>
        <Text color="gray">Enter to continue, Esc to go back</Text>
      </Box>
    );
  }

  return (
    <Box flexDirection="column" padding={1}>
      <Text bold color="green">
        ✓ Configuration saved!
      </Text>
      <Text> </Text>
      <Text>Config file: {displayPath(configPath)}</Text>
      <Text> </Text>
      <Text>Starting Alan...</Text>
    </Box>
  );
}
