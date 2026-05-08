import AppKit
import SwiftUI

#if os(macOS)
private extension Color {
    static func shellAdaptive(
        light: (Double, Double, Double),
        dark: (Double, Double, Double),
        alpha: Double = 1
    ) -> Color {
        shellAdaptive(light: light, lightAlpha: alpha, dark: dark, darkAlpha: alpha)
    }

    static func shellAdaptive(
        light: (Double, Double, Double),
        lightAlpha: Double,
        dark: (Double, Double, Double),
        darkAlpha: Double
    ) -> Color {
        Color(
            NSColor(name: nil) { appearance in
                let isDark = appearance.bestMatch(from: [.darkAqua, .aqua]) == .darkAqua
                let rgb = isDark ? dark : light
                let alpha = isDark ? darkAlpha : lightAlpha
                return NSColor(red: rgb.0, green: rgb.1, blue: rgb.2, alpha: alpha)
            }
        )
    }
}

enum ShellPalette {
    static let canvas = Color.shellAdaptive(
        light: (0.94, 0.94, 0.965),
        dark: (0.045, 0.050, 0.062)
    )
    static let window = Color.shellAdaptive(
        light: (0.972, 0.973, 0.985),
        dark: (0.055, 0.061, 0.074)
    )
    static let sidebar = Color.shellAdaptive(
        light: (0.922, 0.924, 0.953),
        dark: (0.071, 0.079, 0.096)
    )
    static let sidebarRail = Color.shellAdaptive(
        light: (0.902, 0.907, 0.941),
        dark: (0.083, 0.092, 0.112)
    )
    static let sidebarCard = Color.shellAdaptive(
        light: (0.98, 0.98, 0.995),
        lightAlpha: 1.0,
        dark: (0.172, 0.188, 0.224),
        darkAlpha: 0.92
    )
    static let sidebarSelection = Color.shellAdaptive(
        light: (1.0, 1.0, 1.0),
        lightAlpha: 0.15,
        dark: (0.205, 0.225, 0.270),
        darkAlpha: 0.72
    )
    static let sidebarHover = Color.shellAdaptive(
        light: (1.0, 1.0, 1.0),
        lightAlpha: 0.08,
        dark: (0.185, 0.205, 0.245),
        darkAlpha: 0.46
    )
    static let sidebarControl = Color.shellAdaptive(
        light: (1.0, 1.0, 1.0),
        lightAlpha: 0.12,
        dark: (0.190, 0.210, 0.252),
        darkAlpha: 0.54
    )
    static let sidebarControlStrong = Color.shellAdaptive(
        light: (1.0, 1.0, 1.0),
        lightAlpha: 0.18,
        dark: (0.215, 0.235, 0.282),
        darkAlpha: 0.72
    )
    static let railBase = Color.shellAdaptive(
        light: (1.0, 1.0, 1.0),
        lightAlpha: 0.08,
        dark: (0.155, 0.172, 0.210),
        darkAlpha: 0.58
    )
    static let railHover = Color.shellAdaptive(
        light: (1.0, 1.0, 1.0),
        lightAlpha: 0.14,
        dark: (0.190, 0.210, 0.252),
        darkAlpha: 0.66
    )
    static let railSelection = Color.shellAdaptive(
        light: (1.0, 1.0, 1.0),
        lightAlpha: 0.22,
        dark: (0.235, 0.255, 0.310),
        darkAlpha: 0.78
    )
    static let workspace = Color.shellAdaptive(
        light: (0.979, 0.98, 0.989),
        dark: (0.050, 0.056, 0.070)
    )
    static let terminal = Color.shellAdaptive(
        light: (0.10, 0.12, 0.16),
        dark: (0.050, 0.061, 0.076)
    )
    static let terminalSoft = Color.shellAdaptive(
        light: (0.16, 0.18, 0.24),
        dark: (0.100, 0.116, 0.145)
    )
    static let accent = Color.shellAdaptive(
        light: (0.31, 0.39, 0.71),
        dark: (0.50, 0.60, 0.94)
    )
    static let accentSoft = Color.shellAdaptive(
        light: (0.90, 0.92, 0.98),
        dark: (0.18, 0.22, 0.34)
    )
    static let ink = Color.shellAdaptive(
        light: (0.16, 0.18, 0.24),
        dark: (0.90, 0.92, 0.96)
    )
    static let mutedInk = Color.shellAdaptive(
        light: (0.43, 0.45, 0.54),
        dark: (0.64, 0.68, 0.75)
    )
    static let line = Color.shellAdaptive(
        light: (0.82, 0.83, 0.89),
        dark: (0.255, 0.285, 0.345)
    )
    static let panel = Color.shellAdaptive(
        light: (1.0, 1.0, 1.0),
        lightAlpha: 0.74,
        dark: (0.135, 0.150, 0.180),
        darkAlpha: 0.78
    )
    static let panelSoft = Color.shellAdaptive(
        light: (1.0, 1.0, 1.0),
        lightAlpha: 0.60,
        dark: (0.125, 0.140, 0.170),
        darkAlpha: 0.64
    )
    static let materialScrim = Color.shellAdaptive(
        light: (0.922, 0.924, 0.953),
        lightAlpha: 0.35,
        dark: (0.030, 0.037, 0.050),
        darkAlpha: 0.78
    )
    static let materialTopWash = Color.shellAdaptive(
        light: (1.0, 1.0, 1.0),
        lightAlpha: 0.06,
        dark: (0.130, 0.150, 0.205),
        darkAlpha: 0.18
    )
    static let materialBottomShade = Color.shellAdaptive(
        light: (0.84, 0.86, 0.92),
        lightAlpha: 0.04,
        dark: (0.012, 0.016, 0.024),
        darkAlpha: 0.34
    )
    static let attention = Color.shellAdaptive(
        light: (0.82, 0.55, 0.24),
        dark: (0.94, 0.68, 0.34)
    )
}

enum ShellRadii {
    static let control: CGFloat = 6
    static let row: CGFloat = 8
    static let surface: CGFloat = 10
    static let overlay: CGFloat = 12
}

private struct SidebarMaterialView: NSViewRepresentable {
    func makeNSView(context: Context) -> NSVisualEffectView {
        let view = NSVisualEffectView()
        view.material = .sidebar
        view.blendingMode = .behindWindow
        view.state = .followsWindowActiveState
        return view
    }

    func updateNSView(_ nsView: NSVisualEffectView, context: Context) {}
}

struct ShellMaterialBackgroundView: View {
    var body: some View {
        ZStack {
            SidebarMaterialView()
            ShellPalette.materialScrim
            LinearGradient(
                colors: [
                    ShellPalette.materialTopWash,
                    ShellPalette.materialBottomShade,
                ],
                startPoint: .topLeading,
                endPoint: .bottomTrailing
            )
        }
    }
}
#endif
