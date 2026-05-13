import AppKit
import Foundation

@main
struct ShellWindowPlacementTestRunner {
    static func main() throws {
        try ShellWindowPlacementTests.run()
    }
}

private enum ShellWindowPlacementTests {
    static func run() throws {
        try verifiesAppearanceModeAppliesToAttachedWindowImmediately()
        try verifiesSystemModeClearsExplicitWindowAppearanceImmediately()
        print("Shell window placement tests passed.")
    }

    private static func verifiesAppearanceModeAppliesToAttachedWindowImmediately() throws {
        let (window, placementView) = makeAttachedPlacementView()

        placementView.updateAppearanceMode(.dark)

        expect(
            window.appearance?.bestMatch(from: [.darkAqua, .aqua]) == .darkAqua,
            "dark appearance mode must apply to the attached window immediately"
        )
    }

    private static func verifiesSystemModeClearsExplicitWindowAppearanceImmediately() throws {
        let (window, placementView) = makeAttachedPlacementView()
        placementView.updateAppearanceMode(.dark)

        placementView.updateAppearanceMode(.system)

        expect(
            window.appearance == nil,
            "system appearance mode must clear the explicit window appearance immediately"
        )
    }

    private static func makeAttachedPlacementView() -> (
        window: NSWindow,
        placementView: ShellWindowPlacementNSView
    ) {
        let window = NSWindow(
            contentRect: NSRect(x: 0, y: 0, width: 320, height: 240),
            styleMask: [.titled, .closable, .miniaturizable, .resizable],
            backing: .buffered,
            defer: false
        )
        let contentView = NSView(frame: window.contentView?.bounds ?? .zero)
        let placementView = ShellWindowPlacementNSView(appearanceMode: .system) { _ in }

        contentView.addSubview(placementView)
        window.contentView = contentView

        return (window, placementView)
    }

    private static func expect(
        _ condition: @autoclosure () -> Bool,
        _ message: @autoclosure () -> String
    ) {
        guard condition() else {
            fatalError(message())
        }
    }
}
