# Sidebar-local Space Swipe Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Restore macOS space swipe to a sidebar-local, finger-tracked content pager that leaves the terminal workspace and fixed sidebar controls stable until commit.

**Architecture:** `MacShellRootView` returns to a stable shell layout with one sidebar surface and one committed `ShellWorkspaceView`. `ShellSidebarView` owns the space content pager for only the active space header and tab list. `ShellSidebarSwipeMonitor` remains an input adapter; committed selection still flows through `ShellHostController`.

**Tech Stack:** SwiftUI, AppKit scroll event monitoring, OpenSpec, shell Swift script tests, Xcode macOS build.

---

### Task 1: Correct The Active OpenSpec Delta

**Files:**
- Modify: `openspec/changes/refine-macos-sidebar-interactions/proposal.md`
- Modify: `openspec/changes/refine-macos-sidebar-interactions/design.md`
- Modify: `openspec/changes/refine-macos-sidebar-interactions/specs/macos-shell-workspace-interactions/spec.md`
- Modify: `openspec/changes/refine-macos-sidebar-interactions/specs/macos-shell-build-test-contract/spec.md`
- Modify: `openspec/changes/refine-macos-sidebar-interactions/tasks.md`

- [ ] **Step 1: Replace the incorrect full-window pager language in the design**

  In `openspec/changes/refine-macos-sidebar-interactions/design.md`, replace Decision 4 with this text:

  ```markdown
  4. **Space swipe is a sidebar-local content pager.**

     The gesture model should track `sourceIndex`, `targetIndex`, `dragOffset`,
     `pageWidth`, and settlement phase for the sidebar's active-space content
     area. The moving page includes only the active space title/header and the
     active space tab list. Command input, the bottom space switcher, sidebar
     material/chrome, traffic lights, and the terminal workspace surface remain
     visually fixed while dragging. Commit and cancel use the same pager model,
     but shell selection and terminal focus change only when the gesture commits
     to a target space.

     Alternative considered: make the entire shell content area a continuous
     space pager. That breaks the accepted terminal-first layout because the
     terminal workspace slides, duplicates, and exposes artifacts during a
     sidebar navigation gesture.
  ```

  Also update the Goals section so the swipe goal reads:

  ```markdown
  - Replace discontinuous sidebar space swipe behavior with a continuous,
    sidebar-local content pager over the ordered `ShellSpace` sequence.
  ```

