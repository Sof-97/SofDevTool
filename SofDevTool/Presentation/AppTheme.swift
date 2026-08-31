import SwiftUI

enum AppTheme: String, CaseIterable, Identifiable, Sendable {
    case graphite
    case frappe

    static let defaultsKey = "appearance.theme"

    var id: Self { self }

    var displayName: String {
        switch self {
        case .graphite: "Graphite"
        case .frappe: "Catppuccin Frappé"
        }
    }

    var palette: SemanticThemePalette {
        switch self {
        case .graphite: .graphite
        case .frappe: .frappe
        }
    }
}

struct ThemeColor: Equatable, Sendable {
    let hex: UInt32

    var color: Color {
        Color(
            red: Double((hex >> 16) & 0xFF) / 255,
            green: Double((hex >> 8) & 0xFF) / 255,
            blue: Double(hex & 0xFF) / 255
        )
    }
}

struct SemanticThemePalette: Equatable, Sendable {
    let windowBackground: ThemeColor
    let secondaryPane: ThemeColor
    let raisedSurface: ThemeColor
    let editorSurface: ThemeColor
    let separator: ThemeColor
    let primaryText: ThemeColor
    let secondaryText: ThemeColor
    let subtleText: ThemeColor
    let selection: ThemeColor
    let appAccent: ThemeColor
    let secondaryAccent: ThemeColor
    let success: ThemeColor
    let warning: ThemeColor
    let error: ThemeColor
    let codeText: ThemeColor

    static let graphite = SemanticThemePalette(
        windowBackground: ThemeColor(hex: 0x09_0B_10),
        secondaryPane: ThemeColor(hex: 0x11_15_1C),
        raisedSurface: ThemeColor(hex: 0x17_1C_25),
        editorSurface: ThemeColor(hex: 0x0D_11_18),
        separator: ThemeColor(hex: 0x69_71_82),
        primaryText: ThemeColor(hex: 0xF3_F5_F8),
        secondaryText: ThemeColor(hex: 0x90_98_A8),
        subtleText: ThemeColor(hex: 0x69_71_82),
        selection: ThemeColor(hex: 0x7C_83_FF),
        appAccent: ThemeColor(hex: 0x7C_83_FF),
        secondaryAccent: ThemeColor(hex: 0x4B_D6_D0),
        success: ThemeColor(hex: 0x4B_D1_8B),
        warning: ThemeColor(hex: 0xFF_B4_54),
        error: ThemeColor(hex: 0xFF_6B_7A),
        codeText: ThemeColor(hex: 0xF3_F5_F8)
    )

    // Catppuccin Frappé palette v1.8.0, mapped to app-owned semantic roles.
    static let frappe = SemanticThemePalette(
        windowBackground: ThemeColor(hex: 0x30_34_46),
        secondaryPane: ThemeColor(hex: 0x29_2C_3C),
        raisedSurface: ThemeColor(hex: 0x41_45_59),
        editorSurface: ThemeColor(hex: 0x23_26_34),
        separator: ThemeColor(hex: 0x51_57_6D),
        primaryText: ThemeColor(hex: 0xC6_D0_F5),
        secondaryText: ThemeColor(hex: 0xA5_AD_CE),
        subtleText: ThemeColor(hex: 0x83_8B_A7),
        selection: ThemeColor(hex: 0xBA_BB_F1),
        appAccent: ThemeColor(hex: 0xCA_9E_E6),
        secondaryAccent: ThemeColor(hex: 0x81_C8_BE),
        success: ThemeColor(hex: 0xA6_D1_89),
        warning: ThemeColor(hex: 0xE5_C8_90),
        error: ThemeColor(hex: 0xE7_82_84),
        codeText: ThemeColor(hex: 0xB5_BF_E2)
    )
}

private struct AppThemeEnvironmentKey: EnvironmentKey {
    static let defaultValue = AppTheme.graphite
}

extension EnvironmentValues {
    var appTheme: AppTheme {
        get { self[AppThemeEnvironmentKey.self] }
        set { self[AppThemeEnvironmentKey.self] = newValue }
    }

    var themePalette: SemanticThemePalette { appTheme.palette }
}
