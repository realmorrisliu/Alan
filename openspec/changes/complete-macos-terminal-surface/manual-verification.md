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

## Automated Coverage Used With Manual Notes

- `clients/apple/scripts/test-terminal-surface-controller.sh` covers scrollback metrics, alternate-screen and mouse-reporting scroll routing, key command routing, pointer mode routing, Ghostty-compatible other-button mapping, pressure events, unready pointer suppression, search actions, selection copy, paste delivery, and overlay projection.
- `clients/apple/scripts/test-terminal-runtime-service.sh` covers runtime attachment, teardown, text delivery, and fallback delivery states with fake terminal surfaces.
- `clients/apple/scripts/test-shell-runtime-metadata.sh` covers renderer health, input readiness, process exit, terminal mode, cwd/title, and attention projection into pane metadata.
- `bash clients/apple/scripts/check-shell-contracts.sh` guards controller ownership, fake-surface tests, pointer adapter coverage, passive overlays, non-draggable terminal hosts, and metadata projection contracts.

## Notes And Limits

- The ScreenCaptureKit helper `clients/apple/scripts/capture-alan-window.sh` listed the live Alan window but hung while capturing a screenshot in this Codex-controlled session; the hung helper process was terminated. Verification therefore uses Computer Use screenshots/state plus focused automated tests instead of a committed screenshot artifact.
- Dedicated bracketed paste, live secure-input callbacks, and live URL-hover callbacks remain unsupported until Ghostty exposes the needed surface APIs. They remain in the in-flight change as future parity work and are intentionally omitted from the accepted specs.
