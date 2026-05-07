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
        try verifiesLegacySplitDecodeDefaultsToEqualRatio()
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

    private static func verifiesLegacySplitDecodeDefaultsToEqualRatio() throws {
        let legacyJSON = """
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
        let data = Data(legacyJSON.utf8)
        let state = try JSONDecoder().decode(ShellStateSnapshot.self, from: data)
        let tree = try requireFocusedTabTree(state)

        expect(tree.ratio == 0.5, "legacy split trees without ratio must decode as equal splits")

        let encoded = try JSONEncoder().encode(state)
        let encodedString = String(decoding: encoded, as: UTF8.self)
        expect(encodedString.contains("\"ratio\""), "decoded legacy split ratios must persist when re-encoded")
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