- [ ] **Step 2: Fix the workspace interaction delta spec**

  In `openspec/changes/refine-macos-sidebar-interactions/specs/macos-shell-workspace-interactions/spec.md`:

  - Remove the `REMOVED Requirements` section that removes `Sidebar swipe previews spaces without moving the workspace`.
  - Remove the `Requirement: Space switching uses a continuous pager` block.
  - Add this `MODIFIED Requirements` block after the existing `ADDED Requirements` block for authoritative selection, or make it the first section if there is no `MODIFIED Requirements` section:

  ```markdown
  ## MODIFIED Requirements

  ### Requirement: Sidebar swipe previews spaces without moving the workspace
  Horizontal swipe gestures that originate inside the macOS sidebar SHALL drive
  a sidebar-local, finger-tracked space content pager. The moving page SHALL
  include only the sidebar's active-space header and active-space tab list. The
  command input, bottom space switcher, sidebar material surface, sidebar chrome,
  macOS traffic-light placement, and workspace terminal surface SHALL remain
  visually fixed while the gesture is active. alan SHALL avoid mutating durable
  shell selection until the gesture commits.

  #### Scenario: Gesture-tracked sidebar content pager
  - **WHEN** a user horizontally swipes inside the sidebar and an adjacent space exists
  - **THEN** the current sidebar space header and tab list move with the gesture while the adjacent space content previews from the side
  - **AND** the space header and tab list use the same full sidebar content page width for horizontal offsets
  - **AND** the preview movement is rendered directly from horizontal finger translation instead of being amplified, quantized, or shaped by the commit threshold
  - **AND** the command input remains fixed
  - **AND** the bottom space switcher remains fixed as the stable space navigation control
  - **AND** the workspace terminal surface remains visually stable on the original selected space
  - **AND** visible terminal panes keep their runtime identities instead of being restarted, duplicated, or horizontally offset as a side effect of the drag
  - **AND** vertical tab-list scrolling does not move while horizontal intent is locked

  #### Scenario: Undecided axis buffers mixed deltas
  - **WHEN** a sidebar scroll gesture has not yet crossed the horizontal or vertical intent threshold
  - **THEN** alan buffers the initial mixed deltas instead of applying partial vertical tab-list scrolling or horizontal pager movement
  - **AND** the gesture is routed only after horizontal or vertical intent is locked

  #### Scenario: Content pager reaches sequence edge
  - **WHEN** a user swipes past the first or last available space
  - **THEN** alan applies bounded edge resistance to the moving sidebar content rather than wrapping unexpectedly or showing a nonexistent space page
  - **AND** releasing before a valid target is selected returns the content pager to the current space

  #### Scenario: Commit updates focus at the authoritative transition point
  - **WHEN** the user releases a space swipe past the commit threshold or with sufficient release velocity toward an adjacent space
  - **THEN** alan commits the target space through the shell controller selection and focus path
  - **AND** the sidebar content pager settles smoothly to the committed space without being reverted by concurrent runtime updates
  - **AND** the workspace terminal surface and terminal focus follow the committed space after shell selection commits

  #### Scenario: Cancel preserves focus and layout
  - **WHEN** the user releases a space swipe before the commit threshold
  - **THEN** alan animates the sidebar content pager back to the original space
  - **AND** selected space, selected tab, terminal focus, split tree, and divider ratios remain unchanged

  #### Scenario: Phaseful gesture waits for real release
  - **WHEN** a user pauses a phaseful horizontal trackpad swipe while their fingers remain on the trackpad
  - **THEN** alan keeps the sidebar content pager at the current drag offset
  - **AND** alan does not commit or cancel until the gesture ends, is cancelled, or enters momentum

  #### Scenario: Release uses last effective velocity
  - **WHEN** a phaseful horizontal trackpad swipe ends or enters momentum
  - **THEN** alan evaluates commit using current pager progress and the last effective finger velocity before release
  - **AND** alan does not replace that velocity with a zero-delta ended event

  #### Scenario: Fast flick can commit
  - **WHEN** a user performs a fast horizontal flick inside the sidebar
  - **THEN** alan recognizes the dominant horizontal release or momentum handoff as a space switch
  - **AND** alan may commit from velocity even when the gesture produced only a short visible translation before release

  #### Scenario: Phase-less gesture settles
  - **WHEN** a horizontal sidebar swipe comes from a scroll device that does not provide gesture phases
  - **THEN** alan may treat a short idle gap as release to avoid leaving the content pager stuck
  - **AND** shell selection follows the same commit threshold as other sidebar swipes

  #### Scenario: Vertical scroll is not captured
  - **WHEN** a user's gesture is primarily vertical in the sidebar tab list
  - **THEN** the native vertical tab-list scroll receives the gesture and the workspace space transition does not begin
  - **AND** horizontal sidebar content pager movement is not applied while vertical intent is locked
  ```

- [ ] **Step 3: Update build-test wording**

  In `openspec/changes/refine-macos-sidebar-interactions/specs/macos-shell-build-test-contract/spec.md`, make the space gesture scenario require fixed terminal/sidebar regions:

  ```markdown
  #### Scenario: Sidebar-local space pager gesture tested
  - **WHEN** horizontal space swipe behavior changes
  - **THEN** focused tests cover undecided-axis buffering, horizontal intent lock, vertical scroll pass-through, edge resistance, commit threshold, cancel threshold, phaseful release, phase-less idle release, and fast flick velocity commit
  - **AND** verification confirms only the sidebar active-space header and tab list move during the gesture
  - **AND** verification confirms the command input, bottom space switcher, sidebar chrome, traffic lights, and workspace terminal surface remain fixed during the gesture
  ```

