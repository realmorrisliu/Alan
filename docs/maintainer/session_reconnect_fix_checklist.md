# Session Reconnect Fix Checklist

This checklist tracks the fixes identified from sessions `04cc2106` and
`1d5d6824`.

## Stack 1: TUI reconnect error handling

- [x] Reclassify replay `404 Not Found` as a session lifecycle problem instead of a raw transport error.
- [x] Stop surfacing misleading `Disconnected from agent` when reconnect fails before initialization completes.
- [x] Add TUI client tests covering replay `404` for active and missing sessions.

## Stack 2: Archived session read-only fallback

- [ ] Let daemon `read_session` fall back to rollout-only archived sessions when no active session entry exists.
- [ ] Let daemon `history` fall back to rollout-only archived sessions when no active session entry exists.
- [ ] Return `active: false` for archived/read-only session reads.
- [ ] Add route tests covering archived session read/history fallback.

## Stack 3: 04cc quality fixes

- [ ] Preserve whitespace-only streaming text deltas so spaces and newlines are not dropped from assistant output.
- [ ] Add regression tests for whitespace-preserving streamed text assembly.
- [ ] Make bash tool guidance explicit about avoiding opaque interpreter wrappers that sandbox preflight will reject.
