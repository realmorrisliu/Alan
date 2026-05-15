import Foundation

@main
struct ShellSplitModelTestRunner {
    static func main() throws {
        try ShellSplitModelTests.run()
    }
}

private enum ShellSplitModelTests {
    static func run() throws {
        try verifiesNewSplitsStoreEqualRatio()
        try verifiesDirectionalSplitsPlaceNewPaneOnRequestedSide()
        try verifiesSplitRatiosClampWhenResized()
        try verifiesEqualizeRestoresEverySplitRatio()
        try verifiesSameDirectionAttachKeepsBinarySplitTree()
        try verifiesSpatialFocusFollowsSplitTree()
        try verifiesSpatialFocusPreservesPerpendicularPosition()
        try verifiesPaneScopedCloseRemovesSelectedPane()
        try verifiesPaneScopedCloseKeepsInactivePaneTargeting()
        try verifiesPaneScopedCloseClosesSinglePaneTab()
        try verifiesPaneScopedCloseLeavesFinalSpaceEmpty()
        try verifiesPaneAllocationSkipsReservedRuntimeIDs()
        try verifiesSplitDecodeRequiresPersistedRatio()
        print("Shell split model tests passed.")
    }

    private static func verifiesNewSplitsStoreEqualRatio() throws {
        let state = ShellStateSnapshot.bootstrapDefault(workingDirectory: "/tmp")
        let result = try state.splittingPane("pane_1", direction: .vertical)
        let tree = try requireFocusedTabTree(result.state)

        expect(tree.kind == .split, "splitting a pane must create a split branch")
        expect(tree.ratio == 0.5, "new split branches must persist an equal divider ratio")
        expect(tree.children?.count == 2, "a split branch must keep two structural children")
    }

    private static func verifiesDirectionalSplitsPlaceNewPaneOnRequestedSide() throws {
        let base = ShellStateSnapshot.bootstrapDefault(workingDirectory: "/tmp")

        let rightTree = try requireFocusedTabTree(try base.splittingPane("pane_1", placement: .right).state)
        expect(rightTree.direction == .vertical, "split right must create a vertical split branch")
        expect(rightTree.paneIDs == ["pane_1", "pane_2"], "split right must place the new pane after the focused pane")

        let leftTree = try requireFocusedTabTree(try base.splittingPane("pane_1", placement: .left).state)
        expect(leftTree.direction == .vertical, "split left must create a vertical split branch")
        expect(leftTree.paneIDs == ["pane_2", "pane_1"], "split left must place the new pane before the focused pane")

        let downTree = try requireFocusedTabTree(try base.splittingPane("pane_1", placement: .down).state)
        expect(downTree.direction == .horizontal, "split down must create a horizontal split branch")
        expect(downTree.paneIDs == ["pane_1", "pane_2"], "split down must place the new pane after the focused pane")

        let upTree = try requireFocusedTabTree(try base.splittingPane("pane_1", placement: .up).state)
        expect(upTree.direction == .horizontal, "split up must create a horizontal split branch")
        expect(upTree.paneIDs == ["pane_2", "pane_1"], "split up must place the new pane before the focused pane")
    }

    private static func verifiesSplitRatiosClampWhenResized() throws {
        let state = ShellStateSnapshot.bootstrapDefault(workingDirectory: "/tmp")
        let split = try state.splittingPane("pane_1", direction: .vertical).state
        let splitID = try requireFocusedTabTree(split).nodeID

        let tooSmall = try split.resizingSplit(splitID, ratio: 0.01).state
        let tooSmallRatio = try requireFocusedTabTree(tooSmall).ratio
        expect(
            tooSmallRatio == ShellPaneTreeNode.minimumSplitRatio,
            "resize must clamp tiny split ratios to the minimum usable ratio"
        )

        let tooLarge = try split.resizingSplit(splitID, ratio: 0.99).state
        let tooLargeRatio = try requireFocusedTabTree(tooLarge).ratio
        expect(
            tooLargeRatio == ShellPaneTreeNode.maximumSplitRatio,
            "resize must clamp large split ratios to the maximum usable ratio"
        )
    }

    private static func verifiesEqualizeRestoresEverySplitRatio() throws {
        var state = ShellStateSnapshot.bootstrapDefault(workingDirectory: "/tmp")
        state = try state.splittingPane("pane_1", direction: .vertical).state
        let rootID = try requireFocusedTabTree(state).nodeID
        state = try state.resizingSplit(rootID, ratio: 0.72).state
        state = try state.equalizingSplits(in: state.focusedTabID).state
        let equalizedRatio = try requireFocusedTabTree(state).ratio

        expect(
            equalizedRatio == 0.5,
            "equalize must restore the tab's root split ratio"
        )
    }

