## 1. Manifest Model And Store

- [x] 1.1 Add `ShellWorkspaceManifest`, Space record, Tab record, pin snapshot, live snapshot, and active-task value types with `Codable` schema versioning.
- [x] 1.2 Add a manifest persistence store under Application Support using `shell-workspace-window_main.json`, atomic writes, sorted/pretty JSON, and timestamped corrupt-file quarantine.
- [x] 1.3 Add default manifest creation for a missing manifest without reading or migrating `shell-state-window_main.json`.
- [x] 1.4 Add focused model/store tests for missing manifest startup and corrupt manifest quarantine.

## 2. Materialization And Lifecycle Pruning

- [x] 2.1 Add a manifest materializer that converts retained manifest Spaces/Tabs into the current `ShellStateSnapshot` shape and preserves empty Space selection.
- [x] 2.2 Implement Unpinned Tab pruning using `now - max(lastActivatedAt, lastActivityAt) > 12h` and active-task protection.
- [x] 2.3 Implement selection repair when the selected Tab is pruned, including selected empty Space with `selectedTabID = nil`.
- [x] 2.4 Keep the materializer adapter small enough to swap from current pane-shaped state to future content-container state without changing manifest semantics.

## 3. Controller And Runtime Synchronization

- [x] 3.1 Change primary macOS shell startup to load the workspace manifest and materialize shell state instead of forcing fresh bootstrap.
- [x] 3.2 Sync Space creation, explicit Space deletion, Tab creation, Tab close, Tab selection, and Space selection from `ShellHostController` into the manifest.
- [x] 3.3 Add pin, unpin, and update-pin controller mutations that write explicit restore snapshots without auto-updating snapshots on later transient changes.
- [x] 3.4 Project terminal cwd/title/activity metadata into manifest live snapshots and `lastActivityAt` without storing runtime-only renderer/debug state.
- [x] 3.5 Add terminal-aware active-task projection for foreground command activity, alan active/pending/yield states, idle shell, and exited terminal states.
- [x] 3.6 Ensure lifecycle retirement finalizes affected terminal runtimes through the runtime service path used by close operations.

## 4. UI And Control Surfaces

- [x] 4.1 Update sidebar/workspace rendering so empty Spaces remain visible and show a restrained empty state with a new-tab action.
- [x] 4.2 Add pin/unpin/update-pin affordances in the existing Arc-like sidebar/tab interaction model without adding dashboard-style chrome.
- [x] 4.3 Update close-tab behavior so closing the last Tab in a Space leaves the Space empty rather than deleting it.
- [x] 4.4 Update shell events or diagnostics where useful so manifest load failures, corrupt-file quarantine, lifecycle retirement, and pin updates are inspectable.

## 5. Verification And Archive Readiness

- [x] 5.1 Add focused Swift tests for Pinned Tab single-pane restore, Pinned Tab split restore, and post-pin transient changes not mutating the pin snapshot.
- [x] 5.2 Add focused Swift tests for retained Unpinned Tabs inside TTL, retired inactive Unpinned Tabs after TTL, and selected-tab pruning repair.
- [x] 5.3 Add focused Swift tests for active-task protection: foreground command, alan pending/yield, and idle shell eligibility for retirement.
- [x] 5.4 Update shell contract checks to prevent `ShellStateSnapshot` from becoming the workspace restore authority again.
- [x] 5.5 Run focused Apple shell scripts affected by manifest, metadata, and lifecycle changes.
- [x] 5.6 Run the macOS app build or document the exact local dependency blocker.
- [x] 5.7 Validate `persist-macos-shell-workspaces` with `openspec validate persist-macos-shell-workspaces --strict`.
- [x] 5.8 Run `openspec validate --all --strict` and `git diff --check`.
- [x] 5.9 After implementation is merged, sync accepted requirements into `openspec/specs/` and prepare archive-readiness notes.
