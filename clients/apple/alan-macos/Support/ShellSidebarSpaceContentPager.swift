import SwiftUI

#if os(macOS)
enum ShellSidebarSpaceContentPagerSettlementPhase: Equatable {
    case dragging
    case settlingToSource
    case settlingToTarget
}

struct ShellSidebarSpaceContentPagerState: Equatable {
    static let renderRadius = 2
    static let overdragGapRatio: CGFloat = 0.18

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
        let lowerBound = sourceIndex - Self.renderRadius
        let upperBound = sourceIndex + Self.renderRadius
        return Array(lowerBound...upperBound)
    }

    func pageIndicesForRendering(validRange: Range<Int>) -> [Int] {
        pageIndicesForRendering.filter { validRange.contains($0) }
    }

    func offset(for index: Int) -> CGFloat {
        CGFloat(index - sourceIndex) * max(pageWidth, 1) + dragOffset
    }

    static func clampedDragOffset(for translationX: CGFloat, pageWidth: CGFloat) -> CGFloat {
        let limit = max(pageWidth, 1) * (1 + overdragGapRatio)
        return min(max(translationX, -limit), limit)
    }
}
#endif