    private static func verifiesSameDirectionAttachKeepsBinarySplitTree() throws {
        let state = ShellStateSnapshot.bootstrapDefault(workingDirectory: "/tmp")
        let split = try state.splittingPane("pane_1", direction: .vertical).state
        let attached = try requireFocusedTabTree(split).attachingPane(
            "pane_3",
            direction: .vertical,
            splitNodeID: "node_nested_split",
            newLeafNodeID: "node_pane_3"
        )

        expect(
            attached.children?.count == 2,
            "same-direction pane attachment must keep split branches binary"
        )
        guard let nestedSplit = attached.children?.last else {
            throw TestFailure("nested split missing")
        }
        expect(nestedSplit.kind == .split, "same-direction attachment must nest the final child")
        expect(nestedSplit.direction == .vertical, "nested split must keep the requested direction")
        expect(nestedSplit.children?.count == 2, "nested split must own exactly two children")
        expect(
            attached.paneIDs == ["pane_1", "pane_2", "pane_3"],
            "same-direction attachment must preserve pane ordering"
        )
    }

    private static func verifiesSpatialFocusFollowsSplitTree() throws {
        var state = ShellStateSnapshot.bootstrapDefault(workingDirectory: "/tmp")
        state = try state.splittingPane("pane_1", placement: .right).state
        let pane2 = state.focusedPaneID ?? "pane_2"
        let pane1Focused = try state.focusingPane("pane_1").state

        let rightResult = try pane1Focused.focusingAdjacentPane(.right)
        expect(rightResult.paneID == pane2, "focus right must move to the right sibling pane")

        let leftResult = try rightResult.state.focusingAdjacentPane(.left)
        expect(leftResult.paneID == "pane_1", "focus left must return to the left sibling pane")

        do {
            _ = try pane1Focused.focusingAdjacentPane(.left)
            expect(false, "focus left without a neighbor must throw")
        } catch ShellStateMutationError.spatialFocusTargetNotFound {
            // Expected.
        }
    }

    private static func verifiesSpatialFocusPreservesPerpendicularPosition() throws {
        var state = ShellStateSnapshot.bootstrapDefault(workingDirectory: "/tmp")
        state = try state.splittingPane("pane_1", placement: .right).state
        state = try state.splittingPane("pane_1", placement: .down).state
        state = try state.splittingPane("pane_2", placement: .down).state

        let lowerLeftFocused = try state.focusingPane("pane_3").state
        let rightResult = try lowerLeftFocused.focusingAdjacentPane(.right)
        expect(
            rightResult.paneID == "pane_4",
            "focus right from the lower-left pane must land on the lower-right pane"
        )

        let leftResult = try rightResult.state.focusingAdjacentPane(.left)
        expect(
            leftResult.paneID == "pane_3",
            "focus left from the lower-right pane must return to the lower-left pane"
        )

        var rowState = ShellStateSnapshot.bootstrapDefault(workingDirectory: "/tmp")
        rowState = try rowState.splittingPane("pane_1", placement: .down).state
        rowState = try rowState.splittingPane("pane_1", placement: .right).state
        rowState = try rowState.splittingPane("pane_2", placement: .right).state

        let upperRightFocused = try rowState.focusingPane("pane_3").state
        let downResult = try upperRightFocused.focusingAdjacentPane(.down)
        expect(
            downResult.paneID == "pane_4",
            "focus down from the upper-right pane must land on the lower-right pane"
        )

        let upResult = try downResult.state.focusingAdjacentPane(.up)
        expect(
            upResult.paneID == "pane_3",
            "focus up from the lower-right pane must return to the upper-right pane"
        )
    }

    private static func verifiesPaneScopedCloseKeepsInactivePaneTargeting() throws {
        var state = ShellStateSnapshot.bootstrapDefault(workingDirectory: "/tmp")
        state = try state.splittingPane("pane_1", placement: .right).state
        state = try state.focusingPane("pane_1").state

        let result = try state.closingPane("pane_2")
        let tree = try requireFocusedTabTree(result.state)

        expect(result.state.pane(paneID: "pane_2") == nil, "targeted close must remove the requested pane")
        expect(result.state.pane(paneID: "pane_1") != nil, "targeted close must preserve the selected sibling")
        expect(result.state.focusedPaneID == "pane_1", "closing an inactive pane must not move focus")
        expect(tree.paneIDs == ["pane_1"], "split tree must repair after closing the inactive pane")
    }

    private static func verifiesPaneScopedCloseRemovesSelectedPane() throws {
        var state = ShellStateSnapshot.bootstrapDefault(workingDirectory: "/tmp")
        state = try state.splittingPane("pane_1", placement: .right).state

        let result = try state.closingPane("pane_2")
        let tree = try requireFocusedTabTree(result.state)

        expect(result.state.pane(paneID: "pane_2") == nil, "selected pane close must remove the selected pane")
        expect(result.state.focusedPaneID == "pane_1", "selected pane close must focus the remaining sibling")
        expect(tree.paneIDs == ["pane_1"], "selected pane close must repair the split tree")
    }

