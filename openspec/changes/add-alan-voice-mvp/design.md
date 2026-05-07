## Context

Alan macOS is becoming a terminal-first native app where Alan is available as a
quiet capability layered onto the user's work surface. Voice input should match
that direction: fast, keyboard-driven, low-distraction, and intent-oriented. It
should not feel like a separate dictation tool, a Siri-style assistant, a voice
call surface, or an always-listening background agent.

The current Apple client has a small `ShellVoiceCommandController` based on
`NSSpeechRecognizer` and a fixed list of shell commands. That controller is not
the right foundation for Alan Voice because it recognizes command vocabulary
rather than free-form user intent, and it creates a parallel voice-control path
that would drift from Alan's main runtime and context model.

Apple's Speech framework is the best default local foundation. Apple documents
`SFSpeechRecognizer` as the object that initiates recognition, supports live
audio requests through `SFSpeechAudioBufferRecognitionRequest`, can report
partial results, and exposes on-device capability through
`supportsOnDeviceRecognition` plus request-level `requiresOnDeviceRecognition`.
However, Apple also documents that some languages require network availability,
so Alan must treat on-device recognition as a preferred local capability rather
than an unconditional guarantee for every locale and device.

Feature brand decision: use **Alan Voice** as the capability brand and **Hold
to Talk** as the first interaction name. Avoid "Alan Dictation" because this is
not raw text insertion, avoid "Alan Whisper" because it collides with OpenAI's
model brand, and avoid "Alan Listen" because it implies always-on listening.

## Goals / Non-Goals

**Goals:**

- Make voice input available through a press-and-hold shortcut, with release to
  stop and Escape to cancel.
- Convert speech into typed intents such as capture, agent command, task
  creation, search, and app command.
- Default to Local Mode with no Alan-managed model download, no account
  requirement, and no audio upload when local recognition is selected.
- Offer Cloud Mode as an explicit opt-in for higher recognition quality and
  richer speech-to-intent behavior, with visible provider and mode state.
- Keep UI feedback compact and native: recording, processing, success, failure,
  and cancellation should be visible without taking over the terminal.
- Retire the legacy fixed-command voice controller so there is one voice input
  system and one intent-routing contract.
- Preserve startup performance by lazy-loading voice recognition and provider
  clients only when the user first invokes or configures Alan Voice.

**Non-Goals:**

- Always listening, wake-word detection, continuous dictation, meeting
  transcription, long-form audio recording, voice chat, TTS, voice calls, or a
  Siri-style assistant.
- Guaranteeing offline recognition for every language/locale/device when the
  platform recognizer does not support it.
- Replacing the normal Alan text composer or terminal command surfaces.
- Building full voice navigation, streaming transcript display, voice memory,
  or multilingual optimization beyond the first-phase intent pipeline.

## Decisions

1. Brand the feature as Alan Voice and the MVP interaction as Hold to Talk.

   Alan Voice is broad enough to cover future voice capabilities while the MVP
   remains constrained by the Hold to Talk interaction. The UI should use
   action language such as "Hold to Talk", "Release to send", and "Esc to
   cancel" rather than surfacing implementation labels like speech recognizer,
   transcript, or provider pipeline.

   Alternatives considered: "Alan Dictation" over-emphasizes raw transcript
   insertion, "Alan Whisper" conflicts with OpenAI's model name, and "Alan
   Listen" suggests always-on listening.

2. Replace the legacy fixed-command voice path instead of extending it.

   The existing `NSSpeechRecognizer` command list can recognize only predefined
   phrases. Alan Voice needs free-form audio capture, transcription, intent
   classification, and routing through the same session/runtime paths as typed
   Alan input. Keeping both paths would create duplicate command ownership and
   inconsistent behavior.

   Alternative considered: keep the legacy controller for shell-only commands.
   That would make voice behavior depend on whether the utterance is a shell
   command or an Alan intent, which is exactly the distinction the product
   should hide.

3. Separate audio recognition mode from intent execution.

   Local Mode controls where audio recognition happens. It must not upload
   audio, and it should use Apple on-device recognition when the selected locale
   supports it. Intent execution is a separate Alan decision: simple intents
   can be routed deterministically, while agent commands may still become normal
   Alan turns that use the current model/provider profile. The UI must disclose
   cloud recognition separately from any normal Alan runtime/model execution.

   Alternative considered: treat Local Mode as fully offline intent execution.
   That would overpromise because many Alan intents naturally invoke the
   configured agent runtime and model.