- [ ] **Step 4: Update tasks terminology**

  In `openspec/changes/refine-macos-sidebar-interactions/tasks.md`, rename section `## 3. Continuous Space Pager` to:

  ```markdown
  ## 3. Sidebar-local Space Content Pager
  ```

  Replace tasks 3.1 and 3.3 with:

  ```markdown
  - [ ] 3.1 Replace the root-level `ShellSpacePagerState` usage with a sidebar-local pager state that tracks source index, target index, drag offset, sidebar content page width, commit/cancel state, and settlement phase.
  - [ ] 3.3 Render current and adjacent sidebar active-space content pages from the same pager offset so users can see the target space edge while dragging, without moving command input, the bottom space switcher, sidebar chrome, or the terminal workspace surface.
  ```

- [ ] **Step 5: Validate the spec change**

  Run:

  ```bash
  openspec validate refine-macos-sidebar-interactions --strict
  ```

  Expected: exits `0` with no validation errors.

- [ ] **Step 6: Commit the spec correction**

  Run:

  ```bash
  git add openspec/changes/refine-macos-sidebar-interactions
  git commit -m "Spec sidebar-local space swipe"
  ```

### Task 2: Extract Sidebar Content Pager State And Tests

**Files:**
- Create: `clients/apple/alan-macos/Support/ShellSidebarSpaceContentPager.swift`
- Modify: `clients/apple/alan-macos/Support/ShellSidebarSwipeMonitor.swift`
- Modify: `clients/apple/scripts/test-shell-sidebar-swipe-monitor.swift`
- Modify: `clients/apple/scripts/test-shell-sidebar-swipe-monitor.sh`
- Modify: `clients/apple/alan-macos.xcodeproj/project.pbxproj`

- [ ] **Step 1: Add failing pager-state expectations**

  In `clients/apple/scripts/test-shell-sidebar-swipe-monitor.swift`, rename the pager tests to the sidebar-local type before the implementation exists:

  ```swift
  private static func verifiesPagerOffsetsPagesFromSharedDragOffset() {
      let pager = ShellSidebarSpaceContentPagerState(
          sourceIndex: 1,
          targetIndex: 2,
          dragOffset: -72,
          pageWidth: 240,
          settlementPhase: .dragging
      )

      expect(
          pager.offset(for: 1).isApproximately(-72),
          "source sidebar content page must move directly with finger translation"
      )
      expect(
          pager.offset(for: 2).isApproximately(168),
          "target sidebar content page must share the same drag offset from the adjacent page position"
      )
      expect(
          pager.pageIndicesForRendering == [1, 2],
          "sidebar content pager must render source and adjacent target pages together"
      )
  }
  ```

  Also change the edge and settlement tests to use `ShellSidebarSpaceContentPagerState`.

- [ ] **Step 2: Run the test to verify it fails**

  Run:

  ```bash
  clients/apple/scripts/test-shell-sidebar-swipe-monitor.sh
  ```

  Expected: compile failure mentioning `cannot find 'ShellSidebarSpaceContentPagerState' in scope`.

- [ ] **Step 3: Create the sidebar-local pager state file**

  Add `clients/apple/alan-macos/Support/ShellSidebarSpaceContentPager.swift`:

  ```swift
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
  ```

- [ ] **Step 4: Remove pager state from the monitor file**

  In `clients/apple/alan-macos/Support/ShellSidebarSwipeMonitor.swift`, delete the
  existing `ShellSpacePagerSettlementPhase` enum and the existing
  `ShellSpacePagerState` struct. After this edit the file should still define
  only the swipe input types and monitor:

  ```swift
  enum ShellSidebarSwipePhase {
      case began
      case changed
      case ended
      case cancelled
  }

  struct ShellSidebarSwipeUpdate {
      let phase: ShellSidebarSwipePhase
      let translationX: CGFloat
      let velocityX: CGFloat
  }

  struct ShellSidebarSwipeMonitor: NSViewRepresentable {
      let onUpdate: (ShellSidebarSwipeUpdate) -> Void
  }
  ```

  Keep `ShellSidebarSwipePhase`, `ShellSidebarSwipeUpdate`, and `ShellSidebarSwipeMonitor` unchanged.

