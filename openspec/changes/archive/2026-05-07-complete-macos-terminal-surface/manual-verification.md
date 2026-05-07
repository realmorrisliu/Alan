# Manual Verification Notes

Date: 2026-05-06
Build: Debug `Alan.app` from `target/xcode-derived/Build/Products/Debug/Alan.app`
Window observed: `Alan`, bundle `dev.alan.native`, pid `17907`

## Live App Smoke

- Launched the Debug app and confirmed the default window opens to the terminal-first shell surface with one terminal tab.
- Confirmed a real fish shell prompt appears in the terminal pane.
- Entered `echo alan-visible` and sent carriage return with `Control-M`; the terminal printed `alan-visible` and returned to the prompt.
- Entered `seq 1 80`; the terminal printed rows `1...80`, the native scrollbar appeared, and the accessibility tree exposed a settable scroll bar for the terminal scroll area.
- Scrolled the terminal scroll area up and down. The visible buffer changed from rows `41...80` to rows `20...62` and back to rows `41...80`, confirming normal-buffer scrollback synchronization.
- Invoked `Command-F`; the terminal showed the compact `Search terminal` overlay for the focused pane without resizing the sidebar or replacing the terminal workflow.
- Pressed `Escape`; the search overlay dismissed and focus returned to the terminal pane.
- Opened `less /etc/hosts`; the terminal entered a terminal-application screen, showed `/etc/hosts`, and the tab row projected terminal attention status. Pressing `q` exited the terminal application and returned to the shell prompt.
- Right-clicked in the terminal surface during the smoke. The terminal pane remained stable and did not start dragging the window background.

## 2026-05-07 Closeout Pass

- Re-ran the focused Apple terminal scripts:
  `bash clients/apple/scripts/test-terminal-surface-controller.sh`,
  `bash clients/apple/scripts/test-terminal-runtime-service.sh`,
  `bash clients/apple/scripts/test-shell-runtime-metadata.sh`, and
  `bash clients/apple/scripts/check-shell-contracts.sh`.
- Rebuilt the app with
  `xcodebuild -project clients/apple/AlanNative.xcodeproj -scheme AlanNative -configuration Debug -destination platform=macOS -derivedDataPath target/xcode-derived build`.
- Launched the rebuilt Debug app and observed the live window as bundle
  `dev.alan.native`, pid `93270`.
- Entered `echo ALAN_COPY_SENTINEL`; the terminal printed the sentinel and
  returned to the prompt.
- Drag-selected the printed `ALAN_COPY_SENTINEL` output line in the live
  terminal. The selected range highlighted in the terminal canvas, confirming
  terminal-host drag selection rather than window dragging.
- The Codex-controlled environment did not read or overwrite the system
  pasteboard while verifying this pass, so live pasteboard contents were not
  inspected. Copy/paste delivery is covered by
  `verifiesSelectionCopyAndPasteUseController()` and
  `verifiesClipboardDeliveryStates()` with fake pasteboard and fake surface
  handles.
- Direct Computer Use text injection could not drive macOS marked-text/preedit
  composition. IME/preedit ownership is therefore verified by the AppKit
  `NSTextInputClient` path in `TerminalHostView.swift`, the surface-controller
  input contract checks, and the successful live printable-input smoke.
- Running `exit` in the default live shell profile returned the pane to a new
  login shell during this session rather than leaving a reproducible child-exit
  overlay on screen. Child-exit overlay language, metadata projection, and
  control-plane non-delivery are covered by
  `verifiesMetadataOverlayProjection()` and
  `test-shell-runtime-metadata.swift`.
- Renderer/fallback overlay behavior was not forced in the live app because the
  debug build had a healthy linked Ghostty renderer. Renderer-failure state,
  overlay language, debug detail separation, and metadata projection are
  covered by `verifiesMetadataOverlayProjection()` and
  `test-shell-runtime-metadata.swift`.

## Automated Coverage Used With Manual Notes

- `clients/apple/scripts/test-terminal-surface-controller.sh` covers scrollback metrics, alternate-screen and mouse-reporting scroll routing, key command routing, pointer mode routing, Ghostty-compatible other-button mapping, pressure events, unready pointer suppression, search actions, selection copy, paste delivery, and overlay projection.
- `clients/apple/scripts/test-terminal-runtime-service.sh` covers runtime attachment, teardown, text delivery, and fallback delivery states with fake terminal surfaces.
- `clients/apple/scripts/test-shell-runtime-metadata.sh` covers renderer health, input readiness, process exit, terminal mode, cwd/title, and attention projection into pane metadata.
- `bash clients/apple/scripts/check-shell-contracts.sh` guards controller ownership, fake-surface tests, pointer adapter coverage, passive overlays, non-draggable terminal hosts, and metadata projection contracts.

## Notes And Limits

- The ScreenCaptureKit helper `clients/apple/scripts/capture-alan-window.sh` listed the live Alan window but hung while capturing a screenshot in this Codex-controlled session; the hung helper process was terminated. Verification therefore uses Computer Use screenshots/state plus focused automated tests instead of a committed screenshot artifact.
- Dedicated bracketed paste, live secure-input callbacks, and live URL-hover callbacks remain unsupported until Ghostty exposes the needed surface APIs. They are documented as unsupported parity gaps and are intentionally omitted from the accepted specs.
