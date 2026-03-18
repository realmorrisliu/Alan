/**
 * First-time setup wizard for Alan
 *
 * Runs when no agent config exists at the canonical or override path.
 */

import React, { useState } from "react";
import { Box, Text, useInput } from "ink";
import TextInput from "ink-text-input";
import { chmodSync, mkdirSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";
import { homedir } from "node:os";
import {
  ADVANCED_PROVIDER_CATALOG,
  buildConfigContent,
  buildHostConfigContent,
  configForSetupSelection,
  configFieldsForSetup,
  DEFAULT_CONFIG,
  isConfigurableSetupOption,
  SERVICE_CATALOG,
  type ConfigValues,
  type ConfigurableSetupOption,
} from "./setup-catalog.js";

interface InitWizardProps {
  onComplete: () => void;
  agentConfigPath: string;
  hostConfigPath: string;
}

type WizardStep =
  | "welcome"
  | "service"
  | "advanced_provider"
  | "config"
  | "done";
type ConfigReturnStep = "service" | "advanced_provider";

function displayPath(path: string): string {
  const home = homedir();
  if (path === home) return "~";
  const homePrefix = `${home}/`;
  return path.startsWith(homePrefix)
    ? `~/${path.slice(homePrefix.length)}`
    : path;
}

const DEFAULT_TARGET =
  SERVICE_CATALOG.find(isConfigurableSetupOption) ??
  ADVANCED_PROVIDER_CATALOG[0];

export function InitWizard({
  onComplete,
  agentConfigPath,
  hostConfigPath,
}: InitWizardProps) {
  const [step, setStep] = useState<WizardStep>("welcome");
  const [selectedTarget, setSelectedTarget] =
    useState<ConfigurableSetupOption>(DEFAULT_TARGET);
  const [configReturnStep, setConfigReturnStep] =
    useState<ConfigReturnStep>("service");
  const [config, setConfig] = useState<ConfigValues>(DEFAULT_CONFIG);

  const [serviceCursor, setServiceCursor] = useState(0);
  const [advancedProviderCursor, setAdvancedProviderCursor] = useState(0);
  const [configFieldIndex, setConfigFieldIndex] = useState(0);
  const [inputValue, setInputValue] = useState("");

  const beginConfig = (
    option: ConfigurableSetupOption,
    returnStep: ConfigReturnStep,
  ) => {
    const nextConfig = configForSetupSelection(config, selectedTarget, option);
    const fields = configFieldsForSetup(option);
    setSelectedTarget(option);
    setConfigReturnStep(returnStep);
    setConfig(nextConfig);
    setConfigFieldIndex(0);
    setInputValue(String(nextConfig[fields[0].key] || ""));
    setStep("config");
  };

  const saveConfig = (
    option: ConfigurableSetupOption,
    values: ConfigValues,
  ) => {
    const agentConfigContent = buildConfigContent(option, values);
    const hostConfigContent = buildHostConfigContent();

    mkdirSync(dirname(agentConfigPath), { recursive: true });
    writeFileSync(agentConfigPath, agentConfigContent, { mode: 0o600 });
    chmodSync(agentConfigPath, 0o600);

    mkdirSync(dirname(hostConfigPath), { recursive: true });
    writeFileSync(hostConfigPath, hostConfigContent, { mode: 0o600 });
    chmodSync(hostConfigPath, 0o600);
  };

  useInput((_input, key) => {
    if (step === "welcome") {
      if (key.return) {
        setStep("service");
      }
      return;
    }

    if (step === "service") {
      if (key.upArrow) {
        setServiceCursor((cursor) =>
          cursor > 0 ? cursor - 1 : SERVICE_CATALOG.length - 1,
        );
      } else if (key.downArrow) {
        setServiceCursor((cursor) =>
          cursor < SERVICE_CATALOG.length - 1 ? cursor + 1 : 0,
        );
      } else if (key.return) {
        const option = SERVICE_CATALOG[serviceCursor];
        if (isConfigurableSetupOption(option)) {
          beginConfig(option, "service");
        } else {
          setStep("advanced_provider");
        }
      }
      return;
    }

    if (step === "advanced_provider") {
      if (key.upArrow) {
        setAdvancedProviderCursor((cursor) =>
          cursor > 0 ? cursor - 1 : ADVANCED_PROVIDER_CATALOG.length - 1,
        );
      } else if (key.downArrow) {
        setAdvancedProviderCursor((cursor) =>
          cursor < ADVANCED_PROVIDER_CATALOG.length - 1 ? cursor + 1 : 0,
        );
      } else if (key.escape) {
        setStep("service");
      } else if (key.return) {
        beginConfig(
          ADVANCED_PROVIDER_CATALOG[advancedProviderCursor],
          "advanced_provider",
        );
      }
      return;
    }

    if (step !== "config") {
      return;
    }

    const fields = configFieldsForSetup(selectedTarget);
    const currentField = fields[configFieldIndex];

    if (key.escape) {
      setInputValue("");
      setStep(configReturnStep);
      return;
    }

    if (!key.return) {
      return;
    }

    const nextConfig = { ...config, [currentField.key]: inputValue };
    setConfig(nextConfig);

    if (configFieldIndex < fields.length - 1) {
      const nextIndex = configFieldIndex + 1;
      setConfigFieldIndex(nextIndex);
      setInputValue(String(nextConfig[fields[nextIndex].key] || ""));
      return;
    }

    saveConfig(selectedTarget, nextConfig);
    setStep("done");
    setTimeout(onComplete, 2000);
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
        <Text>First, choose the service you want Alan to connect to.</Text>
        <Text> </Text>
        <Text color="gray">
          Alan will write canonical agent and host config files for you.
          Advanced / custom setup is still available.
        </Text>
        <Text> </Text>
        <Text color="gray">Press Enter to continue...</Text>
      </Box>
    );
  }

  if (step === "service") {
    let previousGroup: string | null = null;

    return (
      <Box flexDirection="column" padding={1}>
        <Text bold>Select a service to connect:</Text>
        <Text color="gray">
          You pick the service. Alan handles the underlying provider mapping.
        </Text>
        <Text> </Text>
        {SERVICE_CATALOG.map((option, index) => {
          const heading =
            option.group === previousGroup ? null : (
              <Text key={`${option.key}-group`} bold color="cyan">
                {option.group}
              </Text>
            );
          previousGroup = option.group;

          return (
            <Box key={option.key} flexDirection="column" marginBottom={1}>
              {heading}
              <Text color={serviceCursor === index ? "green" : "white"}>
                {serviceCursor === index ? "> " : "  "}
                {option.name}
              </Text>
              <Text color="gray"> {option.desc}</Text>
              <Text color="gray"> {option.detail}</Text>
            </Box>
          );
        })}
        <Text color="gray">↑↓ to select, Enter to continue</Text>
      </Box>
    );
  }

  if (step === "advanced_provider") {
    return (
      <Box flexDirection="column" padding={1}>
        <Text bold>Advanced / custom setup</Text>
        <Text color="gray">
          Choose the raw API family only if you need manual endpoint-level
          control.
        </Text>
        <Text> </Text>
        {ADVANCED_PROVIDER_CATALOG.map((option, index) => (
          <Box key={option.key} flexDirection="column" marginBottom={1}>
            <Text color={advancedProviderCursor === index ? "green" : "white"}>
              {advancedProviderCursor === index ? "> " : "  "}
              {option.name}
            </Text>
            <Text color="gray"> {option.desc}</Text>
            <Text color="gray"> {option.detail}</Text>
          </Box>
        ))}
        <Text color="gray">
          ↑↓ to select, Enter to continue, Esc to go back
        </Text>
      </Box>
    );
  }

  if (step === "config") {
    const fields = configFieldsForSetup(selectedTarget);
    const currentField = fields[configFieldIndex];
    const exposesBaseUrl = fields.some((field) =>
      field.key.includes("base_url"),
    );

    return (
      <Box flexDirection="column" padding={1}>
        <Text bold>Configure {selectedTarget.name}</Text>
        <Text color="gray">{selectedTarget.desc}</Text>
        <Text color="gray">{selectedTarget.detail}</Text>
        <Text color="gray">
          Alan writes canonical agent and host config for{" "}
          {selectedTarget.provider}.
        </Text>
        {!exposesBaseUrl &&
          selectedTarget.provider !== "google_gemini_generate_content" && (
            <Text color="gray">
              Endpoint defaults are prefilled for this service. Use Advanced /
              custom setup if you need to edit raw base URLs or switch API
              families.
            </Text>
          )}
        <Text color="gray">
          Step {configFieldIndex + 1} of {fields.length}
        </Text>
        <Text> </Text>

        {fields.slice(0, configFieldIndex).map((field) => (
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
      <Text>Selected service: {selectedTarget.name}</Text>
      <Text>Agent config: {displayPath(agentConfigPath)}</Text>
      <Text>Host config: {displayPath(hostConfigPath)}</Text>
      <Text> </Text>
      <Text>Starting Alan...</Text>
    </Box>
  );
}