- [ ] **Step 5: Compile the new file in the script test**

  In `clients/apple/scripts/test-shell-sidebar-swipe-monitor.sh`, add the new Swift file before the test file:

  ```bash
  CLANG_MODULE_CACHE_PATH="$MODULE_CACHE_DIR" swiftc \
      -D ALAN_TESTING \
      "$REPO_ROOT/clients/apple/alan-macos/Support/ShellSidebarSwipeMonitor.swift" \
      "$REPO_ROOT/clients/apple/alan-macos/Support/ShellSidebarSpaceContentPager.swift" \
      "$REPO_ROOT/clients/apple/scripts/test-shell-sidebar-swipe-monitor.swift" \
      -o "$TEST_BINARY"
  ```

- [ ] **Step 6: Add the new Swift source to the Xcode project**

  In `clients/apple/alan-macos.xcodeproj/project.pbxproj`, add a build file, file reference, group child, and Sources entry for:

  ```text
  ShellSidebarSpaceContentPager.swift
  ```

  Follow the existing pattern for `ShellSidebarSwipeMonitor.swift`:

  ```text
  00000000000000000000003B /* ShellSidebarSpaceContentPager.swift in Sources */ = {isa = PBXBuildFile; fileRef = 00000000000000000000013B /* ShellSidebarSpaceContentPager.swift */; };
  00000000000000000000013B /* ShellSidebarSpaceContentPager.swift */ = {isa = PBXFileReference; lastKnownFileType = sourcecode.swift; path = Support/ShellSidebarSpaceContentPager.swift; sourceTree = "<group>"; };
  ```

  Add `00000000000000000000013B` to the main group children near `ShellSidebarSwipeMonitor.swift`, and add `00000000000000000000003B` to the app target `PBXSourcesBuildPhase`.

- [ ] **Step 7: Run the focused test**

  Run:

  ```bash
  clients/apple/scripts/test-shell-sidebar-swipe-monitor.sh
  ```

  Expected output includes:

  ```text
  Shell sidebar swipe monitor tests passed.
  ```

- [ ] **Step 8: Commit pager state extraction**

  Run:

  ```bash
  git add clients/apple/alan-macos/Support/ShellSidebarSpaceContentPager.swift clients/apple/alan-macos/Support/ShellSidebarSwipeMonitor.swift clients/apple/scripts/test-shell-sidebar-swipe-monitor.swift clients/apple/scripts/test-shell-sidebar-swipe-monitor.sh clients/apple/alan-macos.xcodeproj/project.pbxproj
  git commit -m "Extract sidebar space content pager state"
  ```

### Task 3: Move Space Swipe Ownership Into ShellSidebarView

**Files:**
- Modify: `clients/apple/alan-macos/Views/Shell/ShellSidebarView.swift`

- [ ] **Step 1: Add local pager state**

  In `ShellSidebarView`, add these properties:

  ```swift
  @Environment(\.accessibilityReduceMotion) private var reduceMotion
  @State private var spacePager: ShellSidebarSpaceContentPagerState?
  @State private var spacePagerToken = 0
  @State private var spacePagerPageWidth: CGFloat = 1
  ```

  Remove these stored inputs:

  ```swift
  let previewedSpaceID: String?
  let isSpaceSwipeGestureLocked: Bool
  let onSpaceSwipe: (ShellSidebarSwipeUpdate) -> Void
  ```

