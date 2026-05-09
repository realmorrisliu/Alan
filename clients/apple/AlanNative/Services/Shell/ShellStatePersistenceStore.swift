import Foundation

#if os(macOS)
struct ShellStatePersistenceStore {
    private static let persistenceFilePrefix = "shell-state-"
    private static let persistenceFileExtension = ".json"
    private static let defaultRestorationWindowID = "window_main"

    private let fileManager: FileManager
    private let persistenceURL: URL

    init(fileManager: FileManager = .default, persistenceURL: URL) {
        self.fileManager = fileManager
        self.persistenceURL = persistenceURL
    }

    func save(_ shellState: ShellStateSnapshot) {
        let parentURL = persistenceURL.deletingLastPathComponent()
        try? fileManager.createDirectory(at: parentURL, withIntermediateDirectories: true)
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
        guard let data = try? encoder.encode(shellState) else { return }
        try? data.write(to: persistenceURL, options: .atomic)
    }

    static func defaultPersistenceURL(windowID: String, fileManager: FileManager) -> URL {
        let sanitizedWindowID = windowID
            .replacingOccurrences(of: "/", with: "_")
            .replacingOccurrences(of: ":", with: "_")
        return persistenceDirectory(fileManager: fileManager)
            .appendingPathComponent("\(persistenceFilePrefix)\(sanitizedWindowID)\(persistenceFileExtension)")
    }

    @MainActor
    static func restoredWindowContext(
        fileManager: FileManager,
        restorePrevious: Bool
    ) -> ShellWindowContext? {
        guard restorePrevious else { return nil }

        let directory = persistenceDirectory(fileManager: fileManager)
        guard let urls = try? fileManager.contentsOfDirectory(
            at: directory,
            includingPropertiesForKeys: [.contentModificationDateKey],
            options: [.skipsHiddenFiles]
        ) else {
            return nil
        }

        let candidates = urls.compactMap { url -> (Date, ShellWindowContext)? in
            guard isShellStatePersistenceURL(url),
                  let state = restoreShellState(fileManager: fileManager, persistenceURL: url)
            else {
                return nil
            }

            let values = try? url.resourceValues(forKeys: [.contentModificationDateKey])
            let modifiedAt = values?.contentModificationDate ?? .distantPast
            return (
                modifiedAt,
                ShellWindowContext(
                    windowID: state.windowID,
                    persistenceURL: url,
                    terminalRuntimeRegistry: TerminalRuntimeRegistry()
                )
            )
        }

        return candidates.max { lhs, rhs in lhs.0 < rhs.0 }?.1
    }

    @MainActor
    static func defaultWindowContext(
        fileManager: FileManager,
        restorePrevious: Bool
    ) -> ShellWindowContext {
        if restorePrevious {
            return ShellWindowContext.make(
                fileManager: fileManager,
                windowID: defaultRestorationWindowID
            )
        }

        return ShellWindowContext.make(fileManager: fileManager)
    }

    static func restoreShellState(
        fileManager: FileManager,
        persistenceURL: URL
    ) -> ShellStateSnapshot? {
        guard fileManager.fileExists(atPath: persistenceURL.path),
              let data = try? Data(contentsOf: persistenceURL),
              let state = try? JSONDecoder().decode(ShellStateSnapshot.self, from: data),
              !state.spaces.isEmpty,
              !state.panes.isEmpty
        else {
            return nil
        }
        return state
    }

    private static func persistenceDirectory(fileManager: FileManager) -> URL {
        let appSupportURL =
            fileManager.urls(for: .applicationSupportDirectory, in: .userDomainMask).first
            ?? fileManager.temporaryDirectory
        return appSupportURL.appendingPathComponent("AlanNative", isDirectory: true)
    }

    private static func isShellStatePersistenceURL(_ url: URL) -> Bool {
        let fileName = url.lastPathComponent
        return fileName.hasPrefix(persistenceFilePrefix)
            && fileName.hasSuffix(persistenceFileExtension)
            && fileName != "shell-state-v0.1.json"
    }
}
#endif
