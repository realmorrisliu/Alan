## ADDED Requirements

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