- [ ] **Step 2: Handle swipe updates locally**

  Add these methods inside `ShellSidebarView`:

  ```swift
  private func handleSpaceSwipe(_ update: ShellSidebarSwipeUpdate) {
      switch update.phase {
      case .began:
          guard spacePager?.isSettling != true else { return }
          beginSpacePager()
      case .changed:
          guard spacePager?.isSettling != true else { return }
          updateSpacePager(translationX: update.translationX)
      case .ended:
          finishSpacePager(velocityX: update.velocityX)
      case .cancelled:
          settleSpacePager(committing: false)
      }
  }

  private func beginSpacePager() {
      guard let sourceIndex = selectedSpaceIndex else { return }
      var transaction = Transaction()
      transaction.disablesAnimations = true
      withTransaction(transaction) {
          spacePager = ShellSidebarSpaceContentPagerState(
              sourceIndex: sourceIndex,
              targetIndex: nil,
              dragOffset: 0,
              pageWidth: sidebarSwipePageWidth,
              settlementPhase: .dragging
          )
      }
  }

  private func updateSpacePager(translationX: CGFloat) {
      guard abs(translationX) > 0.5 else { return }
      guard let sourceIndex = spacePager?.sourceIndex ?? selectedSpaceIndex else { return }
      let direction = translationX < 0 ? 1 : -1
      let targetIndex = adjacentSpaceIndex(from: sourceIndex, direction: direction)
      let dragOffset = targetIndex == nil ? resistedEdgeOffset(for: translationX) : translationX

      var transaction = Transaction()
      transaction.disablesAnimations = true
      withTransaction(transaction) {
          spacePager = ShellSidebarSpaceContentPagerState(
              sourceIndex: sourceIndex,
              targetIndex: targetIndex,
              dragOffset: dragOffset,
              pageWidth: sidebarSwipePageWidth,
              settlementPhase: .dragging
          )
      }
  }

  private func finishSpacePager(velocityX: CGFloat) {
      guard let pager = spacePager else { return }
      guard pager.targetIndex != nil else {
          settleSpacePager(committing: false)
          return
      }

      let velocityDirection = velocityX < 0 ? 1 : -1
      let fastEnough = abs(velocityX) >= 120 && velocityDirection == pager.direction
      let farEnough = pager.progress >= 0.28
      settleSpacePager(committing: farEnough || fastEnough)
  }

  private func settleSpacePager(committing: Bool) {
      guard var pager = spacePager else { return }
      let targetIndex = pager.targetIndex
      if committing,
         let targetIndex,
         host.spaces.indices.contains(targetIndex)
      {
          host.select(spaceID: host.spaces[targetIndex].spaceID)
      }

      pager.settlementPhase = committing ? .settlingToTarget : .settlingToSource
      pager.pageWidth = sidebarSwipePageWidth
      pager.dragOffset = committing ? -CGFloat(pager.direction) * sidebarSwipePageWidth : 0
      spacePagerToken += 1
      let token = spacePagerToken
      let duration = reduceMotion ? 0.12 : 0.28

      withAnimation(settleAnimation) {
          spacePager = pager
      }

      DispatchQueue.main.asyncAfter(deadline: .now() + duration) {
          guard spacePagerToken == token else { return }
          var transaction = Transaction()
          transaction.disablesAnimations = true
          withTransaction(transaction) {
              spacePager = nil
          }
      }
  }
  ```

