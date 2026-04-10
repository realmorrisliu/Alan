import { afterEach, describe, expect, test } from "bun:test";
import { AlanClient } from "./client.js";
import type { EventEnvelope } from "./types.js";

function eventId(sequence: number): string {
  return `evt_${sequence.toString().padStart(16, "0")}`;
}

function makeEnvelope(sequence: number): EventEnvelope {
  return {
    event_id: eventId(sequence),
    sequence,
    session_id: "sess-test",
    turn_id: "turn_000001",
    item_id: `item_000001_${sequence.toString().padStart(4, "0")}`,
    timestamp_ms: sequence,
    type: "text_delta",
    chunk: `chunk-${sequence}`,
    is_final: false,
  };
}

function jsonResponse(body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status: 200,
    headers: { "Content-Type": "application/json" },
  });
}

const originalFetch = globalThis.fetch;
type FetchImpl = (
  input: Parameters<typeof fetch>[0],
  init?: Parameters<typeof fetch>[1],
) => Promise<Response>;

function installMockFetch(mockImpl: FetchImpl): void {
  globalThis.fetch = mockImpl as unknown as typeof fetch;
}

afterEach(() => {
  globalThis.fetch = originalFetch;
});

describe("AlanClient replay", () => {
  test("advances replay cursor with last event in page instead of latest_event_id", async () => {
    const client = new AlanClient({
      url: "ws://example.com",
      autoManageDaemon: false,
    });

    (client as any).lastEventId = eventId(1);
    (client as any).connectionVersion = 1;

    const replayedIds: string[] = [];
    client.on("event", (envelope) => {
      replayedIds.push(envelope.event_id);
    });

    const requestAfterIds: string[] = [];
    const page1 = Array.from({ length: 200 }, (_, idx) =>
      makeEnvelope(idx + 2),
    );
    const page2 = Array.from({ length: 200 }, (_, idx) =>
      makeEnvelope(idx + 202),
    );

    installMockFetch(async (input): Promise<Response> => {
      const requestUrl =
        typeof input === "string"
          ? new URL(input)
          : input instanceof URL
            ? input
            : new URL(input.url);
      const after = requestUrl.searchParams.get("after_event_id");
      if (!after) {
        throw new Error("Expected after_event_id");
      }
      requestAfterIds.push(after);

      if (after === eventId(1)) {
        return jsonResponse({
          gap: false,
          oldest_event_id: eventId(2),
          latest_event_id: eventId(401),
          events: page1,
        });
      }
      if (after === eventId(201)) {
        return jsonResponse({
          gap: false,
          oldest_event_id: eventId(2),
          latest_event_id: eventId(401),
          events: page2,
        });
      }

      throw new Error(`Unexpected replay cursor: ${after}`);
    });

    await (client as any).replayMissedEvents("sess-test", 1);

    expect(requestAfterIds).toEqual([eventId(1), eventId(201)]);
    expect(replayedIds).toHaveLength(400);
    expect(replayedIds[0]).toBe(eventId(2));
    expect(replayedIds[399]).toBe(eventId(401));
    expect((client as any).lastEventId).toBe(eventId(401));
  });

  test("fails replay when gap is reported but no replayable events are returned", async () => {
    const client = new AlanClient({
      url: "ws://example.com",
      autoManageDaemon: false,
    });

    (client as any).lastEventId = eventId(42);
    (client as any).connectionVersion = 7;

    const errors: string[] = [];
    client.on("error", (error) => {
      errors.push(error.message);
    });

    installMockFetch(async () =>
      jsonResponse({
        gap: true,
        oldest_event_id: eventId(100),
        latest_event_id: eventId(200),
        events: [],
      }),
    );

    await expect(
      (client as any).replayMissedEvents("sess-test", 7),
    ).rejects.toThrow(
      "Event replay gap detected but no replayable events were returned",
    );
    expect(errors).toHaveLength(1);
    expect(errors[0]).toContain("Event replay gap detected");
  });

  test("captures and restores replay state snapshots", () => {
    const client = new AlanClient({
      url: "ws://example.com",
      autoManageDaemon: false,
    });

    (client as any).currentSessionId = "sess-test";
    (client as any).lastEventId = eventId(10);
    (client as any).seenEventIds = [eventId(8), eventId(9), eventId(10)];
    (client as any).seenEventSet = new Set([
      eventId(8),
      eventId(9),
      eventId(10),
    ]);

    const snapshot = client.captureReplayState();

    expect(snapshot).toEqual({
      sessionId: "sess-test",
      lastEventId: eventId(10),
      seenEventIds: [eventId(8), eventId(9), eventId(10)],
    });

    (client as any).resetReplayState();
    expect((client as any).lastEventId).toBeNull();

    (client as any).restoreReplayState(snapshot);
    expect((client as any).lastEventId).toBe(eventId(10));
    expect((client as any).seenEventIds).toEqual([
      eventId(8),
      eventId(9),
      eventId(10),
    ]);
    expect((client as any).seenEventSet.has(eventId(10))).toBe(true);
  });
});

