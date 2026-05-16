## Why

alan currently has a split local installation story: `just install` installs the
release CLI and TUI, while `just app` builds and force-restarts a debug
`Alan.app`. This does not match the intended user-facing distribution model,
where `Alan.app` is the primary signed artifact and the CLI/TUI are installed
from that same release package.

## What Changes

- Introduce a signed release packaging contract where `Alan.app` is the primary
  distribution artifact and embeds release `alan` and `alan-tui` executables.
- Require Developer ID signing for the app bundle and nested CLI/TUI binaries;
  local ad-hoc signing is not an accepted distribution path for this change.
- Define the future Homebrew cask shape: install `Alan.app` and expose the
  embedded `alan` and `alan-tui` binaries through Homebrew's `bin` directory.
- Change the local developer install flow so `just install` builds and installs
  the release app and PATH-visible CLI/TUI symlinks without killing or launching
  the running app.
- Remove `~/.alan/bin` as a CLI/TUI install location; it is no longer a
  supported distribution, fallback, local install target, or documented PATH
  setup.
- **BREAKING**: remove `just app` entirely and do not add a replacement
  debug-run recipe.

## Capabilities

### New Capabilities

- `alan-app-distribution`: defines the signed app-first distribution model,
  embedded CLI/TUI layout, Homebrew cask expectations, direct app command-line
  tool installation, and local release installation behavior.

### Modified Capabilities

- `macos-shell-build-test-contract`: replace the debug app runner as the
  documented local app workflow with release install validation, and require
  focused checks that prevent reintroducing `just app`.

## Impact

- Release and local install scripts:
  - `justfile`
  - `scripts/install.sh`
  - new or updated release assembly/signing scripts under `scripts/` or
    `clients/apple/scripts/`
- Apple build outputs and signing:
  - `clients/apple/alan-macos.xcodeproj`
  - `target/xcode-derived/Build/Products/Release/Alan.app`
  - `Alan.app/Contents/Resources/bin/alan`
  - `Alan.app/Contents/Resources/bin/alan-tui`
- Homebrew distribution metadata for the future `alan` cask.
- Documentation and focused Apple validation scripts that currently assume
  `just app` or `clients/apple/scripts/run-alan-debug-app.sh` is the local app
  launch path.