- [ ] **Step 3: Add local pager helpers**

  Add these helpers inside `ShellSidebarView`:

  ```swift
  private var settleAnimation: Animation {
      if reduceMotion {
          return .easeOut(duration: 0.12)
      }
      return .interactiveSpring(response: 0.28, dampingFraction: 0.86, blendDuration: 0.04)
  }

  private var sidebarSwipePageWidth: CGFloat {
      max(spacePagerPageWidth, 1)
  }

  private func resistedEdgeOffset(for translationX: CGFloat) -> CGFloat {
      let edgeLimit = sidebarSwipePageWidth * 0.18
      let distance = abs(translationX)
      let resistedDistance = edgeLimit * distance / (distance + edgeLimit)
      return translationX < 0 ? -resistedDistance : resistedDistance
  }

  private func adjacentSpaceIndex(from sourceIndex: Int, direction: Int) -> Int? {
      let targetIndex = sourceIndex + direction
      guard host.spaces.indices.contains(targetIndex) else { return nil }
      return targetIndex
  }

  private var selectedSpaceIndex: Int? {
      guard let selectedSpaceID = host.selectedSpace?.spaceID else { return nil }
      return host.spaces.firstIndex { $0.spaceID == selectedSpaceID }
  }

  private var previewedSpaceID: String? {
      guard let targetIndex = spacePager?.targetIndex else { return nil }
      return spaceID(forSpaceAt: targetIndex)
  }

  private func spaceID(forSpaceAt index: Int) -> String? {
      guard host.spaces.indices.contains(index) else { return nil }
      return host.spaces[index].spaceID
  }

  private var isTabListScrollDisabled: Bool {
      spacePager != nil
  }
  ```

- [ ] **Step 4: Replace the static header/list with a content pager**

  Replace `spaceLabelRow` plus `tabSection` in `sidebarContent` with:

  ```swift
  spaceContentPager
  ```

  Add this view:

  ```swift
  private var spaceContentPager: some View {
      GeometryReader { proxy in
          let pageWidth = max(proxy.size.width, 1)
          ZStack(alignment: .leading) {
              ForEach(spacePageIndices, id: \.self) { index in
                  VStack(alignment: .leading, spacing: 0) {
                      spaceLabelRow(for: spaceID(forSpaceAt: index))
                          .padding(.bottom, 2)
                      tabSection(for: spaceID(forSpaceAt: index))
                  }
                  .frame(width: pageWidth, height: proxy.size.height, alignment: .topLeading)
                  .offset(x: spacePageOffset(for: index, pageWidth: pageWidth))
                  .allowsHitTesting(spacePager == nil && index == selectedSpaceIndex)
              }
          }
          .clipped()
          .onAppear {
              updateSpacePagerPageWidth(pageWidth)
          }
          .onChange(of: proxy.size.width) { _, width in
              updateSpacePagerPageWidth(max(width, 1))
          }
      }
      .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
  }
  ```

- [ ] **Step 5: Parameterize header and tab list by space id**

  Change:

  ```swift
  private var spaceLabelRow: some View
  private var tabSection: some View
  ```

  to:

  ```swift
  private func spaceLabelRow(for spaceID: String?) -> some View {
      ShellSidebarSpaceHeader(
          host: host,
          spaceID: spaceID
      )
      .frame(maxWidth: .infinity)
      .frame(height: 28)
  }

  private func tabSection(for spaceID: String?) -> some View {
      VStack(alignment: .leading, spacing: 0) {
          tabListPage(for: spaceID)
              .overlay(alignment: .top) {
                  if spaceID == sourceSpaceID {
                      ShellSidebarScrollBoundary(progress: tabListBoundaryProgress)
                  }
              }
              .clipped()
      }
      .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
  }
  ```

- [ ] **Step 6: Add page index and offset helpers**

  Add:

  ```swift
  private var spacePageIndices: [Int] {
      guard let spacePager else {
          return selectedSpaceIndex.map { [$0] } ?? []
      }
      return spacePager.pageIndicesForRendering.filter { host.spaces.indices.contains($0) }
  }

  private func spacePageOffset(for index: Int, pageWidth: CGFloat) -> CGFloat {
      guard var spacePager else { return 0 }
      spacePager.pageWidth = pageWidth
      return spacePager.offset(for: index)
  }

  private func updateSpacePagerPageWidth(_ pageWidth: CGFloat) {
      let clampedPageWidth = max(pageWidth, 1)
      spacePagerPageWidth = clampedPageWidth
      guard var spacePager,
            spacePager.pageWidth != clampedPageWidth
      else {
          return
      }
      spacePager.pageWidth = clampedPageWidth
      self.spacePager = spacePager
  }
  ```

