import Foundation
import SwiftUI

#if os(macOS)
@MainActor
final class AlanMacPrimaryShellOwner: ObservableObject {
    let host: ShellHostController

    init(fileManager: FileManager = .default) {
        let windowContext = ShellWindowContext.make(
            fileManager: fileManager,
            windowID: "window_main"
        )
        host = ShellHostController.live(
            fileManager: fileManager,
            windowContext: windowContext,
            startupMode: .fresh
        )
    }
}
#endif
