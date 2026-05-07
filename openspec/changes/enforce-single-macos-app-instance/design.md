## Context

The native macOS entry point currently declares `WindowGroup("Alan")`, which is
the SwiftUI scene type for independently creatable windows. Each
`MacShellRootView` initialization creates a fresh `ShellWindowContext`, so a
second main window also creates a second shell `window_id`, control directory,
socket path, state file, and terminal runtime registry.

That multi-window model conflicts with the desired Alan shell shape: one focused
native workspace, with tabs and splits inside the window rather than duplicate
app windows outside it. Normal Finder/Dock launch usually reuses the running
bundle, but `open -n` and direct executable launches can still create another
process unless Alan enforces its own startup guard.

## Goals / Non-Goals

**Goals:**

- Guarantee at most one running Alan macOS app instance per user session and
  bundle identifier.
- Guarantee at most one primary Alan shell window in that app instance.
- Focus or reopen the existing primary window for repeated launch, Dock reopen,
  activation, and New Window command paths.
- Prevent a forced second process from creating SwiftUI scenes, shell control
  sockets, persisted shell state, or terminal runtimes.
- Keep iOS scene behavior unchanged.
- Add focused automated and manual verification for process and window
  singleton behavior.

**Non-Goals:**

- Remove tabs, spaces, splits, panes, or terminal runtime ownership inside the
  primary shell window.
- Define multiple independent Alan workspaces as separate windows.
- Enforce a singleton for the Rust daemon, CLI, TUI, or helper tools.
- Add file-opening, URL-opening, or deep-link handoff between a blocked second
  process and the running app.
- Rely on packaging-only launch hints as the sole duplicate-process guarantee.

## Decisions

1. Use a single macOS `Window("Alan", id: "main")` scene for the primary shell.

   `Window` models a unique scene identity, while `WindowGroup` models repeated
   window creation. The app should keep the existing hidden-titlebar and default
   size styling on the single scene, but new shell windows should no longer be
   creatable through SwiftUI scene duplication.

   Alternative considered: keep `WindowGroup` and count `NSWindow` instances at
   runtime. That still lets SwiftUI create root views and shell contexts before
   the app can reject duplicates.

2. Move primary shell ownership to an app-level macOS controller.

   The macOS app should create one `ShellWindowContext` and one
   `ShellHostController` at process scope, then inject or reference that owner
   from `MacShellRootView`. Reopening the primary scene must reuse that owner
   instead of letting root-view initialization allocate a fresh shell context.

   Alternative considered: let `MacShellRootView` keep creating fresh contexts
   and depend on the single scene to hide the issue. That does not cover
   close/reopen or future scene lifecycle changes.

3. Replace the New Window command group for macOS.

   The app should remove or replace the standard New Window command and
   `Command-N` path. If a command remains visible, it must be a focus/reopen
   command for the existing primary window, not a duplicate-window creator.

   Alternative considered: leave the default menu command in place because a
   `Window` scene is unique. That creates ambiguous UI and makes regressions
   harder to notice.

4. Add an early macOS startup singleton guard.

   A macOS-only app delegate or launch controller should acquire an advisory
   lock under the user's Application Support directory before scenes and shell
   runtime state are created. The lock must be held for the process lifetime. A
   second process that cannot acquire the lock must activate the existing app
   with the same bundle identifier and then terminate.

   The lock implementation should use OS-backed exclusive locking such as
   `flock` on an open file descriptor so stale locks are released when a process
   exits or crashes. A PID file can be diagnostic metadata, but it must not be
   the authority for ownership.

   Alternative considered: rely on LaunchServices or Info.plist keys such as
   multiple-instance prohibition. Those can help normal launch paths but do not
   fully cover forced launch or direct executable starts.

5. Keep shell state singleton-aware but not window-count dependent.

   Existing control-plane behavior must stop treating a second macOS window as
   a supported way to obtain an isolated shell identity. The primary app
   instance owns one active shell window context; duplicate UI/process requests
   must focus that context and must not create competing sockets or state files.

   Alternative considered: allow hidden internal shell windows while presenting
   one visible window. That would preserve old isolation semantics but violate
   the user's one-window expectation and create confusing agent control targets.

## Risks / Trade-offs

- Lock acquisition happens too late -> Acquire the singleton lock in the
  macOS launch delegate before SwiftUI scene/root view construction can create
  shell state.
- App activation cannot find the existing process -> Use the bundle identifier
  as the primary lookup and terminate the duplicate process even if activation
  fails, to avoid two Alan instances.
- Closing the primary window tears down AppKit terminal views -> Preserve the
  app-level shell owner and ensure reopen creates at most one primary window;
  terminal surface reconnection or process-exit behavior must be truthful.
- Generated Info.plist settings differ between Debug and Release -> Keep the
  lock authoritative and treat Info.plist launch hints as optional support only.
- Automated UI process-count checks can be flaky in CI -> Cover pure lock
  behavior with focused Swift tests and keep app-level launch checks as local
  scripts or manual verification when CI cannot run foreground macOS apps.

## Migration Plan

1. Add the macOS singleton guard and focused lock tests without changing UI.
2. Replace the macOS `WindowGroup` with a unique primary `Window` scene.
3. Move shell context/host ownership to an app-level controller and inject it
   into the root view.
4. Replace New Window command behavior and wire reopen/activation to the primary
   scene.
5. Update README and control-plane wording that currently describes each macOS
   window as an independent shell context.
6. Add verification scripts or notes for repeated launch, forced launch,
   `Command-N`, Dock reopen, close/reopen, and `Command-Q`.

## Open Questions

- Should closing the primary window keep terminal child processes alive until
  `Command-Q`, or should close behave as a full shell-window shutdown?
- Should a future workspace switcher use separate persisted workspaces inside
  the single window, or is workspace selection outside this native app for now?
