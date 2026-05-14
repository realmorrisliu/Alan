import SwiftUI

#if os(macOS)
import AppKit

@MainActor
final class ShellVoiceCommandController: NSObject, ObservableObject, NSSpeechRecognizerDelegate {
    @Published private(set) var isListening = false

    private let recognizer = NSSpeechRecognizer()
    private var recognitionHandler: ((String) -> Void)?

    override init() {
        super.init()
        recognizer?.delegate = self
        recognizer?.listensInForegroundOnly = false
        recognizer?.blocksOtherRecognizers = false
        recognizer?.commands = [
            "new space",
            "new alan space",
            "open tab",
            "open in alan",
            "focus best pane",
            "route to best pane",
            "split right",
            "split down",
            "split left",
            "split up",
            "focus left",
            "focus right",
            "focus up",
            "focus down",
            "equalize splits",
            "lift pane",
            "close pane",
            "close tab",
            "jump to attention",
            "focus waiting pane",
            "copy snapshot",
        ]
    }

    func toggleListening(handler: @escaping (String) -> Void) {
        isListening ? stopListening() : startListening(handler: handler)
    }

    func startListening(handler: @escaping (String) -> Void) {
        recognitionHandler = handler
        recognizer?.startListening()
        isListening = recognizer != nil
    }

    func stopListening() {
        recognizer?.stopListening()
        isListening = false
        recognitionHandler = nil
    }

    func speechRecognizer(_ sender: NSSpeechRecognizer, didRecognizeCommand command: String) {
        recognitionHandler?(command)
    }
}
#endif
