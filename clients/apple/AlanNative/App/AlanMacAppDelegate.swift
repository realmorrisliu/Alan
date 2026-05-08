#if os(macOS)
import AppKit

final class AlanMacAppDelegate: NSObject, NSApplicationDelegate {
    func applicationShouldHandleReopen(
        _ sender: NSApplication,
        hasVisibleWindows flag: Bool
    ) -> Bool {
        AlanMacPrimaryWindowPresenter.focusExistingWindow()
        return true
    }
}
#endif
