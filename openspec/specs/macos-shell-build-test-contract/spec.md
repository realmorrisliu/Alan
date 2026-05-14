# macos-shell-build-test-contract Specification

## Purpose
Define the Apple client build, dependency, and focused test contract for the
macOS shell host.
## Requirements
### Requirement: Build requirements match documentation
The Apple client SHALL keep documented system requirements, deployment targets,
project settings, project naming, scheme naming, and build commands aligned
with the active `alan for macOS` engineering identity.

#### Scenario: Deployment target changes
- **WHEN** the Xcode project deployment targets are changed
- **THEN** `clients/apple/README.md` and relevant specs are updated in the same
  change

#### Scenario: Documented build command
- **WHEN** a developer runs the documented macOS build command after preparing
  dependencies
- **THEN** the command succeeds or fails with documented, actionable dependency
  setup instructions

#### Scenario: Project or scheme name changes
- **WHEN** the Apple project, source root, target, scheme, or generated product
  name changes
- **THEN** `clients/apple/README.md`, architecture docs, focused scripts, active
  OpenSpec tasks, and Xcode project settings are updated in the same change
- **AND** no active documented build command references `AlanNative`

### Requirement: Ghostty dependency setup is explicit
The Apple project SHALL treat Ghostty framework, resources, and terminfo as an
explicit local dependency with a verifiable setup path.

#### Scenario: Dependencies are missing
- **WHEN** `GhosttyKit.xcframework`, `ghostty-resources`, or `ghostty-terminfo` are absent
- **THEN** the build or setup check reports the missing dependency and points to the supported preparation command

#### Scenario: Dependencies are present
- **WHEN** local Ghostty artifacts are prepared
- **THEN** the macOS app build links/copies them without module-map or umbrella-header warnings that obscure real failures

### Requirement: Shell model behavior has focused tests
The Apple client SHALL have focused automated tests for shell state mutation and
control-plane behavior that can run without launching the full app UI.

#### Scenario: State mutation tests
- **WHEN** shell spaces, tabs, and panes are created, split, moved, lifted, focused, and closed
- **THEN** tests verify focused IDs, pane trees, space membership, attention state, and failure cases

#### Scenario: Control-plane tests
- **WHEN** control-plane query and mutation commands are executed against a test host
- **THEN** tests verify successful responses, missing-target errors, event records, and text-delivery acknowledgement semantics

### Requirement: Terminal host boundary is testable
The terminal host SHALL expose a testable boundary for runtime attachment,
teardown, and text delivery without requiring the real Ghostty library in every
test.

#### Scenario: Mock runtime accepts text
- **WHEN** a test runtime is registered for a pane and `pane.send_text` is issued
- **THEN** the test verifies the text reaches the runtime and the control response reports accepted bytes

#### Scenario: Mock runtime unavailable
- **WHEN** no runtime is registered for a pane and `pane.send_text` is issued
- **THEN** the test verifies the response reports failure or durable queueing according to the delivery contract

### Requirement: Surface behavior has focused verification
The Apple client SHALL add focused tests or documented manual verification for
terminal scrollback, input translation, IME/preedit, selection, clipboard,
search, terminal mode changes, renderer health, and child-exit behavior.

#### Scenario: Scrollback verification
- **WHEN** terminal surface work changes scrollback or scrollbar behavior
- **THEN** tests or manual notes verify normal-buffer scrolling, alternate-screen behavior, and scrollbar synchronization

#### Scenario: Input verification
- **WHEN** terminal input adapter behavior changes
- **THEN** tests or manual notes verify printable input, command-key routing, modifiers, IME composition, paste, and terminal mouse mode

#### Scenario: Failure-state verification
- **WHEN** renderer health, child-exit, or fallback UI changes
- **THEN** tests or manual notes verify that the default UI is truthful and debug details remain inspector-only

### Requirement: Surface adapters are unit-testable with fakes
Terminal surface controllers and input/scrollback adapters SHALL be testable
with fake surface handles for state transitions and event translation that do
not require a live Ghostty renderer.

#### Scenario: Fake scroll metrics
- **WHEN** a fake surface publishes scrollback metrics
- **THEN** adapter tests verify native scrollbar range and visible viewport updates

#### Scenario: Fake input events
- **WHEN** adapter tests send keyboard, mouse, paste, and search commands through fake events
- **THEN** the fake surface receives normalized terminal operations or command-routing decisions

### Requirement: Runtime service ownership has focused tests
The Apple client SHALL include focused tests for process bootstrap, window
runtime service ownership, pane handle creation, reattachment, text delivery,
and teardown using fake Ghostty adapters where possible.

