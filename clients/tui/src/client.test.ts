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
});

describe("AlanClient auth", () => {
  test("reads ChatGPT auth status from the daemon auth surface", async () => {
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
        provider: "chatgpt",
        kind: "logged_in",
        account_id: "acct_123",
        email: "user@example.com",
      });
    });

    const snapshot = await client.getChatgptAuthStatus();

    expect(requestedUrl).toBe(
      "http://example.com/api/v1/auth/providers/chatgpt/status",
    );
    expect(snapshot.kind).toBe("logged_in");
    expect(snapshot.account_id).toBe("acct_123");
  });

  test("starts ChatGPT browser login through the daemon-owned flow", async () => {
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
          "http://127.0.0.1:8090/api/v1/auth/providers/chatgpt/login/browser/callback/browser_123",
        created_at: "2026-04-08T00:00:00Z",
        expires_at: "2026-04-08T00:10:00Z",
      });
    });

    const start = await client.startChatgptBrowserLogin({
      workspace_id: "acct_123",
      timeout_secs: 120,
    });

    expect(requestedUrl).toBe(
      "http://example.com/api/v1/auth/providers/chatgpt/login/browser/start",
    );
    expect(requestedMethod).toBe("POST");
    expect(requestedBody).toBe(
      JSON.stringify({ workspace_id: "acct_123", timeout_secs: 120 }),
    );
    expect(start.login_id).toBe("browser_123");
    expect(start.redirect_uri).toContain("/browser/callback/browser_123");
  });
});
