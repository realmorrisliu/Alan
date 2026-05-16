import CoreGraphics

#if os(macOS)
enum ShellSidebarPresentationHitTestingRole: Equatable {
    case none
    case pinned
    case floating
    case morphingFloatingToPinned
}

struct ShellSidebarPresentationConfiguration: Equatable {
    let sidebarWidth: CGFloat
    let floatingSidebarInset: CGFloat
    let floatingCornerRadius: CGFloat
}

enum ShellSidebarPresentationPhase: Equatable {
    case pinned(progress: CGFloat)
    case collapsedHidden
    case floatingRevealed(showsTrafficLights: Bool)
    case morphingFloatingToPinned(progress: CGFloat)

    static func resolved(
        isSidebarCollapsed: Bool,
        pinnedProgress: CGFloat,
        isFloatingPanelRevealed: Bool,
        showsFloatingTrafficLights: Bool,
        isFloatingToPinnedMorphActive: Bool,
        floatingToPinnedMorphProgress: CGFloat
    ) -> ShellSidebarPresentationPhase {
        if isFloatingToPinnedMorphActive {
            return .morphingFloatingToPinned(progress: floatingToPinnedMorphProgress)
        }

        if isFloatingPanelRevealed {
            return .floatingRevealed(showsTrafficLights: showsFloatingTrafficLights)
        }

        if !isSidebarCollapsed || pinnedProgress > ShellSidebarPresentationSnapshot.visibilityEpsilon {
            return .pinned(progress: pinnedProgress)
        }

        return .collapsedHidden
    }
}

struct ShellSidebarPresentationSnapshot: Equatable {
    static let visibilityEpsilon: CGFloat = 0.001

    let phase: ShellSidebarPresentationPhase
    let layoutProgress: CGFloat
    let surfaceOrigin: CGPoint
    let surfaceWidth: CGFloat
    let contentOffsetX: CGFloat
    let contentOpacity: Double
    let floatingTreatmentProgress: CGFloat
    let cornerRadius: CGFloat
    let overlayBottomInset: CGFloat
    let hitTestingRole: ShellSidebarPresentationHitTestingRole
    let chromeSurface: ShellWindowChromeSurface
    let showsOverlaySurface: Bool
    let showsPinnedSurfaceContent: Bool

    init(
        phase: ShellSidebarPresentationPhase,
        configuration: ShellSidebarPresentationConfiguration
    ) {
        self.phase = phase
        surfaceWidth = configuration.sidebarWidth

        switch phase {
        case let .pinned(progress):
            let progress = Self.clamp(progress)
            layoutProgress = progress
            surfaceOrigin = CGPoint(
                x: -configuration.sidebarWidth * (1 - progress),
                y: 0
            )
            contentOffsetX = surfaceOrigin.x
            contentOpacity = Double(progress)
            floatingTreatmentProgress = 0
            cornerRadius = 0
            overlayBottomInset = 0
            showsOverlaySurface = false
            showsPinnedSurfaceContent = progress > Self.visibilityEpsilon
            hitTestingRole = showsPinnedSurfaceContent ? .pinned : .none
            chromeSurface = ShellWindowChromeSurface(
                isVisible: showsPinnedSurfaceContent,
                origin: surfaceOrigin,
                width: configuration.sidebarWidth,
                showsStandardTrafficLights: showsPinnedSurfaceContent
            )

        case .collapsedHidden:
            layoutProgress = 0
            surfaceOrigin = .zero
            contentOffsetX = 0
            contentOpacity = 0
            floatingTreatmentProgress = 0
            cornerRadius = 0
            overlayBottomInset = 0
            showsOverlaySurface = false
            showsPinnedSurfaceContent = false
            hitTestingRole = .none
            chromeSurface = ShellWindowChromeSurface(
                isVisible: false,
                origin: .zero,
                width: configuration.sidebarWidth,
                showsStandardTrafficLights: false
            )

        case let .floatingRevealed(showsTrafficLights):
            layoutProgress = 0
            surfaceOrigin = CGPoint(
                x: configuration.floatingSidebarInset,
                y: configuration.floatingSidebarInset
            )
            contentOffsetX = 0
            contentOpacity = 1
            floatingTreatmentProgress = 1
            cornerRadius = configuration.floatingCornerRadius
            overlayBottomInset = configuration.floatingSidebarInset
            showsOverlaySurface = true
            showsPinnedSurfaceContent = false
            hitTestingRole = .floating
            chromeSurface = ShellWindowChromeSurface(
                isVisible: true,
                origin: surfaceOrigin,
                width: configuration.sidebarWidth,
                showsStandardTrafficLights: showsTrafficLights
            )

        case let .morphingFloatingToPinned(progress):
            let progress = Self.clamp(progress)
            let floatingProgress = 1 - progress
            layoutProgress = progress
            surfaceOrigin = CGPoint(
                x: configuration.floatingSidebarInset * floatingProgress,
                y: configuration.floatingSidebarInset * floatingProgress
            )
            contentOffsetX = 0
            contentOpacity = 1
            floatingTreatmentProgress = floatingProgress
            cornerRadius = configuration.floatingCornerRadius * floatingProgress
            overlayBottomInset = configuration.floatingSidebarInset * floatingProgress
            showsOverlaySurface = true
            showsPinnedSurfaceContent = false
            hitTestingRole = .morphingFloatingToPinned
            chromeSurface = ShellWindowChromeSurface(
                isVisible: true,
                origin: surfaceOrigin,
                width: configuration.sidebarWidth,
                showsStandardTrafficLights: true
            )
        }
    }

    var isSurfaceVisible: Bool {
        showsOverlaySurface || showsPinnedSurfaceContent
    }

    var layoutWidth: CGFloat {
        surfaceWidth * layoutProgress
    }

    var showsFloatingShadow: Bool {
        floatingTreatmentProgress > Self.visibilityEpsilon
    }

    var visibleSurfaceCount: Int {
        (showsOverlaySurface ? 1 : 0) + (showsPinnedSurfaceContent ? 1 : 0)
    }

    private static func clamp(_ value: CGFloat) -> CGFloat {
        min(max(value, 0), 1)
    }
}
#endif
