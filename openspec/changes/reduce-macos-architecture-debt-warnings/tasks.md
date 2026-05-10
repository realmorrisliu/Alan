## 1. Baseline And Sequencing

- [x] 1.1 Run `bash clients/apple/scripts/check-architecture-maintainability.sh` and record the current seven-warning baseline.
- [x] 1.2 Confirm `clients/apple/ARCHITECTURE.md` names each current warning class and explains why it is non-blocking migration debt.
- [x] 1.3 Confirm the first implementation slice targets the smallest AppKit-leak warning unless discovery shows a lower-risk dependency order.

## 2. Terminal Runtime Registry AppKit Warning

- [x] 2.1 Inspect why `TerminalRuntimeRegistry.swift` imports AppKit and identify the durable owner for that dependency.
- [x] 2.2 Move or isolate the AppKit dependency so `TerminalRuntimeRegistry.swift` no longer triggers the architecture warning.
- [x] 2.3 Run the architecture report and focused terminal runtime validation.
- [x] 2.4 Update `clients/apple/ARCHITECTURE.md` to remove or narrow the `TerminalRuntimeRegistry.swift` debt entry and record the new warning count.
- [x] 2.5 Commit and open the first stacked PR for the resolved warning.

## 3. Shell Host Controller Debt

- [x] 3.1 Identify the next `ShellHostController.swift` controller, store, projection, or command-routing boundary that can move without changing behavior.
- [x] 3.2 Extract the selected boundary into a named owner while preserving `ShellWorkspaceCommand` as the shared command vocabulary.
- [x] 3.3 Run shell contract validation and the architecture report.
- [x] 3.4 Update the debt ledger and warning expectation if the `ShellHostController.swift` warning count is reduced.
- [x] 3.5 Commit and open the next stacked PR.

## 4. Terminal Surface And Host Debt

- [x] 4.1 Identify the next `TerminalSurfaceController.swift` adapter or input/surface boundary that can move without changing behavior.
- [x] 4.2 Extract the selected terminal surface boundary and run focused terminal surface validation.
- [x] 4.3 Identify the next `TerminalHostView.swift` collaborator boundary for runtime attachment, overlay presentation, input routing, window observation, metadata publishing, or surface coordination.
- [x] 4.4 Extract the selected terminal host boundary and run focused terminal host/runtime validation.
- [x] 4.5 Update `clients/apple/ARCHITECTURE.md` and script expectations after each reduced warning.
- [x] 4.6 Commit and open stacked PRs for each reduced terminal warning.

## 5. Console Content View Debt

- [ ] 5.1 Inspect `Views/Console/ContentView.swift` and identify the lowest-risk console/mobile boundary to extract first.
- [ ] 5.2 Move the selected console view, model projection, or platform bridge code into a named console owner while preserving primary macOS shell separation.
- [ ] 5.3 Run the architecture report and relevant Apple build or focused validation for the touched console path.
- [ ] 5.4 Update `clients/apple/ARCHITECTURE.md` and script expectations if the console warning count is reduced.
- [ ] 5.5 Commit and open the final stacked PR for this change's planned console debt reduction.

## 6. Verification And Archive Readiness

- [ ] 6.1 Run `openspec validate reduce-macos-architecture-debt-warnings --type change --strict --json`.
- [ ] 6.2 Run `openspec validate --all --strict --json`.
- [ ] 6.3 Run `git diff --check`.
- [ ] 6.4 Before archive, sync the accepted requirements into `openspec/specs/macos-app-architecture-maintainability/spec.md`.
- [ ] 6.5 Archive the change after all selected warning-reduction slices are merged and the debt ledger is current.
