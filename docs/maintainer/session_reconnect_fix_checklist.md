# Session Reconnect Fix Checklist

This checklist tracks the fixes identified from sessions `04cc2106` and
`1d5d6824`.

## Stack 1: TUI reconnect error handling

- [x] Reclassify replay `404 Not Found` as a session lifecycle problem instead of a raw transport error.
- [x] Stop surfacing misleading `Disconnected from agent` when reconnect fails before initialization completes.
- [x] Add TUI client tests covering replay `404` for active and missing sessions.

## Stack 2: Archived session lifecycle

- [x] Archive TTL-expired sessions in place instead of hard-deleting their persisted binding.
- [x] Return `active: false` for recovered or archived sessions in `get_session`, `list_sessions`, and `read_session`.
- [x] Keep archived sessions resumable by preserving their session entry, replay buffer, and rollout binding.
- [x] Add daemon tests covering archived session cleanup and inactive session reads.

## Stack 3: 04cc quality fixes

- [x] Preserve whitespace-only streaming text deltas so spaces and newlines are not dropped from assistant output.
- [x] Add regression tests for whitespace-preserving streamed text assembly.
- [x] Make bash tool guidance explicit about avoiding opaque interpreter wrappers that sandbox preflight will reject.
