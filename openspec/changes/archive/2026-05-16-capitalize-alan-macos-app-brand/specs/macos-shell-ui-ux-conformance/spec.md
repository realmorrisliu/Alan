## MODIFIED Requirements

### Requirement: Visible macOS app copy follows product brand identity
The default macOS app UI SHALL render the public product brand as `Alan` and
SHALL use `Alan for macOS` only where platform distinction is useful.

#### Scenario: App chrome is visible
- **WHEN** the Dock name, app menu, window title, toolbar labels, command
  palette labels, sidebar buttons, help text, or accessibility labels name the
  product
- **THEN** they use `Alan`
- **AND** they do not use lowercase `alan`, `AlanNative`, `alanterm`, or
  `Alan Shell` as visible product names

#### Scenario: Terminal app category is visible
- **WHEN** the UI or docs explain the native app's category
- **THEN** they call it a terminal emulator or terminal workspace
- **AND** they do not call the product a shell
