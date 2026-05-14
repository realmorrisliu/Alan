import Foundation

#if os(macOS)
@MainActor
final class AlanShellDiagnostics {
    private let handler: @MainActor (String) -> Void

    init(handler: @escaping @MainActor (String) -> Void) {
        self.handler = handler
    }

    func record(_ message: String) {
        handler(message)
    }
}
#endif
