#if os(macOS)
import AppKit

enum AlanTerminalRoutedMouseEvent: Equatable {
    case mouseDown
    case mouseUp
    case rightMouseDown
    case rightMouseUp
    case otherMouseDown
    case otherMouseUp
    case mouseEntered
    case mouseMoved
    case mouseDragged
    case rightMouseDragged
    case otherMouseDragged
    case mouseExited
    case pressureChange
}

@MainActor
final class AlanTerminalNativeScrollViewAdapter {
    let scrollView = AlanTerminalRoutingScrollView()
    var onVisibleRowChange: ((Int) -> Void)?
    var onScrollWheel: ((NSEvent) -> Bool)? {
        didSet {
            scrollView.onScrollWheel = onScrollWheel
        }
    }
    var onMouseEvent: ((AlanTerminalRoutedMouseEvent, NSEvent) -> Bool)? {
        didSet {
            scrollView.onMouseEvent = onMouseEvent
        }
    }

    private let documentView = NSView(frame: .zero)
    private weak var canvasView: NSView?
    private var observers: [NSObjectProtocol] = []
    private var state = AlanTerminalScrollbackState.empty
    private var rowHeight: CGFloat = 1
    private var isProgrammaticSync = false
    private var lastSentRow: Int?

    init() {
        scrollView.hasVerticalScroller = false
        scrollView.hasHorizontalScroller = false
        scrollView.autohidesScrollers = false
        scrollView.usesPredominantAxisScrolling = true
        scrollView.scrollerStyle = .overlay
        scrollView.drawsBackground = false
        scrollView.contentView.clipsToBounds = false
        scrollView.contentView.postsBoundsChangedNotifications = true
        scrollView.documentView = documentView
        scrollView.translatesAutoresizingMaskIntoConstraints = false
        observers.append(
            NotificationCenter.default.addObserver(
                forName: NSView.boundsDidChangeNotification,
                object: scrollView.contentView,
                queue: .main
            ) { [weak self] _ in
                Task { @MainActor [weak self] in
                    self?.handleBoundsChange()
                }
            }
        )
    }

    deinit {
        observers.forEach { NotificationCenter.default.removeObserver($0) }
    }

    func attachCanvasView(_ canvasView: NSView) {
        guard self.canvasView !== canvasView else { return }
        self.canvasView?.removeFromSuperview()
        self.canvasView = canvasView
        canvasView.removeFromSuperview()
        canvasView.translatesAutoresizingMaskIntoConstraints = true
        canvasView.autoresizingMask = [.width, .height]
        canvasView.frame = scrollView.contentView.bounds
        scrollView.contentView.addSubview(canvasView)
    }

    func sync(
        state: AlanTerminalScrollbackState,
        viewportSize: CGSize,
        rowHeight: CGFloat = 1
    ) {
        self.state = state
        self.rowHeight = max(rowHeight, 1)
        let viewportWidth = max(viewportSize.width, 0)
        let viewportHeight = max(viewportSize.height, 0)
        documentView.frame.size.width = viewportWidth
        scrollView.hasVerticalScroller = state.nativeScrollbarVisible
        scrollView.verticalScrollElasticity = state.nativeScrollbarVisible ? .allowed : .none

        guard viewportWidth > 0, viewportHeight > 0, state.nativeScrollbarVisible else {
            documentView.frame.size.height = viewportHeight
            scrollView.contentView.scroll(to: .zero)
            synchronizeCanvasView()
            scrollView.reflectScrolledClipView(scrollView.contentView)
            return
        }

        let contentHeight = max(CGFloat(state.metrics.totalRows) * self.rowHeight, viewportHeight)
        documentView.frame.size.height = contentHeight
        let offsetY = CGFloat(
            state.metrics.totalRows
                - state.metrics.firstVisibleRow
                - state.metrics.visibleRows
        ) * self.rowHeight
        isProgrammaticSync = true
        scrollView.contentView.scroll(to: CGPoint(x: 0, y: max(offsetY, 0)))
        synchronizeCanvasView()
        scrollView.reflectScrolledClipView(scrollView.contentView)
        lastSentRow = state.metrics.firstVisibleRow
        isProgrammaticSync = false
    }