- [ ] **Step 7: Wire the monitor locally**

  In `body`, replace:

  ```swift
  ShellSidebarSwipeMonitor(onUpdate: onSpaceSwipe)
  ```

  with:

  ```swift
  ShellSidebarSwipeMonitor(onUpdate: handleSpaceSwipe)
  ```

- [ ] **Step 8: Commit the sidebar pager move**

  Run:

  ```bash
  git add clients/apple/alan-macos/Views/Shell/ShellSidebarView.swift
  git commit -m "Move space swipe pager into sidebar"
  ```

### Task 4: Restore Stable Root Shell Layout

**Files:**
- Modify: `clients/apple/alan-macos/MacShellRootView.swift`
- Modify: `clients/apple/alan-macos/Views/Shell/ShellWorkspaceView.swift`

- [ ] **Step 1: Remove root pager state and functions**

  In `MacShellRootView.swift`, remove these state properties:

  ```swift
  @State private var isSpaceSwipeGestureLocked = false
  @State private var spacePager: ShellSpacePagerState?
  @State private var spacePagerToken = 0
  @State private var spacePagerPageWidth: CGFloat = 1
  @State private var spacePagerPageSelectedPaneIDs: [Int: String] = [:]
  ```

  Remove the root-level functions and computed properties that only support full-window paging:

  ```swift
  handleSpaceSwipe(_:)
  beginSpacePager()
  updateSpacePager(translationX:)
  finishSpacePager(velocityX:)
  settleSpacePager(committing:)
  sidebarSwipePageWidth
  resistedEdgeOffset(for:)
  adjacentSpaceIndex(from:direction:)
  selectedSpaceIndex
  previewedSpaceID
  swipeEnabledSpaceIndex
  floatingSidebarDisplaySpaceID
  spaceID(forSpaceAt:)
  firstPaneID(forSpaceAt:)
  selectedPaneID(forSpaceAt:)
  spacePagerPages
  spacePageIndices
  spacePage(index:pageWidth:)
  spacePageOffset(for:pageWidth:)
  updateSpacePagerPageWidth(_:)
  ```

- [ ] **Step 2: Replace full-window pages with stable HStack layout**

  In `body`, replace:

  ```swift
  spacePagerPages
      .frame(
          minWidth: ShellWindowSizing.minimumSize.width,
          minHeight: ShellWindowSizing.minimumSize.height
      )
  ```

  with:

  ```swift
  HStack(spacing: 0) {
      pinnedSidebarSurface()

      ShellWorkspaceView(
          host: host,
          expandedSidebarProgress: clampedPinnedSidebarPresentationProgress
      )
      .frame(maxWidth: .infinity, maxHeight: .infinity)
      .ignoresSafeArea(edges: .top)
  }
  .frame(
      minWidth: ShellWindowSizing.minimumSize.width,
      minHeight: ShellWindowSizing.minimumSize.height
  )
  ```

- [ ] **Step 3: Simplify sidebar surface calls**

  Change:

  ```swift
  private func pinnedSidebarSurface(
      displaySpaceID: String?,
      previewedSpaceID: String?,
      isSwipeEnabled: Bool
  ) -> some View
  ```

  to:

  ```swift
  private func pinnedSidebarSurface() -> some View
  ```

  Its body should call:

  ```swift
  sidebarContent(isSwipeEnabled: true)
  ```

  Change `floatingSidebarPanel` so it calls:

  ```swift
  sidebarContent(isSwipeEnabled: true)
  ```

  Remove `displaySpaceID` and `previewedSpaceID` arguments from `sidebarContent`.

- [ ] **Step 4: Simplify ShellSidebarView construction**

  In `sidebarContent`, construct:

  ```swift
  ShellSidebarView(
      host: host,
      chromeMetrics: windowChromeMetrics,
      displaySpaceID: nil,
      isSwipeEnabled: isSwipeEnabled
  ) {
      presentCommandInput()
  }
  ```

