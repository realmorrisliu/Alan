## 1. Terminal Runtime Ownership

- [x] 1.1 Add a pane-keyed terminal runtime handle protocol and registry that can be owned by the shell host/model layer.
- [x] 1.2 Add a mock terminal runtime implementation for tests, including attach, detach, send text, metadata update, and teardown observation.
- [x] 1.3 Move terminal process, renderer phase, cwd, title, attention, and last-command metadata updates onto stable pane IDs.
- [x] 1.4 Update tab and pane close paths to tear down registry-owned runtimes exactly once.
- [x] 1.5 Update `TerminalHostView` so view removal detaches from a runtime but does not tear down the pane process.
- [x] 1.6 Add lifecycle tests for switching tabs, returning to a tab, closing a pane, and closing a tab.

## 2. Text Delivery And Mutation Acknowledgements

- [x] 2.1 Define stable control-plane result types/error codes for accepted, queued, rejected, missing target, unavailable runtime, and timeout outcomes.
- [x] 2.2 Route `pane.send_text` through the runtime registry instead of treating NotificationCenter delivery as success.
- [x] 2.3 Implement background-pane text delivery for existing runtimes without changing the selected tab.
- [x] 2.4 Implement the unavailable-runtime behavior chosen for this change: explicit rejection or a durable pane-specific queue with flush tests.
- [x] 2.5 Update control-plane responses and shell events to include accepted byte counts or stable rejection details.
- [x] 2.6 Add tests for visible delivery, background delivery, missing pane, closed pane, and unavailable runtime responses.

## 3. Window-Scoped Shell Context

- [x] 3.1 Introduce a per-window shell context containing `window_id`, control directory, socket path, state path, event path, and runtime registry.
- [x] 3.2 Replace fixed `window_main` state/socket/control paths with context-derived paths in the app scene and shell controller.
- [x] 3.3 Ensure a second `WindowGroup` instance creates or restores a distinct context and does not share shell state with the first window.
- [x] 3.4 Add best-effort migration or clear handling for legacy fixed-path shell state files.
- [x] 3.5 Add tests for opening two contexts, publishing independent state, and querying only one window's data.

## 4. IPC, Persistence, And Diagnostics

- [x] 4.1 Add maximum request byte size enforcement to the local shell control socket.
- [x] 4.2 Add request read deadlines and command execution deadlines so stalled clients cannot block later clients.
- [x] 4.3 Refactor socket handling so one slow client or main-actor command does not stall the accept loop.
- [x] 4.4 Convert shell state, event, binding, and file-command IO operations to record or return failures.
- [x] 4.5 Add inspectable diagnostics for recent control-plane persistence, decode, timeout, and delivery failures.
- [x] 4.6 Add tests for no-newline clients, oversized requests, slow command handling, state write failure, and undecodable file commands.

## 5. macOS Shell UI Conformance

- [x] 5.1 Audit `MacShellRootView`, `TerminalPaneView`, and command UI against the impeccable/OpenSpec design acceptance layer before refactoring.
- [x] 5.2 Refactor the sidebar into a compact space rail plus active-space tab list.
- [x] 5.3 Separate space creation and tab creation affordances so each lives in its expected rail/list or toolbar context.
- [x] 5.4 Update space selection so the tab list filters to the active space and preserves terminal runtime identity.
- [x] 5.5 Make tab rows compact, stable, and skimmable, with row-level attention/status treatment instead of cards or repeated pills.
- [x] 5.6 Make the single-pane terminal region visually dominant without a pane selector strip or nested decorative panel.
- [x] 5.7 Keep split-pane chrome lightweight and use subtle focus treatment instead of engineering labels.
- [x] 5.8 Replace default raw identifiers and runtime jargon with product terms in normal workflows, including command search result titles and summaries.
- [x] 5.9 Move raw pane IDs, socket paths, runtime phases, binding data, and JSON snapshots behind an explicit Debug inspector layer.
- [x] 5.10 Restrain the toolbar to current context, `Go to or Command...`, frequent actions, and optional inspector toggle.
- [x] 5.11 Apply the light-mode native-material pass to the sidebar, toolbar, terminal surround, and inspector so the shell does not read as a hard-coded themed dashboard.
- [x] 5.12 Capture or record default, Overview inspector, and Debug inspector UI review evidence before marking UI conformance complete.

## 6. Build Contract And Dependency Setup

- [x] 6.1 Align Apple README and relevant specs with the Xcode project deployment targets.
- [x] 6.2 Add or document a supported Ghostty dependency preparation/check command for framework, resources, and terminfo artifacts.
- [x] 6.3 Make missing Ghostty artifacts fail with actionable setup guidance instead of opaque project or linker errors.
- [x] 6.4 Remove or suppress avoidable Ghostty module-map/umbrella-header warnings that obscure real build failures.
- [x] 6.5 Add a documented macOS build verification command for the prepared dependency state.

## 7. Verification

- [x] 7.1 Run focused Apple tests for shell model mutation, runtime delivery, window isolation, and control-plane behavior.
- [x] 7.2 Run the documented macOS Xcode build command after preparing Ghostty dependencies.
- [x] 7.3 Run `openspec status --change stabilize-macos-shell-terminal` and confirm the change is implementation-complete.
- [x] 7.4 Update review notes or follow-up OpenSpec items for any intentionally deferred runtime queueing, migration, or UI debug-surface decisions.
- [x] 7.5 Confirm UI review evidence covers the default light-mode shell, Overview inspector, and Debug inspector states.