#### Scenario: Fake runtime reattaches view
- **WHEN** a test creates a pane handle, detaches the host view, and attaches a replacement host view
- **THEN** the test verifies that the pane handle identity and runtime metadata remain unchanged

#### Scenario: Fake runtime tears down once
- **WHEN** a test closes a pane, tab, or window through shell actions
- **THEN** the fake runtime observes exactly one teardown call per affected pane

### Requirement: Ghostty bootstrap is testable without launching the full app
The Apple client SHALL expose a bootstrap seam that lets tests verify Ghostty
dependency and initialization behavior without launching the full SwiftUI app or
requiring real terminal rendering.

#### Scenario: Bootstrap dependency missing
- **WHEN** a fake bootstrap reports missing Ghostty resources
- **THEN** tests verify that pane creation enters a non-ready state with an actionable error

#### Scenario: Bootstrap reused
- **WHEN** two window runtime services request terminal support in one test process
- **THEN** tests verify that the process bootstrap is invoked once and both services receive the same bootstrap result

### Requirement: Control-plane runtime tests use the service boundary
Control-plane tests SHALL exercise runtime-dependent mutations through the same
terminal runtime service boundary used by production code.

#### Scenario: Service accepts text
- **WHEN** a control-plane test sends text to a fake live pane runtime
- **THEN** the command response reports accepted bytes from the fake service and shell diagnostics remain clean

#### Scenario: Service reports runtime missing
- **WHEN** a control-plane test sends text to a pane whose service handle is absent
- **THEN** the command response reports a stable runtime-missing error

### Requirement: Terminal event ownership is contract-checked
The Apple client SHALL include focused shell contract checks that preserve the
terminal event ownership boundary between SwiftUI layout, AppKit terminal host
input, rendering canvases, and native window background dragging.

#### Scenario: SwiftUI terminal tap wrapper is reintroduced
- **WHEN** a code change wraps the terminal native view in a SwiftUI tap gesture for pane selection
- **THEN** the shell contract check fails with an error explaining that terminal-area selection belongs to the terminal host

#### Scenario: Activation delegate strongly retains controller state
- **WHEN** a code change stores terminal activation as a strong registry-owned closure
- **THEN** the shell contract check fails or the focused review checklist requires replacing it with the weak activation boundary

#### Scenario: Rendering canvas becomes interactive owner
- **WHEN** a code change lets Ghostty or fallback rendering canvas views receive terminal mouse-down hit tests as independent owners
- **THEN** the shell contract check fails or the focused review checklist requires routing those events through the terminal host

#### Scenario: Focused manual verification is performed
- **WHEN** event ownership implementation is ready for review
- **THEN** verification covers click-to-select, immediate typing, drag selection, right click, scrolling, and background window dragging in the running macOS app

### Requirement: Pane title bars have focused verification
The Apple client SHALL include focused automated or documented verification for
pane title-bar consumption, pane-scoped close routing, and terminal input
ownership when pane title bars are changed.

#### Scenario: Pane title-bar consumption tested
- **WHEN** pane title-bar helpers receive existing terminal title, working-directory, cwd, launch-target, and process metadata combinations
- **THEN** focused tests verify title-bar priority, fallback ordering, long-title handling, and suppression of raw pane IDs or debug terms without retesting terminal title capture itself

#### Scenario: Pane close routing tested
- **WHEN** a title-bar close action targets a selected pane, an inactive split pane, a single-pane tab with other tabs, or the final remaining pane
- **THEN** focused tests verify the shell mutation result, selected pane after close, split tree repair, and final-pane protection

#### Scenario: Terminal input preservation reviewed
- **WHEN** pane title-bar implementation is ready for review
- **THEN** maintainers can inspect automated shell contract checks or manual notes covering terminal click-to-focus, typing, selection drag, right click, scrolling, and close-button interaction

#### Scenario: Window chrome and collapsed sidebar guardrails run
- **WHEN** hidden-titlebar window chrome, titlebar double-click behavior, local app launch behavior, or collapsed-sidebar floating-panel behavior changes
- **THEN** focused checks verify launch presents one primary window, empty titlebar double-click zoom targets only non-control chrome, system traffic-light buttons keep their normal behavior, and collapsed-sidebar reveal uses narrow hover targets with stable workspace geometry

#### Scenario: Visual evidence captured
- **WHEN** pane title-bar UI polish is marked complete
- **THEN** maintainers can inspect a running-app screenshot or manual note showing light-mode single-pane and split-pane tabs with compact pane title bars, readable titles, restrained close buttons, and no default debug labels

