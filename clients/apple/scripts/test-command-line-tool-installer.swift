import Foundation

private enum TestFailure: Error, CustomStringConvertible {
    case message(String)

    var description: String {
        switch self {
        case .message(let message):
            return message
        }
    }
}

private func require(_ condition: @autoclosure () -> Bool, _ message: String) throws {
    if !condition() {
        throw TestFailure.message(message)
    }
}

private func temporaryDirectory(_ name: String) throws -> URL {
    let directory = FileManager.default.temporaryDirectory
        .appendingPathComponent("alan-cli-installer-tests", isDirectory: true)
        .appendingPathComponent(UUID().uuidString, isDirectory: true)
        .appendingPathComponent(name, isDirectory: true)
    try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
    return directory
}

private func makeResourceRoot() throws -> URL {
    let root = try temporaryDirectory("resources")
    let bin = root.appendingPathComponent("bin", isDirectory: true)
    try FileManager.default.createDirectory(at: bin, withIntermediateDirectories: true)

    for tool in AlanCommandLineToolInstaller.toolNames {
        let url = bin.appendingPathComponent(tool)
        try "#!/bin/sh\nexit 0\n".write(to: url, atomically: true, encoding: .utf8)
        try FileManager.default.setAttributes([.posixPermissions: 0o755], ofItemAtPath: url.path)
    }

    return root
}

private func testInstallsSymlinks() throws {
    let resourceRoot = try makeResourceRoot()
    let targetDirectory = try temporaryDirectory("target")

    let records = try AlanCommandLineToolInstaller.install(
        targetDirectory: targetDirectory,
        resourceURL: resourceRoot
    )

    try require(records.count == 2, "installer must report both tools")
    for tool in AlanCommandLineToolInstaller.toolNames {
        let target = targetDirectory.appendingPathComponent(tool)
        let destination = try FileManager.default.destinationOfSymbolicLink(atPath: target.path)
        try require(
            destination.hasSuffix("/bin/\(tool)"),
            "installer must link \(tool) to the embedded resource"
        )
    }
}

private func testSkipsNonAlanFiles() throws {
    let resourceRoot = try makeResourceRoot()
    let targetDirectory = try temporaryDirectory("existing")
    let existing = targetDirectory.appendingPathComponent("alan")
    try "not alan\n".write(to: existing, atomically: true, encoding: .utf8)

    let records = try AlanCommandLineToolInstaller.install(
        targetDirectory: targetDirectory,
        resourceURL: resourceRoot
    )
    let alan = records.first { $0.tool == "alan" }

    guard case .skipped = alan?.status else {
        throw TestFailure.message("installer must skip a non-alan existing file")
    }
}

private func testRejectsHomebrewPrefixTarget() throws {
    let resourceRoot = try makeResourceRoot()
    let homebrewPrefix = try temporaryDirectory("homebrew")
    let targetDirectory = homebrewPrefix.appendingPathComponent("bin", isDirectory: true)

    do {
        _ = try AlanCommandLineToolInstaller.install(
            targetDirectory: targetDirectory,
            resourceURL: resourceRoot,
            homebrewPrefixes: [homebrewPrefix.path]
        )
        throw TestFailure.message("installer must reject Homebrew-managed targets")
    } catch let error as CocoaError where error.code == .fileWriteNoPermission {
        _ = error
        return
    }
}

private func testRejectsAlanHomeBinTarget() throws {
    let resourceRoot = try makeResourceRoot()
    let targetDirectory = try temporaryDirectory("home")
        .appendingPathComponent(".alan", isDirectory: true)
        .appendingPathComponent("bin", isDirectory: true)

    do {
        _ = try AlanCommandLineToolInstaller.install(
            targetDirectory: targetDirectory,
            resourceURL: resourceRoot
        )
        throw TestFailure.message("installer must reject ~/.alan/bin-style targets")
    } catch let error as CocoaError where error.code == .fileWriteInvalidFileName {
        _ = error
        return
    }
}

@main
private enum TestRunner {
    static func main() throws {
        try testInstallsSymlinks()
        try testSkipsNonAlanFiles()
        try testRejectsHomebrewPrefixTarget()
        try testRejectsAlanHomeBinTarget()
    }
}
