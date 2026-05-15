import SwiftUI

#if os(macOS)
enum ShellSidebarSpaceContentPagerSettlementPhase: Equatable {
    case dragging
    case settlingToSource
    case settlingToTarget
}

struct ShellSidebarSpaceContentPagerState: Equatable {
    let sourceIndex: Int
    var targetIndex: Int?
    var dragOffset: CGFloat
    var pageWidth: CGFloat
    var settlementPhase: ShellSidebarSpaceContentPagerSettlementPhase

    var isSettling: Bool {
        settlementPhase != .dragging
    }

    var isEdgeResistance: Bool {
        targetIndex == nil
    }

    var committedTargetIndex: Int? {
        guard settlementPhase == .settlingToTarget else { return nil }
        return targetIndex
    }

    var direction: Int {
        guard let targetIndex else {
            return dragOffset < 0 ? 1 : -1
        }
        return targetIndex >= sourceIndex ? 1 : -1
    }

    var progress: CGFloat {
        let width = max(pageWidth, 1)
        return min(abs(dragOffset) / width, 0.98)
    }

    var pageIndicesForRendering: [Int] {
        guard let targetIndex, targetIndex != sourceIndex else {
            return [sourceIndex]
        }
        return [sourceIndex, targetIndex]
    }

    func offset(for index: Int) -> CGFloat {
        CGFloat(index - sourceIndex) * max(pageWidth, 1) + dragOffset
    }
}
#endif