- [ ] **Step 5: Simplify ShellWorkspaceView if no longer needed**

  If `ShellWorkspaceView` no longer has call sites that pass `spaceID` or
  `selectedPaneID`, remove those parameters and keep:

  ```swift
  struct ShellWorkspaceView: View {
      @ObservedObject var host: ShellHostController
      let expandedSidebarProgress: CGFloat

      var body: some View {
          TerminalPaneView(
              host: host,
              tab: host.selectedTab,
              spaceID: host.selectedSpace?.spaceID,
              selectedPaneID: host.selectedPane?.paneID,
              terminalSurfaceInsets: ShellWorkspaceMetrics.terminalSurfaceInsets(
                  expandedSidebarProgress: expandedSidebarProgress
              )
          )
          .frame(maxWidth: .infinity, maxHeight: .infinity)
      }
  }
  ```

- [ ] **Step 6: Build to catch SwiftUI signature drift**

  Run:

  ```bash
  xcodebuild -project clients/apple/alan-macos.xcodeproj -scheme alan -configuration Debug -destination 'platform=macOS' build
  ```

  Expected: exits `0`.

- [ ] **Step 7: Commit root layout restoration**

  Run:

  ```bash
  git add clients/apple/alan-macos/MacShellRootView.swift clients/apple/alan-macos/Views/Shell/ShellWorkspaceView.swift
  git commit -m "Restore stable shell layout during space swipe"
  ```

### Task 5: Validate The Full Change

**Files:**
- Read: `clients/apple/scripts/check-shell-contracts.sh`
- Modify: `openspec/changes/refine-macos-sidebar-interactions/tasks.md`

- [ ] **Step 1: Run the sidebar swipe test**

  Run:

  ```bash
  clients/apple/scripts/test-shell-sidebar-swipe-monitor.sh
  ```

  Expected output includes:

  ```text
  Shell sidebar swipe monitor tests passed.
  ```

- [ ] **Step 2: Run shell contract checks**

  Run:

  ```bash
  clients/apple/scripts/check-shell-contracts.sh
  ```

  Expected: exits `0`.

- [ ] **Step 3: Validate OpenSpec**

  Run:

  ```bash
  openspec validate refine-macos-sidebar-interactions --strict
  openspec validate --all --strict
  ```

  Expected: both commands exit `0`.

- [ ] **Step 4: Build the macOS app**

  Run:

  ```bash
  xcodebuild -project clients/apple/alan-macos.xcodeproj -scheme alan -configuration Debug -destination 'platform=macOS' build
  ```

  Expected: exits `0`.

- [ ] **Step 5: Manually verify the running app**

  Launch or reuse the local app, then verify:

  ```text
  - horizontal swipe starts only from inside the sidebar;
  - only the space title/header and active tab list move with the finger;
  - command input stays fixed;
  - bottom space switcher stays fixed;
  - sidebar chrome and traffic lights stay fixed;
  - terminal workspace does not move, duplicate, or expose top-edge artifacts during drag;
  - release past threshold commits to the adjacent space;
  - release below threshold cancels back to the original space;
  - fast flick can commit from velocity;
  - vertical scrolling still works when the gesture is vertical.
  ```

- [ ] **Step 6: Update task completion state**

  In `openspec/changes/refine-macos-sidebar-interactions/tasks.md`, check off the updated pager and verification tasks that were completed in this implementation pass.

- [ ] **Step 7: Commit validation task updates**

  Run:

  ```bash
  git add openspec/changes/refine-macos-sidebar-interactions/tasks.md
  git commit -m "Mark sidebar swipe validation tasks complete"
  ```

### Self-review Notes

- Spec coverage: Task 1 fixes the incorrect OpenSpec abstraction; Tasks 2-4 implement the sidebar-local content pager and root layout stability; Task 5 covers automated and manual validation.
- Placeholder scan: this plan has no unresolved markers or unspecified test commands.
- Type consistency: the plan uses `ShellSidebarSpaceContentPagerState` and `ShellSidebarSpaceContentPagerSettlementPhase` consistently after extraction.
