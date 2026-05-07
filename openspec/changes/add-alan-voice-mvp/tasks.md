## 1. Product Boundary And Legacy Cleanup

- [ ] 1.1 Confirm Alan Voice / Hold to Talk naming in user-facing strings,
  settings, menu items, and docs.
- [ ] 1.2 Remove or disable the legacy `ShellVoiceCommandController`
  fixed-command voice path in `MacShellRootView.swift`.
- [ ] 1.3 Remove user-facing affordances that expose the old fixed command
  vocabulary as a parallel voice feature.
- [ ] 1.4 Document which legacy shell command phrases are expected to route
  through Alan Voice app-command intents.

## 2. Voice State And Settings Model

- [ ] 2.1 Add an Alan Voice state model covering idle, recording, processing,
  review, success, cancelled, unavailable, and error states.
- [ ] 2.2 Add a typed `VoiceIntent` model with transcript, normalized text,
  intent type, target context, confidence, safety level, and proposed action.
- [ ] 2.3 Add settings for Hold to Talk shortcut, default recognition mode,
  language/locale, Cloud Mode provider, credential status, and mode visibility.
- [ ] 2.4 Store voice provider credentials in Keychain or Alan's host secret
  store rather than workspace files.

## 3. Hold To Talk Capture Lifecycle

- [ ] 3.1 Add configurable Hold to Talk shortcut handling for press, hold,
  release, and cancel transitions.
- [ ] 3.2 Ensure shortcut handling does not conflict with existing terminal
  command ownership or native menu shortcuts.
- [ ] 3.3 Implement microphone capture lifecycle with no overlapping capture
  sessions.
- [ ] 3.4 Implement Escape cancellation that prevents transcripts, tasks,
  commands, and agent submissions from being created.

## 4. Local And Cloud Recognition Providers

- [ ] 4.1 Add Local Mode using Apple Speech framework recognition with
  `requiresOnDeviceRecognition` when platform support is available.
- [ ] 4.2 Detect and report Local Mode unavailability without silently switching
  to Cloud Mode or uploading audio.
- [ ] 4.3 Add Cloud Mode provider abstraction for optional higher-quality speech
  recognition.
- [ ] 4.4 Add Cloud Mode credential validation, provider status, and user-visible
  audio upload disclosure.
- [ ] 4.5 Lazy initialize audio, speech, and cloud provider clients outside the
  app startup critical path.

## 5. Speech-To-Intent Routing

- [ ] 5.1 Implement first-pass intent resolution for capture, agent command,
  task creation, search, and app command intents.
- [ ] 5.2 Include active Alan context, terminal pane/session state, and current
  surface metadata in intent resolution.
- [ ] 5.3 Route capture intents to the current context or a safe capture fallback.
- [ ] 5.4 Route agent command intents through the normal Alan runtime/session
  submission path.
- [ ] 5.5 Route task intents to the current task surface or a compatible fallback
  capture surface.
- [ ] 5.6 Route search and app command intents through their owning app surfaces
  without starting unrelated agent execution.
- [ ] 5.7 Require review or confirmation for ambiguous, destructive, or
  low-confidence state-changing intents.

## 6. Voice Capture UI And Permissions

- [ ] 6.1 Add a compact Alan Voice capture layer for recording, processing,
  review, success, cancellation, unavailable, and error states.
- [ ] 6.2 Keep the active terminal or Alan content visible while the capture
  layer is shown.
- [ ] 6.3 Make the primary recording and cancellation flow fully keyboard
  accessible.
- [ ] 6.4 Add microphone, speech recognition, shortcut, and accessibility
  permission prompts with purpose-specific repair instructions.
- [ ] 6.5 Show current recognition mode and cloud provider state in Alan Voice
  settings and capture feedback when relevant.

## 7. Verification

- [ ] 7.1 Add tests for Alan Voice state transitions: idle, recording,
  processing, cancel, success, unavailable, and error.
- [ ] 7.2 Add tests for Local Mode no-upload behavior and unavailable fallback.
- [ ] 7.3 Add tests for Cloud Mode provider selection, missing credentials, and
  mode/provider disclosure.
- [ ] 7.4 Add tests for first-phase intent routing and low-confidence review
  behavior.
- [ ] 7.5 Add tests or scripted checks proving the legacy fixed-command voice
  controller is no longer exposed as a parallel user-facing path.
- [ ] 7.6 Run focused Apple client build/tests for changed macOS voice surfaces.

## 8. Documentation And OpenSpec Closure

- [ ] 8.1 Update product and maintainer docs to describe Alan Voice as
  push-to-talk speech-to-intent, not dictation or always listening.
- [ ] 8.2 Document Local Mode and Cloud Mode privacy/provider behavior.
- [ ] 8.3 Run `openspec validate add-alan-voice-mvp --type change --strict --json`.
- [ ] 8.4 Run `openspec validate --all --strict --json`.
- [ ] 8.5 Run `git diff --check`.
- [ ] 8.6 Before archive, sync accepted delta requirements into `openspec/specs/`.
- [ ] 8.7 Archive the OpenSpec change after implementation is merged.
