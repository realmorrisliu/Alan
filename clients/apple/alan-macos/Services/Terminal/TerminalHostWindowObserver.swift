#if os(macOS)
import AppKit

final class TerminalHostWindowObserver {
    private var observers: [NSObjectProtocol] = []

    func install(
        for window: NSWindow?,
        onRuntimeEnvironmentChange: @escaping () -> Void,
        onSurfaceEnvironmentChange: @escaping () -> Void
    ) {
        remove()
        guard let window else { return }

        let center = NotificationCenter.default
        observers = [
            center.addObserver(
                forName: NSWindow.didBecomeKeyNotification,
                object: window,
                queue: .main
            ) { _ in
                onRuntimeEnvironmentChange()
            },
            center.addObserver(
                forName: NSWindow.didResignKeyNotification,
                object: window,
                queue: .main
            ) { _ in
                onRuntimeEnvironmentChange()
            },
            center.addObserver(
                forName: NSWindow.didChangeScreenNotification,
                object: window,
                queue: .main
            ) { _ in
                onSurfaceEnvironmentChange()
                onRuntimeEnvironmentChange()
            },
            center.addObserver(
                forName: NSWindow.didChangeOcclusionStateNotification,
                object: window,
                queue: .main
            ) { _ in
                onSurfaceEnvironmentChange()
                onRuntimeEnvironmentChange()
            },
        ]
    }

    func remove() {
        observers.forEach(NotificationCenter.default.removeObserver)
        observers.removeAll()
    }
}
#endif
