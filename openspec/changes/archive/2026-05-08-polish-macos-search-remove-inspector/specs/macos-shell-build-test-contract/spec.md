## MODIFIED Requirements

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

#### Scenario: Search verification
- **WHEN** terminal search UI, routing, or adapter behavior changes
- **THEN** tests or manual notes verify `Command-F`, typed query editing, next/previous navigation, no-result feedback, Escape dismissal, and return of focus to the owning terminal pane

#### Scenario: Failure-state verification
- **WHEN** renderer health, child-exit, or fallback UI changes
- **THEN** tests or manual notes verify that the default UI is truthful and raw diagnostics remain restricted to explicit developer debug surfaces

## ADDED Requirements

### Requirement: UI polish verifies inspector removal
The Apple client SHALL include focused checks or review evidence that inspector
UI and command affordances are absent from the default macOS shell.

#### Scenario: Inspector controls removed
- **WHEN** macOS shell UI polish is ready for review
- **THEN** checks or review notes verify that the sidebar, toolbar, command UI, speech vocabulary, and app storage no longer expose an inspector toggle or inspector pane

#### Scenario: Inspector screenshots retired
- **WHEN** UI smoke or screenshot documentation is updated
- **THEN** it verifies the default light-mode window without inspector and does not require inspector overview or debug screenshots

### Requirement: Find bar has focused verification
The Apple client SHALL verify the polished terminal Find bar through automated
adapter tests where practical and manual or screenshot evidence for visual
behavior.

#### Scenario: Find state test
- **WHEN** a focused test starts search, edits a query, receives match updates, navigates, and dismisses search
- **THEN** it verifies the fake search engine receives start, query, navigation, and end actions in order for the owning pane

#### Scenario: Find visual review
- **WHEN** the running app is inspected in light mode
- **THEN** maintainers can confirm the Find bar is compact, pane scoped, has a real text field, shows match feedback, and does not resize the sidebar, toolbar, split layout, or terminal canvas
