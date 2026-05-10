#if os(macOS)
import AppKit

@MainActor
final class ShellClipboardWriter {
    func writeString(_ value: String) {
        let pasteboard = NSPasteboard.general
        pasteboard.clearContents()
        pasteboard.setString(value, forType: .string)
    }
}
#endif