    private static func verifiesPaneScopedCloseClosesSinglePaneTab() throws {
        var state = ShellStateSnapshot.bootstrapDefault(workingDirectory: "/tmp")
        state = try state.openingTerminalTab(
            in: state.focusedSpaceID,
            title: "Second",
            workingDirectory: "/tmp"
        ).state

        let result = try state.closingPane("pane_2")

        expect(result.state.pane(paneID: "pane_2") == nil, "single-pane tab close must remove that pane")
        expect(result.state.tab(tabID: "tab_2") == nil, "single-pane tab close must reuse tab close semantics")
        expect(result.state.pane(paneID: "pane_1") != nil, "single-pane tab close must preserve remaining tab panes")
        expect(result.state.focusedPaneID == "pane_1", "single-pane tab close must focus a remaining pane")
    }

    private static func verifiesPaneScopedCloseLeavesFinalSpaceEmpty() throws {
        let state = ShellStateSnapshot.bootstrapDefault(workingDirectory: "/tmp")

        let result = try state.closingPane("pane_1")

        expect(result.state.spaces.count == 1, "closing the final pane must keep the space")
        expect(result.state.spaces.first?.spaceID == "space_main", "closing the final pane must keep space identity")
        expect(result.state.spaces.first?.tabs.isEmpty == true, "closing the final pane must leave the space empty")
        expect(result.state.panes.isEmpty, "closing the final pane must remove the pane")
        expect(result.state.focusedSpaceID == "space_main", "closing the final pane must keep the empty space focused")
        expect(result.state.focusedTabID == nil, "closing the final pane must clear tab focus")
        expect(result.state.focusedPaneID == nil, "closing the final pane must clear pane focus")
    }

    private static func verifiesPaneAllocationSkipsReservedRuntimeIDs() throws {
        let state = ShellStateSnapshot.bootstrapDefault(workingDirectory: "/tmp")

        let openedTab = try state.openingTerminalTab(
            in: nil,
            title: nil,
            workingDirectory: nil,
            reservedPaneIDs: ["pane_2"]
        )
        expect(
            openedTab.paneID == "pane_3",
            "opening a tab must not reuse a pane ID reserved by a live runtime"
        )

        let splitPane = try state.splittingPane(
            "pane_1",
            direction: .vertical,
            reservedPaneIDs: ["pane_2"]
        )
        expect(
            splitPane.paneID == "pane_3",
            "splitting a pane must not reuse a pane ID reserved by a live runtime"
        )

        let newSpace = state.creatingTerminalSpace(
            title: nil,
            workingDirectory: nil,
            reservedPaneIDs: ["pane_2"]
        )
        expect(
            newSpace.paneID == "pane_3",
            "creating a space must not reuse a pane ID reserved by a live runtime"
        )
    }

    private static func verifiesSplitDecodeRequiresPersistedRatio() throws {
        let missingRatioJSON = """
        {
          "contract_version": "0.1",
          "window_id": "window_test",
          "focused_space_id": "space_main",
          "focused_tab_id": "tab_main",
          "focused_pane_id": "pane_1",
          "spaces": [
            {
              "space_id": "space_main",
              "title": "Terminal",
              "attention": "active",
              "tabs": [
                {
                  "tab_id": "tab_main",
                  "kind": "terminal",
                  "title": "Shell",
                  "pane_tree": {
                    "node_id": "node_split",
                    "kind": "split",
                    "direction": "vertical",
                    "children": [
                      {"node_id": "node_pane_1", "kind": "pane", "pane_id": "pane_1"},
                      {"node_id": "node_pane_2", "kind": "pane", "pane_id": "pane_2"}
                    ]
                  }
                }
              ]
            }
          ],
          "panes": [
            {"pane_id": "pane_1", "tab_id": "tab_main", "space_id": "space_main", "launch_target": "shell", "attention": "active"},
            {"pane_id": "pane_2", "tab_id": "tab_main", "space_id": "space_main", "launch_target": "shell", "attention": "idle"}
          ]
        }
        """
        do {
            _ = try JSONDecoder().decode(ShellStateSnapshot.self, from: Data(missingRatioJSON.utf8))
            expect(false, "split trees without persisted ratio must fail to decode")
        } catch DecodingError.keyNotFound(_, _) {
            // Expected.
        }
    }

    private static func requireFocusedTabTree(_ state: ShellStateSnapshot) throws -> ShellPaneTreeNode {
        guard let tabID = state.focusedTabID,
              let tab = state.tab(tabID: tabID)
        else {
            throw TestFailure("focused tab missing")
        }
        return tab.paneTree
    }

    private static func expect(
        _ condition: @autoclosure () -> Bool,
        _ message: String
    ) {
        guard condition() else {
            fputs("error: \(message)\n", stderr)
            exit(1)
        }
    }
}

private struct TestFailure: Error, CustomStringConvertible {
    let description: String

    init(_ description: String) {
        self.description = description
    }
}
