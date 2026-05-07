## ADDED Requirements

### Requirement: Alan Voice brand and scope
Alan SHALL present macOS voice input as Alan Voice, with Hold to Talk as the
first-phase interaction model.

#### Scenario: User sees voice input entry points
- **WHEN** Alan presents voice input in menus, settings, shortcut labels, or the
  capture layer
- **THEN** the user-facing language uses Alan Voice and Hold to Talk
- **AND** the default language does not describe the feature as dictation,
  always listening, a voice call, or a Siri-style assistant

#### Scenario: Developer views diagnostics
- **WHEN** a developer opens an explicit diagnostics surface
- **THEN** Alan may show technical labels such as recognizer, provider, audio
  route, transcript, or intent confidence
- **AND** those labels remain outside the default user workflow

### Requirement: Push-to-talk recording lifecycle
Alan SHALL start recording when the user presses and holds the configured Hold
to Talk shortcut and SHALL stop recording when the user releases it.

#### Scenario: User holds shortcut
- **WHEN** the user presses and holds the configured Hold to Talk shortcut
- **THEN** Alan starts a voice capture session
- **AND** Alan shows an active recording state before or as audio capture begins

#### Scenario: User releases shortcut
- **WHEN** the user releases the Hold to Talk shortcut during an active capture
- **THEN** Alan stops audio capture
- **AND** Alan begins recognition and intent processing for the captured audio

#### Scenario: Shortcut is pressed while processing
- **WHEN** the user presses Hold to Talk while a previous voice input is still
  processing
- **THEN** Alan does not start overlapping audio capture
- **AND** Alan either queues, ignores, or offers to cancel the in-flight input
  according to a visible state transition

### Requirement: Cancellation without side effects
Alan SHALL let the user cancel an in-progress voice input before it writes to
Alan state or advances an agent run.

#### Scenario: User cancels while recording
- **WHEN** the user presses Escape or invokes cancel during recording
- **THEN** Alan stops recording
- **AND** no transcript, note, task, command, or agent submission is created

#### Scenario: User cancels while processing
- **WHEN** the user cancels while recognition or intent resolution is in progress
- **THEN** Alan abandons the result if cancellation is still possible
- **AND** Alan does not apply any state-changing action after cancellation

### Requirement: Local recognition mode
Alan SHALL provide Local Mode as the default voice recognition mode and SHALL
avoid uploading audio when Local Mode is selected.

#### Scenario: Local recognition is available
- **WHEN** the user invokes Hold to Talk in Local Mode and the platform supports
  local recognition for the selected locale
- **THEN** Alan performs audio recognition without uploading audio to a cloud
  speech provider
- **AND** Alan may continue intent routing through normal Alan runtime paths
  only after recognition has produced text or a typed intent

#### Scenario: Local recognition is unavailable
- **WHEN** the user invokes Hold to Talk in Local Mode and local recognition is
  unavailable for the selected locale, device, or OS state
- **THEN** Alan reports that local voice recognition is unavailable
- **AND** Alan does not silently switch to Cloud Mode or upload audio

#### Scenario: First launch before voice use
- **WHEN** Alan starts before the user has invoked or configured Alan Voice
- **THEN** Alan does not require an extra model download, account registration,
  or cloud provider setup for the default voice input path

### Requirement: Cloud recognition mode
Alan SHALL provide Cloud Mode as an optional voice recognition mode for users
who want higher quality recognition or provider-backed speech-to-intent.

#### Scenario: User enables Cloud Mode
- **WHEN** the user enables Cloud Mode
- **THEN** Alan requires an explicit provider selection and credential setup
- **AND** Alan shows that audio may be sent to the selected cloud provider

#### Scenario: Cloud provider is missing credentials
- **WHEN** Cloud Mode is selected but the provider credential is missing,
  expired, or invalid
- **THEN** Alan reports the credential problem before recording or upload
- **AND** Alan offers a path to configure credentials or switch back to Local
  Mode

#### Scenario: User switches recognition mode
- **WHEN** the user changes the default voice recognition mode
- **THEN** subsequent Hold to Talk sessions use the selected mode
- **AND** the current mode remains visible in the Alan Voice settings surface

### Requirement: Speech-to-intent output
Alan SHALL convert recognized speech into a typed intent result rather than
treating speech as only a raw transcript.

#### Scenario: Intent is resolved
- **WHEN** Alan successfully recognizes speech
- **THEN** Alan produces a voice intent that includes the transcript, intent
  type, target context, confidence, and proposed action
- **AND** Alan routes the proposed action according to the resolved intent type

#### Scenario: Intent is ambiguous
- **WHEN** Alan cannot resolve a confident intent from recognized speech
- **THEN** Alan asks for review, creates a non-destructive draft, or routes the
  input as a normal Alan message according to the active context
- **AND** Alan does not perform destructive or irreversible actions without
  additional confirmation

### Requirement: First-phase voice intent types
Alan SHALL support capture, agent command, task creation, search, and app
command intents in the first Alan Voice phase.

#### Scenario: Capture intent
- **WHEN** the user says a phrase such as "record this idea"
- **THEN** Alan creates or inserts a capture item in the current Alan context
- **AND** the captured content is based on the recognized intent payload rather
  than the raw audio

#### Scenario: Agent command intent
- **WHEN** the user asks Alan to analyze, inspect, summarize, or otherwise act
  on the current context
- **THEN** Alan creates or uses the appropriate Alan conversation/session
- **AND** Alan submits the command through the normal Alan runtime path

#### Scenario: Task intent
- **WHEN** the user asks Alan to create a todo or task
- **THEN** Alan creates a task-like item in the current task surface or a
  compatible fallback capture surface
- **AND** Alan preserves the task text and relevant context

