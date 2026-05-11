## 1. Material Roles

- [ ] 1.1 Inventory active shell material, background, button, hover, and overlay fills in the Apple client.
- [ ] 1.2 Define semantic material/control roles in the shell support layer for window, sidebar, workspace, terminal surround, floating overlay, and compact controls.
- [ ] 1.3 Keep AppKit visual-effect bridge code isolated behind reusable support wrappers.

## 2. Active Shell Application

- [ ] 2.1 Apply the semantic roles to `MacShellRootView.swift` and workspace backgrounds.
- [ ] 2.2 Apply the sidebar/control roles to `ShellSidebarView.swift` without changing sidebar information architecture.
- [ ] 2.3 Apply terminal surround roles to active terminal shell chrome without reducing terminal text contrast.
- [ ] 2.4 Apply overlay/control roles to active floating shell surfaces without redesigning `Command-K` behavior.

## 3. Verification

- [ ] 3.1 Run focused Apple build or shell UI checks affected by the changed files.
- [ ] 3.2 Capture screenshot or manual review notes for light-mode sidebar, terminal, command entry, controls, and overlays.
- [ ] 3.3 Review reduced-transparency or increased-contrast behavior, or document why local verification was unavailable.
- [ ] 3.4 Run `openspec validate --all --strict` before PR.