describe("AlanClient connections", () => {
  test("reads the connection catalog from the daemon connection surface", async () => {
    const client = new AlanClient({
      url: "ws://example.com",
      autoManageDaemon: false,
    });

    let requestedUrl = "";
    installMockFetch(async (input): Promise<Response> => {
      requestedUrl =
        typeof input === "string"
          ? input
          : input instanceof URL
            ? input.toString()
            : input.url;
      return jsonResponse({
        providers: [
          {
            provider_id: "chatgpt",
            display_name: "ChatGPT / Codex",
            credential_kind: "managed_oauth",
            supports_browser_login: true,
            supports_device_login: true,
            supports_secret_entry: false,
            supports_logout: true,
            supports_test: true,
            required_settings: ["base_url", "model"],
            optional_settings: ["account_id"],
            default_settings: {
              base_url: "https://chatgpt.com/backend-api/codex",
              model: "gpt-5-codex",
              account_id: "",
            },
          },
        ],
      });
    });

    const catalog = await client.getConnectionCatalog();

    expect(requestedUrl).toBe(
      "http://example.com/api/v1/connections/catalog",
    );
    expect(catalog.providers).toHaveLength(1);
    expect(catalog.providers[0].provider_id).toBe("chatgpt");
  });

  test("starts managed browser login through the connection control plane", async () => {
    const client = new AlanClient({
      url: "ws://example.com",
      autoManageDaemon: false,
    });

    let requestedUrl = "";
    let requestedMethod = "";
    let requestedBody = "";
    installMockFetch(async (input, init): Promise<Response> => {
      requestedUrl =
        typeof input === "string"
          ? input
          : input instanceof URL
            ? input.toString()
            : input.url;
      requestedMethod = init?.method ?? "GET";
      requestedBody =
        typeof init?.body === "string" ? init.body : String(init?.body ?? "");
      return jsonResponse({
        login_id: "browser_123",
        auth_url: "https://chatgpt.com/oauth/authorize?state=abc",
        redirect_uri:
          "http://localhost:1455/auth/callback",
        created_at: "2026-04-08T00:00:00Z",
        expires_at: "2026-04-08T00:10:00Z",
      });
    });

    const start = await client.startConnectionBrowserLogin("chatgpt-main", {
      timeout_secs: 120,
    });

    expect(requestedUrl).toBe(
      "http://example.com/api/v1/connections/chatgpt-main/credential/login/browser/start",
    );
    expect(requestedMethod).toBe("POST");
    expect(requestedBody).toBe(JSON.stringify({ timeout_secs: 120 }));
    expect(start.login_id).toBe("browser_123");
    expect(start.redirect_uri).toBe("http://localhost:1455/auth/callback");
  });

  test("reads and updates connection selection state through the daemon connection surface", async () => {
    const client = new AlanClient({
      url: "ws://example.com",
      autoManageDaemon: false,
    });

    const requests: Array<{ url: string; method: string; body: string }> = [];
    installMockFetch(async (input, init): Promise<Response> => {
      const url =
        typeof input === "string"
          ? input
          : input instanceof URL
            ? input.toString()
            : input.url;
      const method = init?.method ?? "GET";
      const body =
        typeof init?.body === "string" ? init.body : String(init?.body ?? "");
      requests.push({ url, method, body });

      if (url.includes("/api/v1/connections/current")) {
        return jsonResponse({
          default_profile: "chatgpt",
          effective_profile: "kimi",
          effective_source: "global_pin",
          global_pin: {
            scope: "global",
            config_path: "/Users/example/.alan/agent/agent.toml",
            profile_id: "kimi",
          },
        });
      }

      return jsonResponse({
        default_profile: "chatgpt",
        effective_profile: "chatgpt",
        effective_source: "default_profile",
      });
    });

    const current = await client.getConnectionCurrent("/tmp/workspace");
    const updated = await client.setConnectionDefault({
      profile_id: "chatgpt",
      workspace_dir: "/tmp/workspace",
    });

    expect(requests[0]?.url).toBe(
      "http://example.com/api/v1/connections/current?workspace_dir=%2Ftmp%2Fworkspace",
    );
    expect(requests[1]?.url).toBe(
      "http://example.com/api/v1/connections/default/set",
    );
    expect(requests[1]?.method).toBe("POST");
    expect(requests[1]?.body).toBe(
      JSON.stringify({
        profile_id: "chatgpt",
        workspace_dir: "/tmp/workspace",
      }),
    );
    expect(current.effective_source).toBe("global_pin");
    expect(updated.effective_profile).toBe("chatgpt");
  });
});
