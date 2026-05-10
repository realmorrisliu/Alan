import SwiftUI
#if os(iOS)
import UIKit
#elseif os(macOS)
import AppKit
#endif

extension Color {
    static func consoleAdaptive(
        light: (Double, Double, Double),
        dark: (Double, Double, Double),
        alpha: Double = 1
    ) -> Color {
        #if os(iOS)
        return Color(
            UIColor { traitCollection in
                let rgb = traitCollection.userInterfaceStyle == .dark ? dark : light
                return UIColor(red: rgb.0, green: rgb.1, blue: rgb.2, alpha: alpha)
            }
        )
        #elseif os(macOS)
        return Color(
            NSColor(name: nil) { appearance in
                let isDark = appearance.bestMatch(from: [.darkAqua, .aqua]) == .darkAqua
                let rgb = isDark ? dark : light
                return NSColor(red: rgb.0, green: rgb.1, blue: rgb.2, alpha: alpha)
            }
        )
        #else
        return Color(red: dark.0, green: dark.1, blue: dark.2, opacity: alpha)
        #endif
    }
}
