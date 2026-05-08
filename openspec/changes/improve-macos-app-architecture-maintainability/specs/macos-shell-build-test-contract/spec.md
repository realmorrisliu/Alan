## ADDED Requirements

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
