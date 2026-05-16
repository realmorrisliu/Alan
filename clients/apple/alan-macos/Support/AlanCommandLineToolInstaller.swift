import Foundation

#if os(macOS)
struct AlanCommandLineToolInstallRecord: Equatable {
    enum Status: Equatable {
        case installed
        case skipped(String)
    }

    let tool: String
    let sourcePath: String
    let targetPath: String
    let status: Status
}

enum AlanCommandLineToolInstaller {
    static let defaultInstallDirectory = URL(fileURLWithPath: "/usr/local/bin", isDirectory: true)
    static let toolNames = ["alan", "alan-tui"]

    static func embeddedBinDirectory(resourceURL: URL? = Bundle.main.resourceURL) -> URL? {
        resourceURL?.appendingPathComponent("bin", isDirectory: true)
    }

    static func install(
        targetDirectory: URL = defaultInstallDirectory,
        resourceURL: URL? = Bundle.main.resourceURL,
        fileManager: FileManager = .default,
        homebrewPrefixes: [String]? = nil
    ) throws -> [AlanCommandLineToolInstallRecord] {
        guard let embeddedBinDirectory = embeddedBinDirectory(resourceURL: resourceURL) else {
            throw CocoaError(.fileNoSuchFile)
        }

        if targetDirectory.standardizedFileURL.path.contains("/.alan/bin") {
            throw CocoaError(.fileWriteInvalidFileName)
        }
        if isHomebrewPrefixTarget(
            targetDirectory,
            fileManager: fileManager,
            homebrewPrefixes: homebrewPrefixes
        ) {
            throw CocoaError(.fileWriteNoPermission)
        }

        try fileManager.createDirectory(
            at: targetDirectory,
            withIntermediateDirectories: true
        )

        return try toolNames.map { tool in
            let source = embeddedBinDirectory.appendingPathComponent(tool)
            let target = targetDirectory.appendingPathComponent(tool)

            guard fileManager.isExecutableFile(atPath: source.path) else {
                throw CocoaError(.fileNoSuchFile)
            }

            if fileManager.fileExists(atPath: target.path) || isSymbolicLink(target, fileManager: fileManager) {
                guard isAlanOwnedLink(target, tool: tool, fileManager: fileManager) else {
                    return AlanCommandLineToolInstallRecord(
                        tool: tool,
                        sourcePath: source.path,
                        targetPath: target.path,
                        status: .skipped("Existing file is not an alan.app command-line link.")
                    )
                }
                try fileManager.removeItem(at: target)
            }

            try fileManager.createSymbolicLink(
                at: target,
                withDestinationURL: source
            )

            return AlanCommandLineToolInstallRecord(
                tool: tool,
                sourcePath: source.path,
                targetPath: target.path,
                status: .installed
            )
        }
    }

    private static func isSymbolicLink(_ url: URL, fileManager: FileManager) -> Bool {
        guard let attributes = try? fileManager.attributesOfItem(atPath: url.path),
              let fileType = attributes[.type] as? FileAttributeType
        else {
            return false
        }
        return fileType == .typeSymbolicLink
    }

    private static func isAlanOwnedLink(
        _ url: URL,
        tool: String,
        fileManager: FileManager
    ) -> Bool {
        guard isSymbolicLink(url, fileManager: fileManager),
              let destination = try? fileManager.destinationOfSymbolicLink(atPath: url.path)
        else {
            return false
        }

        return destination.hasSuffix("/alan.app/Contents/Resources/bin/\(tool)")
    }

    private static func isHomebrewPrefixTarget(
        _ targetDirectory: URL,
        fileManager: FileManager,
        homebrewPrefixes: [String]? = nil
    ) -> Bool {
        let targetPath = targetDirectory.standardizedFileURL.path + "/"
        let prefixes = homebrewPrefixes ?? [
            "/opt/homebrew",
            "/usr/local",
        ].filter { prefix in
            fileManager.fileExists(atPath: "\(prefix)/Homebrew")
                || fileManager.fileExists(atPath: "\(prefix)/bin/brew")
        }

        return prefixes.contains { prefix in
            targetPath.hasPrefix(prefix + "/")
        }
    }
}
#endif
