import { describe, expect, test } from "bun:test";
import { resolveBrowserOpenCommand } from "./open-url.js";

describe("resolveBrowserOpenCommand", () => {
  test("uses rundll32 on Windows to avoid cmd start quoting issues", () => {
    const command = resolveBrowserOpenCommand(
      "https://chatgpt.com/oauth/authorize?state=abc&code=123",
      "win32",
    );

    expect(command).toEqual({
      command: "rundll32",
      args: [
        "url.dll,FileProtocolHandler",
        "https://chatgpt.com/oauth/authorize?state=abc&code=123",
      ],
    });
  });

  test("prefers explicit BROWSER override on every platform", () => {
    const command = resolveBrowserOpenCommand(
      "https://chatgpt.com/oauth/authorize?state=abc&code=123",
      "win32",
      "custom-browser",
    );

    expect(command).toEqual({
      command: "custom-browser",
      args: ["https://chatgpt.com/oauth/authorize?state=abc&code=123"],
    });
  });

  test("splits BROWSER override arguments before appending the auth URL", () => {
    const command = resolveBrowserOpenCommand(
      "https://chatgpt.com/oauth/authorize?state=abc&code=123",
      "linux",
      "firefox --new-window --profile default",
    );

    expect(command).toEqual({
      command: "firefox",
      args: [
        "--new-window",
        "--profile",
        "default",
        "https://chatgpt.com/oauth/authorize?state=abc&code=123",
      ],
    });
  });

  test("keeps quoted BROWSER tokens together", () => {
    const command = resolveBrowserOpenCommand(
      "https://chatgpt.com/oauth/authorize?state=abc&code=123",
      "darwin",
      "\"/Applications/Google Chrome.app/Contents/MacOS/Google Chrome\" --profile-directory=Default",
    );

    expect(command).toEqual({
      command: "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
      args: [
        "--profile-directory=Default",
        "https://chatgpt.com/oauth/authorize?state=abc&code=123",
      ],
    });
  });
});