4. Use a typed `VoiceIntent` boundary.

   The recognizer output should feed an intent resolver that emits a typed
   result with at least: raw transcript, normalized text, intent type, target
   context, confidence, safety level, and proposed action. First-phase intent
   types are capture, agent command, task creation, search, and app command.
   Ambiguous or state-changing actions must be reviewable or cancellable before
   they advance runtime state.

   Alternative considered: send every transcript directly as a user turn. That
   is simpler but fails the core Speech-to-Intent product goal.

5. Keep the capture layer compact and keyboard-first.

   Alan Voice should behave like a quick capture layer over the active Alan
   surface. It should show a small status surface with recording and processing
   states, not a full modal editor. The mouse should not be required for the
   primary path. Cancellation must leave no partial note, draft, or task.

   Alternative considered: reuse the main composer as the primary feedback
   surface. That makes voice feel like dictation into a text box rather than
   intent input.

6. Store Cloud Mode credentials in a host secret store.

   Cloud recognition provider keys must not live in workspace files or
   browser-side state. The macOS app should use Keychain-backed storage or
   Alan's existing connection/secret-store patterns, and settings must make the
   current provider and mode visible.

   Alternative considered: put voice provider settings in `agent.toml`. That
   mixes host/device input configuration with agent-facing runtime config and
   risks exposing secrets in workspace context.

## Risks / Trade-offs

- Local recognition may be unavailable for a user's language or device -> Show
  a clear Local Mode unavailable state and never silently switch to cloud audio
  upload.
- Chinese and mixed Chinese/English quality may be weak in Local Mode -> Make
  Cloud Mode opt-in and visible, and allow provider switching.
- Intent classification may mis-route a command -> Require confirmation for
  destructive or ambiguous actions and keep a visible undo/cancel path before
  runtime state advances.
- Global shortcuts may conflict with terminal/system shortcuts -> Make the
  shortcut configurable, detect conflicts where possible, and keep a menu item
  fallback for first use.
- Voice components may slow app startup -> Lazy initialize audio, speech, and
  cloud provider clients only when Alan Voice is invoked or opened in settings.
- Cloud provider setup may become too complex -> Keep Local Mode default and
  make Cloud Mode optional, with provider status summarized in one settings
  surface.
- Users may confuse Alan Voice with always listening -> Use Hold to Talk
  language, show explicit recording state, and do not run background capture
  outside the held shortcut.

## Migration Plan

1. Introduce the Alan Voice state model, settings model, and typed
   `VoiceIntent` contract without changing existing shell behavior.
2. Remove or disable the legacy `ShellVoiceCommandController` fixed-command
   path and replace any visible voice affordance with Alan Voice entry points.
3. Add push-to-talk shortcut handling, recording lifecycle, cancellation, and
   compact feedback UI behind a feature flag or development setting.
4. Add Local Mode using platform speech recognition with strict no-audio-upload
   behavior when local recognition is selected.
5. Add Cloud Mode provider configuration and secure credential storage.
6. Add the speech-to-intent resolver and route first-phase intents to current
   Alan surfaces.
7. Add focused Apple-client tests and scripted checks for permissions, shortcut
   state transitions, provider switching, cancellation, and intent routing.
8. Rollback by disabling Alan Voice entry points; the old fixed-command voice
   controller should not be restored unless explicitly needed for development.

## Open Questions

- Which default Hold to Talk shortcut should ship without conflicting with the
  current terminal command set?
- Which Cloud Mode provider should be first-class in the MVP, and should it
  reuse existing Alan connection profiles or have a voice-specific provider
  settings surface?
- Should low-confidence non-destructive capture intents write immediately with
  a visible "undo" affordance, or always require review?
- What is the first persistent Task surface that Alan Voice should create tasks
  into if the macOS app does not yet have a mature native task model?

## References

- Apple Developer Documentation: `SFSpeechRecognizer`
  <https://developer.apple.com/documentation/Speech/SFSpeechRecognizer>
- Apple Developer Documentation: `supportsOnDeviceRecognition`
  <https://developer.apple.com/documentation/Speech/SFSpeechRecognizer/supportsOnDeviceRecognition>
- Apple Developer Documentation: `requiresOnDeviceRecognition`
  <https://developer.apple.com/documentation/speech/sfspeechrecognitionrequest/requiresondevicerecognition>
- Apple Developer Documentation: asking permission to use speech recognition
  <https://developer.apple.com/documentation/speech/asking-permission-to-use-speech-recognition>
