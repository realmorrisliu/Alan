#if os(macOS)
import AppKit

enum AlanMacPrimaryWindowPresenter {
    static func focusExistingWindow() {
        guard let window = NSApp.windows.first(where: { $0.title == "alan" }) ?? NSApp.windows.first
        else {
            return
        }
        window.makeKeyAndOrderFront(nil)
        NSApp.activate(ignoringOtherApps: true)
    }

    static func focusExistingWindowSoon() {
        DispatchQueue.main.async {
            focusExistingWindow()
        }
    }
}
#endif