#### Scenario: Search intent
- **WHEN** the user asks Alan to search local Alan data, recent conversations,
  or current app content
- **THEN** Alan routes the request to the appropriate search surface
- **AND** Alan shows search progress or results without starting unrelated
  agent execution

#### Scenario: App command intent
- **WHEN** the user asks Alan to open, focus, summarize, or transform the
  current Alan surface
- **THEN** Alan routes the action through the owning app command or runtime path
- **AND** Alan rejects unsupported commands with a recoverable message

### Requirement: Current context targeting
Alan SHALL use the current macOS app context to decide where a voice intent
should be written or routed.

#### Scenario: Active Alan context exists
- **WHEN** the user completes a Hold to Talk capture while an Alan session,
  terminal pane, task surface, or search surface is active
- **THEN** Alan includes that active context in the intent resolution input
- **AND** Alan writes or routes the result to the matching context unless the
  user chooses another target

#### Scenario: No active target is available
- **WHEN** no safe active target exists for the resolved voice intent
- **THEN** Alan creates a safe default capture or asks the user to choose a
  target
- **AND** Alan does not discard the recognized content unless the user cancels

### Requirement: Low-distraction capture feedback
Alan SHALL provide compact voice feedback states without taking over the main
terminal or Alan surface.

#### Scenario: Recording starts
- **WHEN** Alan begins recording
- **THEN** Alan shows a compact recording indicator with the active mode
- **AND** the active terminal or Alan content remains visible

#### Scenario: Processing starts
- **WHEN** recording stops and recognition or intent resolution begins
- **THEN** Alan shows a processing state
- **AND** the user can distinguish recording from processing

#### Scenario: Voice input succeeds
- **WHEN** Alan applies a voice intent successfully
- **THEN** Alan shows a concise success state identifying the action that was
  applied
- **AND** the feedback automatically recedes unless the result requires review

#### Scenario: Voice input fails
- **WHEN** recording, recognition, provider access, or intent routing fails
- **THEN** Alan shows a recoverable error with the next action available to the
  user
- **AND** Alan does not leave the UI in a recording or processing state

### Requirement: Keyboard-first operation
Alan Voice SHALL be usable without relying on the mouse for the primary voice
input path.

#### Scenario: User completes voice input by keyboard
- **WHEN** the user presses, holds, releases, and optionally cancels the Hold to
  Talk shortcut
- **THEN** Alan completes the voice input lifecycle without requiring pointer
  interaction

#### Scenario: User needs settings
- **WHEN** the user opens Alan Voice settings
- **THEN** Alan exposes keyboard-reachable controls for shortcut, mode,
  provider, credential status, language, and permission repair

### Requirement: Permissions and repair
Alan SHALL handle microphone, speech recognition, global shortcut, and any
required accessibility permissions with clear purpose and recovery language.

#### Scenario: Microphone permission is missing
- **WHEN** the user invokes Alan Voice without microphone permission
- **THEN** Alan explains that the microphone is needed only for Hold to Talk
- **AND** Alan offers a path to grant or reopen system permission settings

#### Scenario: Speech recognition permission is missing
- **WHEN** the selected recognition mode requires system speech recognition
  permission and it is not granted
- **THEN** Alan explains why speech recognition permission is needed
- **AND** Alan offers a path to grant or repair the permission

#### Scenario: Shortcut permission is missing
- **WHEN** the configured Hold to Talk shortcut requires global shortcut or
  accessibility permission that is not available
- **THEN** Alan explains the permission purpose
- **AND** Alan offers a keyboard-accessible fallback or repair path

### Requirement: Privacy and provider disclosure
Alan SHALL make audio handling, recognition mode, and provider state explicit
before cloud audio processing occurs.

#### Scenario: Local Mode is active
- **WHEN** Alan Voice is in Local Mode
- **THEN** Alan indicates that audio recognition is local
- **AND** Alan does not upload captured audio to a cloud speech provider

#### Scenario: Cloud Mode is active
- **WHEN** Alan Voice is in Cloud Mode
- **THEN** Alan indicates the active cloud provider before or during capture
- **AND** Alan makes clear that captured audio may be sent to that provider

#### Scenario: Provider changes
- **WHEN** the user changes the Cloud Mode provider
- **THEN** Alan updates the visible provider state
- **AND** future Cloud Mode captures use the newly selected provider

### Requirement: Performance isolation
Alan Voice SHALL avoid slowing app startup, conversation startup, terminal
startup, and current context loading.

#### Scenario: App starts with Alan Voice enabled
- **WHEN** Alan macOS starts
- **THEN** Alan does not initialize heavyweight audio, speech, or cloud
  recognition work on the critical startup path
- **AND** Alan Voice initializes lazily when invoked or opened in settings

#### Scenario: User finishes speaking
- **WHEN** the user releases Hold to Talk
- **THEN** Alan starts recognition and intent handling promptly
- **AND** Alan reports progress if the result is not ready within the expected
  fast feedback window

### Requirement: Legacy voice command retirement
Alan SHALL retire the old fixed-command shell voice control path when Alan
Voice becomes the owner of macOS voice input.

#### Scenario: Alan Voice is enabled
- **WHEN** Alan Voice is available in the macOS app
- **THEN** the old fixed command vocabulary controller is not exposed as a
  parallel user-facing voice feature
- **AND** shell commands reachable by voice are routed through Alan Voice
  intent resolution and normal app command ownership

#### Scenario: Legacy command phrase is spoken
- **WHEN** the user speaks a phrase that previously matched the legacy shell
  voice command list
- **THEN** Alan resolves it through the Alan Voice intent pipeline
- **AND** unsupported commands fail through the same recoverable voice feedback
  path as other app command intents