### Requirement: Corner-radius conformance is verified
The Apple client SHALL include focused verification for active-shell
corner-radius normalization when default macOS shell chrome is changed.

#### Scenario: Active shell radius check runs
- **WHEN** a change updates active shell visual chrome in `MacShellRootView.swift`, `TerminalPaneView.swift`, or normal-flow `TerminalHostView.swift` fallback surfaces
- **THEN** a focused check or review step verifies that rounded rectangles use the alan shell radius scale and do not introduce large ad hoc radii

#### Scenario: Capsule usage reviewed
- **WHEN** a change adds `Capsule` usage to active default shell chrome
- **THEN** the change documents why the component is a semantic pill or replaces it with a radius-scale rounded rectangle

#### Scenario: Visual comparison captured
- **WHEN** radius normalization implementation is marked complete
- **THEN** maintainers can inspect running-app screenshots or notes for sidebar, terminal, command input, and remaining default-shell overlay states confirming that the UI is smaller-radius, still native, and not visually flat

#### Scenario: Legacy surfaces scoped
- **WHEN** radius inventory finds older or non-primary Apple client surfaces
- **THEN** implementation records whether they are active default shell UI before changing them, instead of silently broadening the polish pass

### Requirement: Find UI has focused verification
The Apple client SHALL include focused automated or documented verification for
the pane-scoped native Find bar and removed inspector surface.

#### Scenario: Find bar behavior verified
- **WHEN** Find behavior changes
- **THEN** focused tests or manual notes cover `Command-F`, query editing, Return, `Command-G`, Shift-`Command-G`, Escape, and printable query text behavior while Find is active

#### Scenario: Inspector removal contract checked
- **WHEN** default shell UI changes
- **THEN** shell contract checks fail if removed inspector UI, commands, voice phrases, or stale inspector affordances are reintroduced

#### Scenario: Passive search overlay blocked
- **WHEN** terminal search UI changes
- **THEN** shell contract checks fail if default Find UI is routed through the passive terminal overlay instead of `ShellFindBarView`

### Requirement: Apple architecture maintainability has focused validation
The Apple client SHALL provide focused validation for source layout,
multi-responsibility hotspots, README/project drift, and SwiftUI/AppKit boundary
regressions when architecture maintainability changes are implemented.

#### Scenario: Architecture report is run
- **WHEN** a developer runs the Apple architecture-maintainability check
- **THEN** the report identifies files or project groups that violate the
  accepted source ownership boundaries and gives actionable paths to the owning
  files or folders

#### Scenario: README and project layout drift
- **WHEN** Apple client source folders or Xcode project groups are reorganized
- **THEN** validation or review confirms `clients/apple/README.md`, source
  paths, and project membership describe the same structure

#### Scenario: AppKit bridge spreads into unrelated SwiftUI
- **WHEN** a change introduces `NSWindow`, `NSView`, `NSApp`, Darwin socket, or
  process-management ownership into a SwiftUI feature view that does not own a
  platform bridge
- **THEN** the architecture check fails or the review checklist requires moving
  the behavior into an app, service, terminal host, or support boundary

#### Scenario: Behavior-preserving move is reviewed
- **WHEN** a refactor slice only moves or extracts Apple client code
- **THEN** verification includes diff review, project membership validation, and
  the focused Apple build or script checks needed to prove behavior was not
  intentionally changed

### Requirement: Streamlined sidebar has focused verification
The Apple client SHALL include focused verification for sidebar information
architecture changes that remove visible copy or restructure space/tab
navigation.

#### Scenario: Sidebar reading order is reviewed
- **WHEN** sidebar IA implementation is marked complete
- **THEN** maintainers can inspect screenshots or manual notes showing the vertical sidebar, active-space tab list, bottom borderless space switcher, separate creation affordances, and no persistent explanatory sidebar blocks

#### Scenario: Sidebar interaction states are reviewed
- **WHEN** tab or space row secondary actions are progressively disclosed
- **THEN** verification covers default, hover, selected, no-notification-dot, and empty states without row resizing or layout shifts
- **AND** verification covers the tab row hierarchy where normal rows are containerless, hover/focus rows are subtle, selected rows are strongest, and creation rows stay muted until interaction
- **AND** verification covers the space-title/tab-list scroll boundary where the divider and shadow appear only as tab rows scroll underneath the fixed space title region

