# Manual Verification

Date: 2026-05-07

Build:

```bash
xcodebuild -project clients/apple/AlanNative.xcodeproj -scheme AlanNative -configuration Debug -destination platform=macOS -derivedDataPath target/xcode-derived build
```

Observed app bundle:

```text
target/xcode-derived/Build/Products/Debug/Alan.app
```

Checks:

- Initial launch opened one primary Alan window with accessibility ID `main`.
- Repeated `open target/xcode-derived/Build/Products/Debug/Alan.app` kept one Alan window.
- Forced `open -n target/xcode-derived/Build/Products/Debug/Alan.app` kept one Alan window and one active owner PID.
- `Command-N` focused the existing Alan window and did not create a second window.
- Closing the Alan window released the singleton lock; reopening created one primary Alan window.
- `Command-Q` released the singleton lock; relaunch created one primary Alan window.

Window evidence was checked with:

```bash
clients/apple/scripts/capture-alan-window.sh --list
```

Representative output:

```text
9053    pid=91102    bundle=dev.alan.native    active=true    frame=120,95,1271,832    Alan
```
