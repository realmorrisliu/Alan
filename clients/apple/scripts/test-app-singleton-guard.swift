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

private func temporarySupportDirectory(_ name: String) throws -> URL {
    let root = FileManager.default.temporaryDirectory
        .appendingPathComponent("alan-singleton-tests", isDirectory: true)
        .appendingPathComponent(UUID().uuidString, isDirectory: true)
        .appendingPathComponent(name, isDirectory: true)
    try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
    return root
}

private func acquiredGuard(
    supportDirectory: URL,
    bundleIdentifier: String
) throws -> AlanAppSingletonGuard {
    let acquisition = try AlanAppSingletonGuard.acquire(
        applicationSupportDirectory: supportDirectory,
        bundleIdentifier: bundleIdentifier
    )

    switch acquisition {
    case .acquired(let guardHandle):
        return guardHandle
    case .alreadyRunning:
        throw TestFailure.message("expected singleton lock acquisition to succeed")
    }
}

private func testRejectsSecondOwnerUntilRelease() throws {
    let supportDirectory = try temporarySupportDirectory("second-owner")
    let bundleIdentifier = "dev.alan.singleton-tests.second-owner"
    let first = try acquiredGuard(
        supportDirectory: supportDirectory,
        bundleIdentifier: bundleIdentifier
    )

    let second = try AlanAppSingletonGuard.acquire(
        applicationSupportDirectory: supportDirectory,
        bundleIdentifier: bundleIdentifier
    )
    switch second {
    case .acquired:
        throw TestFailure.message("second singleton acquisition unexpectedly succeeded")
    case .alreadyRunning(let ownerPID):
        try require(ownerPID == ProcessInfo.processInfo.processIdentifier, "owner PID should be recorded")
    }

    first.release()

    let third = try acquiredGuard(
        supportDirectory: supportDirectory,
        bundleIdentifier: bundleIdentifier
    )
    third.release()
}

private func testDroppedOwnerReleasesLock() throws {
    let supportDirectory = try temporarySupportDirectory("dropped-owner")
    let bundleIdentifier = "dev.alan.singleton-tests.dropped-owner"

    do {
        _ = try acquiredGuard(
            supportDirectory: supportDirectory,
            bundleIdentifier: bundleIdentifier
        )
    }

    let reacquired = try acquiredGuard(
        supportDirectory: supportDirectory,
        bundleIdentifier: bundleIdentifier
    )
    reacquired.release()
}

@main
private enum TestRunner {
    static func main() throws {
        try testRejectsSecondOwnerUntilRelease()
        try testDroppedOwnerReleasesLock()
    }
}
