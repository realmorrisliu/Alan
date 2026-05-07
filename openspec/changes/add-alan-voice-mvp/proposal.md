## Why

Alan macOS needs a native, low-friction way for users to speak intent into the
app without turning Alan into always-on dictation or a voice assistant. The MVP
should make hold-to-talk voice input fast enough for capture, commands, tasks,
search, and agent starts while preserving Alan's terminal-first, low-distraction
macOS experience.

## What Changes

- Add **Alan Voice** as the feature brand for macOS voice input, with **Hold to
  Talk** as the first interaction model.
- Add a push-to-talk flow where pressing and holding a configurable shortcut
  starts recording, releasing the shortcut stops recording, and Escape cancels
  the current capture without writing anything.
- Add Local Mode for default, no-extra-install speech recognition using
  platform speech capabilities where available, with no audio upload in local
  recognition mode.
- Add Cloud Mode as an optional higher-quality recognition path for Chinese,
  mixed Chinese/English, long utterances, colloquial phrasing, and richer
  speech-to-intent handling.
- Add a speech-to-intent pipeline that turns recognized speech into typed Alan
  intents instead of inserting only raw transcripts.
- Retire the existing `NSSpeechRecognizer` fixed-command shell voice control so
  voice input is owned by Alan Voice rather than a parallel command vocabulary.
- Add a quiet, keyboard-first capture layer that reports recording,
  transcribing, intent resolution, success, cancellation, and recoverable
  errors without taking over the main terminal surface.
- Add permission, privacy, provider, and settings requirements for microphone,
  speech recognition, global shortcut capture, cloud provider selection, and
  mode visibility.

## Capabilities

### New Capabilities

- `alan-voice-input`: Defines Alan Voice on macOS, including hold-to-talk
  interaction, local/cloud speech recognition modes, speech-to-intent routing,
  low-distraction UI feedback, cancellation, permissions, privacy, and first
  phase intent types.

### Modified Capabilities

None.

## Impact

- Apple client: macOS app commands, global shortcut handling, microphone/audio
  capture, Speech framework integration, optional cloud speech provider
  clients, settings surfaces, the voice capture overlay, and removal of the old
  shell voice command controller.
- Runtime/client bridge: a typed voice-intent model and routing layer that can
  submit Alan turns, create tasks/captures, run search, or invoke local commands
  against the current app context.
- Configuration and credentials: local mode defaults, cloud mode provider
  selection, API-key storage, and clear mode/provider disclosure.
- Privacy and permissions: microphone and speech recognition permissions,
  optional accessibility/global-shortcut requirements, no silent cloud upload,
  and recovery instructions for revoked permissions.
- Tests/docs: focused Apple client tests or scripted checks for shortcut state,
  cancellation, permission states, provider mode switching, intent routing, and
  low-distraction UI behavior.
