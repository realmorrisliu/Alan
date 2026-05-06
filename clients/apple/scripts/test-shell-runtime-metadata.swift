import Darwin
import Foundation

#if os(macOS)
@main
struct ShellRuntimeMetadataTestRunner {
    static func main() async {
        await MainActor.run {
            ShellRuntimeMetadataTests.run()
        }
    }
}

@MainActor
private enum ShellRuntimeMetadataTests {
    static func run() {
        verifiesRuntimeProjectsTerminalStatusIntoPaneMetadata()
        verifiesTerminalStatusSummaryPrioritizesExitAndRendererHealth()
        print("Shell runtime metadata tests passed.")
    }

    private static func verifiesRuntimeProjectsTerminalStatusIntoPaneMetadata() {
        let controller = makeController()
        guard let pane = controller.selectedPane else {
            fail("bootstrap shell must expose a selected pane")
        }

        controller.updateTerminalRuntime(
            TerminalHostRuntimeSnapshot(
                stage: .windowAttached,
                paneID: pane.paneID,
                tabID: pane.tabID,
                logicalSize: .zero,
                backingSize: .zero,
                displayName: "Studio Display",
                displayID: "display_1",
                attachedWindowTitle: "Alan",
                isFocused: false,
                renderer: TerminalRendererSnapshot(
                    kind: .ghosttyLive,
                    phase: .failed,
                    summary: "renderer failed",
                    detail: "lost drawable",
                    failureReason: "lost device",
                    recentEvents: ["device lost"]
                ),
                paneMetadata: TerminalPaneMetadataSnapshot(
                    title: "vim main.rs",
                    workingDirectory: "/Users/morris/Developer/Alan",
                    summary: "terminal bell",
                    attention: .notable,
                    processExited: false,
                    lastCommandExitCode: nil,
                    lastUpdatedAt: Date(timeIntervalSince1970: 1_000)
                ),
                surfaceState: AlanTerminalSurfaceStateSnapshot(
                    readiness: .unready(reason: .rendererFailed),
                    terminalMode: .normalBuffer,
                    scrollback: .empty,
                    search: nil,
                    readonly: false,
                    secureInput: false,
                    inputReady: false,
                    rendererHealth: "failed",
                    childExited: false,
                    lastUpdatedAt: Date(timeIntervalSince1970: 1_001)
                ),
                lastUpdatedAt: Date(timeIntervalSince1970: 1_002)
            )
        )

        let updated = controller.shellState.panes.first { $0.paneID == pane.paneID }
        expect(updated?.context?.rendererHealth == "failed", "pane context must record renderer health")
        expect(updated?.context?.surfaceReadiness == "renderer_failed", "pane context must record surface readiness")
        expect(updated?.context?.inputReady == false, "pane context must record input readiness")
        expect(updated?.context?.terminalMode == "normal_buffer", "pane context must record terminal mode")
        expect(updated?.viewport?.title == "vim main.rs", "pane viewport must record terminal title")
        expect(updated?.viewport?.summary == "Renderer failed", "pane viewport must expose renderer status")
        expect(updated?.attention == .notable, "pane attention must reflect terminal attention")
        expect(controller.shellState.spaces.first?.attention == .notable, "space attention must track pane attention")
    }

    private static func verifiesTerminalStatusSummaryPrioritizesExitAndRendererHealth() {
        let exited = pane(
            context: context(
                processState: "exited",
                rendererHealth: "ready",
                surfaceReadiness: "child_exited",
                lastCommandExitCode: 2
            ),
            viewport: ShellViewportSnapshot(
                title: "fish",
                summary: "terminal bell",
                visibleExcerpt: nil,
                lastActivityAt: nil
            ),
            attention: .awaitingUser
        )
        expect(shellTerminalStatusSummary(for: exited) == "Exited 2", "exit status must outrank cwd or generic summaries")

        let failedRenderer = pane(
            context: context(
                processState: "running",
                rendererHealth: "failed",
                surfaceReadiness: "renderer_failed",
                lastCommandExitCode: nil
            ),
            viewport: ShellViewportSnapshot(
                title: "fish",
                summary: "terminal bell",
                visibleExcerpt: nil,
                lastActivityAt: nil
            ),
            attention: .notable
        )
        expect(shellTerminalStatusSummary(for: failedRenderer) == "Renderer failed", "renderer failure must outrank generic summaries")

        let ordinary = pane(
            context: context(
                processState: "running",
                rendererHealth: "ready",
                surfaceReadiness: "ready",
                lastCommandExitCode: nil
            ),
            viewport: ShellViewportSnapshot(
                title: "fish",
                summary: "idle shell",
                visibleExcerpt: nil,
                lastActivityAt: nil
            ),
            attention: .idle
        )
        expect(shellTerminalStatusSummary(for: ordinary) == nil, "ordinary summaries must not hide cwd or branch metadata")
    }

    private static func makeController() -> ShellHostController {
        let windowID = "metadata_test_\(UUID().uuidString)"
        let registry = TerminalRuntimeRegistry(runtimeService: FakeAlanTerminalRuntimeService())
        let context = ShellWindowContext.make(
            windowID: windowID,
            terminalRuntimeRegistry: registry
        )
        let persistenceURL = FileManager.default.temporaryDirectory
            .appendingPathComponent("\(windowID).json")
        return ShellHostController(
            shellState: .bootstrapDefault(windowID: windowID),
            windowContext: context,
            persistenceURL: persistenceURL,
            terminalRuntimeRegistry: registry
        )
    }

    private static func pane(
        context: ShellContextSnapshot,
        viewport: ShellViewportSnapshot?,
        attention: ShellAttentionState
    ) -> ShellPane {
        ShellPane(
            paneID: "pane_1",
            tabID: "tab_1",
            spaceID: "space_1",
            launchTarget: .shell,
            cwd: "/Users/morris/Developer/Alan",
            process: ShellProcessBinding(program: "fish", argvPreview: nil),
            attention: attention,
            context: context,
            viewport: viewport,
            alanBinding: nil
        )
    }

    private static func context(
        processState: String,
        rendererHealth: String,
        surfaceReadiness: String,
        lastCommandExitCode: Int?
    ) -> ShellContextSnapshot {
        ShellContextSnapshot(
            workingDirectoryName: "Alan",
            repositoryRoot: nil,
            gitBranch: nil,
            controlPath: nil,
            alanBindingFile: nil,
            launchStrategy: nil,
            shellIntegrationSource: "ghostty_shell_integration",
            processState: processState,
            rendererHealth: rendererHealth,
            surfaceReadiness: surfaceReadiness,
            inputReady: surfaceReadiness == "ready",
            readonly: false,
            terminalMode: "normal_buffer",
            lastMetadataAt: nil,
            lastCommandExitCode: lastCommandExitCode
        )
    }

    private static func expect(
        _ condition: @autoclosure () -> Bool,
        _ message: String
    ) {
        guard condition() else {
            fail(message)
        }
    }

    private static func fail(_ message: String) -> Never {
        fputs("error: \(message)\n", stderr)
        exit(1)
    }
}
#endif
