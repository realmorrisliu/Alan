## 1. Runtime Boundary

- [ ] 1.1 Add process bootstrap protocols for Ghostty initialization, dependency checks, and bootstrap diagnostics.
- [ ] 1.2 Add a window-scoped `AlanTerminalRuntimeService` protocol and production service type.
- [ ] 1.3 Add pane-scoped surface handle types for lifecycle phase, metadata snapshot, delivery state, and teardown status.
- [ ] 1.4 Add fake bootstrap, fake runtime service, and fake surface handles for focused tests.

## 2. Ghostty Ownership Migration

- [ ] 2.1 Move libghostty initialization and resource setup out of host views into the process bootstrap.
- [ ] 2.2 Move Ghostty app/window runtime ownership out of `AlanGhosttyLiveHost` into the window runtime service.
- [ ] 2.3 Move pane surface creation and teardown behind service-owned surface handles.
- [ ] 2.4 Remove or disable view-owned Ghostty app/surface creation paths after the service path is active.

## 3. View Attachment

- [ ] 3.1 Update `TerminalRuntimeRegistry` and shell controller creation paths to resolve pane runtime handles from the service.
- [ ] 3.2 Update `TerminalHostView` and `AlanTerminalHostNSView` so they attach to existing handles and report view metrics.
- [ ] 3.3 Preserve keyboard, mouse, IME, scroll, focus, occlusion, and backing-scale forwarding through the adapter path.
- [ ] 3.4 Fold overlapping activation/event ownership tasks from `converge-terminal-event-ownership` into the new adapter boundary.

## 4. Shell And Control Plane Integration

- [ ] 4.1 Route pane focus, runtime metadata, and renderer/readiness state through service snapshots keyed by pane ID.
- [ ] 4.2 Route `pane.send_text` through the runtime service and return accepted, queued, missing, timeout, or rejected states authoritatively.
- [ ] 4.3 Update pane, tab, window, and app close paths to finalize affected surface handles exactly once.
- [ ] 4.4 Surface runtime service diagnostics in the inspector/debug data without adding default UI jargon.

## 5. Verification

- [ ] 5.1 Add unit tests for bootstrap reuse, pane handle reattachment, text delivery, and teardown-once behavior.
- [ ] 5.2 Add control-plane tests for accepted text, queued text if supported, runtime-missing errors, and timeout diagnostics.
- [ ] 5.3 Run `git diff --check`.
- [ ] 5.4 Run `bash clients/apple/scripts/check-shell-contracts.sh`.
- [ ] 5.5 Build the macOS app with the documented `AlanNative` command.
- [ ] 5.6 Manually verify terminal continuity across tab switching, split resizing, pane close, tab close, and multi-window use.

## 6. PR And Archive Readiness

- [ ] 6.1 Update proposal/task notes with any implementation decisions that differ from this design.
- [ ] 6.2 Review the diff for retain cycles between runtime service, shell controller, host views, and callbacks.
- [ ] 6.3 Before archive, sync accepted delta requirements into `openspec/specs/`.
- [ ] 6.4 Archive the OpenSpec change after implementation is merged.