    private func handleBoundsChange() {
        synchronizeCanvasView()
        guard !isProgrammaticSync else { return }
        guard state.nativeScrollbarVisible else { return }
        let visibleRect = scrollView.contentView.documentVisibleRect
        let documentHeight = documentView.frame.height
        guard rowHeight > 0, visibleRect.height > 0, documentHeight > visibleRect.height else {
            return
        }

        let scrollOffset = documentHeight - visibleRect.origin.y - visibleRect.height
        let row = max(0, min(Int((scrollOffset / rowHeight).rounded()), maxFirstVisibleRow))
        guard row != lastSentRow else { return }
        lastSentRow = row
        onVisibleRowChange?(row)
    }

    private var maxFirstVisibleRow: Int {
        max(state.metrics.totalRows - state.metrics.visibleRows, 0)
    }

    private func synchronizeCanvasView() {
        let visibleRect = scrollView.contentView.documentVisibleRect
        canvasView?.frame = NSRect(origin: visibleRect.origin, size: scrollView.contentView.bounds.size)
    }
}

@MainActor
final class AlanTerminalRoutingScrollView: NSScrollView {
    var onScrollWheel: ((NSEvent) -> Bool)?
    var onMouseEvent: ((AlanTerminalRoutedMouseEvent, NSEvent) -> Bool)?

    override var mouseDownCanMoveWindow: Bool { false }

    override func acceptsFirstMouse(for event: NSEvent?) -> Bool {
        true
    }

    override func mouseDown(with event: NSEvent) {
        if onMouseEvent?(.mouseDown, event) == true {
            return
        }
        super.mouseDown(with: event)
    }

    override func mouseUp(with event: NSEvent) {
        if onMouseEvent?(.mouseUp, event) == true {
            return
        }
        super.mouseUp(with: event)
    }

    override func rightMouseDown(with event: NSEvent) {
        if onMouseEvent?(.rightMouseDown, event) == true {
            return
        }
        super.rightMouseDown(with: event)
    }

    override func rightMouseUp(with event: NSEvent) {
        if onMouseEvent?(.rightMouseUp, event) == true {
            return
        }
        super.rightMouseUp(with: event)
    }

    override func otherMouseDown(with event: NSEvent) {
        if onMouseEvent?(.otherMouseDown, event) == true {
            return
        }
        super.otherMouseDown(with: event)
    }

    override func otherMouseUp(with event: NSEvent) {
        if onMouseEvent?(.otherMouseUp, event) == true {
            return
        }
        super.otherMouseUp(with: event)
    }

    override func mouseEntered(with event: NSEvent) {
        if onMouseEvent?(.mouseEntered, event) == true {
            return
        }
        super.mouseEntered(with: event)
    }

    override func mouseMoved(with event: NSEvent) {
        if onMouseEvent?(.mouseMoved, event) == true {
            return
        }
        super.mouseMoved(with: event)
    }

    override func mouseDragged(with event: NSEvent) {
        if onMouseEvent?(.mouseDragged, event) == true {
            return
        }
        super.mouseDragged(with: event)
    }

    override func rightMouseDragged(with event: NSEvent) {
        if onMouseEvent?(.rightMouseDragged, event) == true {
            return
        }
        super.rightMouseDragged(with: event)
    }

    override func otherMouseDragged(with event: NSEvent) {
        if onMouseEvent?(.otherMouseDragged, event) == true {
            return
        }
        super.otherMouseDragged(with: event)
    }

    override func mouseExited(with event: NSEvent) {
        if onMouseEvent?(.mouseExited, event) == true {
            return
        }
        super.mouseExited(with: event)
    }

    override func scrollWheel(with event: NSEvent) {
        if onScrollWheel?(event) == true {
            return
        }
        super.scrollWheel(with: event)
    }

    override func pressureChange(with event: NSEvent) {
        if onMouseEvent?(.pressureChange, event) == true {
            return
        }
        super.pressureChange(with: event)
    }
}
#endif