#### Scenario: Sidebar space swipe is reviewed
- **WHEN** horizontal space switching is implemented in the sidebar
- **THEN** verification covers gesture-tracked left and right sidebar previews, translation-first drag mapping, fast horizontal flick commit, full-width space-title and tab-list motion, stationary hold, zero-delta release, horizontal/vertical axis lock, later vertical movement during a horizontal swipe, no static side padding gaps during page motion, a stable workspace during drag, commit, cancel, edge resistance, and confirms vertical tab-list scrolling still works

#### Scenario: Split tab indicator is reviewed
- **WHEN** split-aware tab row implementation is marked complete
- **THEN** verification covers single-pane, two-pane horizontal, two-pane vertical, complex split, focused-pane, no-notification-dot, pointer activation, and keyboard or accessibility activation states

#### Scenario: Accessibility copy is preserved
- **WHEN** visible sidebar text is removed or shortened
- **THEN** review confirms accessibility labels, help text, menu labels, or equivalent nonvisual descriptions still identify the affected controls

### Requirement: Command input polish has focused verification
The Apple client SHALL include focused verification for the `Command-P` input
surface when command UI behavior or material treatment changes.

#### Scenario: Command input keyboard flow is verified
- **WHEN** command input implementation is marked complete
- **THEN** focused tests or manual notes cover open/focus, typing, successful Return submission, unresolved Return behavior, Escape dismissal, click-away dismissal, and terminal focus restoration

#### Scenario: Candidate sections stay removed
- **WHEN** default command input UI changes
- **THEN** shell contract checks or review notes confirm action, routing, attention, best-match, command-row, and microphone affordances are not visible in the default command input surface

#### Scenario: Liquid input visual review is captured
- **WHEN** command input material polish is marked complete
- **THEN** maintainers can inspect screenshots or manual notes showing the input over the active light-mode shell with legible text, restrained depth, and no large panel below the field

### Requirement: Material hierarchy has focused verification
The Apple client SHALL include focused verification for active-shell material
changes so native material polish does not reduce terminal readability or
reintroduce hard-coded visual effects.

#### Scenario: Material review is captured
- **WHEN** active macOS shell material roles, background surfaces, or compact control treatments change
- **THEN** maintainers can inspect screenshots or manual notes covering the default light-mode sidebar, terminal content area, command entry, compact controls, and floating overlays

#### Scenario: Accessibility material settings are reviewed
- **WHEN** material hierarchy implementation is marked complete
- **THEN** verification includes reduced-transparency or increased-contrast review notes, or a documented reason those settings could not be exercised locally

#### Scenario: One-off material fills are checked
- **WHEN** a change adds new active-shell material or translucent fills
- **THEN** focused review or a lightweight check confirms the fill is attached to a shared semantic material/control role rather than a local hard-coded effect

#### Scenario: Elevation hierarchy is reviewed
- **WHEN** active macOS shell radius, shadow, rim, or floating-surface treatment changes
- **THEN** focused review confirms terminal surface, sidebar selection, titlebar controls, command launcher, Find bar, command input, and collapsed sidebar panel use the shared semantic radius/elevation scale

#### Scenario: Light-mode shadow cleanliness is reviewed
- **WHEN** active shell elevation changes are marked complete
- **THEN** maintainers can inspect screenshots or notes confirming light-mode shadows are focused and adaptive rather than broad, dirty, or purely black halos

### Requirement: Branding and project identity checks run with Apple validation
Apple-client validation SHALL include focused checks that protect the canonical
`alan` product brand, `alan for macOS` platform label, and `alan-macos`
engineering identity.

#### Scenario: Brand scan runs
- **WHEN** Apple-client validation runs for a branding or project rename change
- **THEN** it scans active Apple source, scripts, docs, project metadata, and
  active OpenSpec changes for non-allowlisted `Alan`, `AlanNative`,
  `Alan Shell`, `alanterm`, and `dev.alan.native` occurrences
- **AND** it reports the expected canonical replacement for each violation

#### Scenario: Renamed Xcode build runs
- **WHEN** implementation is ready for review
- **THEN** the documented Xcode build command uses
  `clients/apple/alan-macos.xcodeproj`, scheme `alan-macos`, configuration
  `Debug`, destination `platform=macOS`, and the shared derived-data path
- **AND** the build produces `alan.app`

#### Scenario: Focused scripts are updated
- **WHEN** focused Apple shell scripts are run after the rename
- **THEN** they read source files from `clients/apple/alan-macos`
- **AND** script defaults such as bundle identifiers, capture helpers, and
  architecture checks use the new app identity instead of `AlanNative` or
  `dev.alan.native`
