import AppKit
import SwiftUI

/// Color roles the Appearance preferences expose, matching the reference
/// editor's picker list. Stored as hex strings in UserDefaults; the syntax
/// highlighter and editor chrome read them on every change.
enum AppearanceColorRole: String, CaseIterable, Identifiable {
    case editorBackground
    case gutterBackground
    case lineNumbers
    case bodyText
    case commands
    case environments
    case math
    case braces
    case comments
    case foldAccent
    case bracketMatch

    var id: Self { self }

    var titleKey: LocalizedStringKey {
        switch self {
        case .editorBackground: "appearance.color.editor_background"
        case .gutterBackground: "appearance.color.gutter_background"
        case .lineNumbers: "appearance.color.line_numbers"
        case .bodyText: "appearance.color.body_text"
        case .commands: "appearance.color.commands"
        case .environments: "appearance.color.environments"
        case .math: "appearance.color.math"
        case .braces: "appearance.color.braces"
        case .comments: "appearance.color.comments"
        case .foldAccent: "appearance.color.fold_accent"
        case .bracketMatch: "appearance.color.bracket_match"
        }
    }

    var detailKey: LocalizedStringKey {
        switch self {
        case .editorBackground: "appearance.color.editor_background_d"
        case .gutterBackground: "appearance.color.gutter_background_d"
        case .lineNumbers: "appearance.color.line_numbers_d"
        case .bodyText: "appearance.color.body_text_d"
        case .commands: "appearance.color.commands_d"
        case .environments: "appearance.color.environments_d"
        case .math: "appearance.color.math_d"
        case .braces: "appearance.color.braces_d"
        case .comments: "appearance.color.comments_d"
        case .foldAccent: "appearance.color.fold_accent_d"
        case .bracketMatch: "appearance.color.bracket_match_d"
        }
    }

    /// Palette used when no custom value is stored — the reference editor's
    /// built-in Dark theme, sampled from rendered pixels.
    var defaultHex: String {
        AppearanceThemePreset.dark.hexValue(for: self)
    }
}

/// Built-in palettes mirroring the reference editor's theme picker. Each
/// preset carries eleven roles in the same order the reference stores them.
/// `bracketMatch` hexes may carry an alpha suffix (RRGGBBAA) — the reference
/// stores the wash as a translucent accent (e.g. `66D9EF33` = 20%).
enum AppearanceThemePreset: String, CaseIterable, Identifiable {
    case light
    case dark
    case monokai
    case dracula
    case nord
    case oneDark
    case solarizedDark
    case solarizedLight
    case gruvbox
    case catppuccinLatte
    case catppuccinFrappe
    case catppuccinMacchiato
    case catppuccinMocha

    var id: Self { self }

    /// Display name in the settings theme picker — proper nouns, not
    /// localized, matching the reference editor's labels.
    var title: String {
        switch self {
        case .light: "Light"
        case .dark: "Dark"
        case .monokai: "Monokai"
        case .dracula: "Dracula"
        case .nord: "Nord"
        case .oneDark: "One Dark"
        case .solarizedDark: "Solarized Dark"
        case .solarizedLight: "Solarized Light"
        case .gruvbox: "Gruvbox"
        case .catppuccinLatte: "Catppuccin Latte"
        case .catppuccinFrappe: "Catppuccin Frappé"
        case .catppuccinMacchiato: "Catppuccin Macchiato"
        case .catppuccinMocha: "Catppuccin Mocha"
        }
    }

    /// Which app-wide appearance the preset implies.
    var isDark: Bool {
        switch self {
        case .light, .solarizedLight, .catppuccinLatte: false
        default: true
        }
    }

    /// Role order: editorBg, gutterBg, lineNumbers, body, commands,
    /// environments, math, braces, comments, foldAccent, bracketMatch.
    private var palette: [String] {
        switch self {
        case .light:
            ["#FFFFFF", "#F6F8FA", "#BABBBD", "#24292E", "#D73A49",
             "#6F42C1", "#005CC5", "#24292E", "#6A737D", "#0366D6", "#0366D622"]
        case .dark:
            ["#2C2C33", "#3C3D42", "#9E9E9E", "#E0E0DA", "#77C3F3",
             "#9FD790", "#D0A5E3", "#9E9D92", "#929D92", "#FF902F", "#6DB6E566"]
        case .monokai:
            ["#272822", "#2E2F28", "#90908A", "#F8F8F2", "#F92672",
             "#A6E22E", "#AE81FF", "#F8F8F2", "#75715E", "#66D9EF", "#66D9EF33"]
        case .dracula:
            ["#282A36", "#21222C", "#6272A4", "#F8F8F2", "#FF79C6",
             "#8BE9FD", "#BD93F9", "#F8F8F2", "#6272A4", "#BD93F9", "#BD93F933"]
        case .nord:
            ["#2E3440", "#3B4252", "#4C566A", "#D8DEE9", "#81A1C1",
             "#8FBCBB", "#B48EAD", "#ECEFF4", "#616E88", "#88C0D0", "#88C0D033"]
        case .oneDark:
            ["#282C34", "#2C313A", "#4B5263", "#ABB2BF", "#C678DD",
             "#E5C07B", "#D19A66", "#ABB2BF", "#5C6370", "#61AFEF", "#61AFEF33"]
        case .solarizedDark:
            ["#002B36", "#073642", "#586E75", "#839496", "#268BD2",
             "#2AA198", "#D33682", "#93A1A1", "#586E75", "#B58900", "#268BD233"]
        case .solarizedLight:
            ["#FDF6E3", "#EEE8D5", "#93A1A1", "#657B83", "#268BD2",
             "#2AA198", "#D33682", "#586E75", "#93A1A1", "#B58900", "#268BD226"]
        case .gruvbox:
            ["#282828", "#3C3836", "#7C6F64", "#EBDBB2", "#FB4934",
             "#8EC07C", "#D3869B", "#EBDBB2", "#928374", "#FE8019", "#83A59833"]
        case .catppuccinLatte:
            ["#EFF1F5", "#E6E9EF", "#8C8FA1", "#4C4F69", "#1E66F5",
             "#40A02B", "#8839EF", "#DF8E1D", "#7C7F93", "#FE640B", "#1E66F533"]
        case .catppuccinFrappe:
            ["#303446", "#292C3C", "#737994", "#C6D0F5", "#8CAAEE",
             "#A6D189", "#CA9EE6", "#E5C890", "#949CBB", "#EF9F76", "#8CAAEE33"]
        case .catppuccinMacchiato:
            ["#24273A", "#1E2030", "#6E738D", "#CAD3F5", "#8AADF4",
             "#A6DA95", "#C6A0F6", "#EED49F", "#939AB7", "#F5A97F", "#8AADF433"]
        case .catppuccinMocha:
            ["#1E1E2E", "#181825", "#6C7086", "#CDD6F4", "#89B4FA",
             "#A6E3A1", "#CBA6F7", "#F9E2AF", "#9399B2", "#FAB387", "#89B4FA33"]
        }
    }

    func hexValue(for role: AppearanceColorRole) -> String {
        palette[Self.roleOrder.firstIndex(of: role) ?? 0]
    }

    private static let roleOrder: [AppearanceColorRole] = [
        .editorBackground, .gutterBackground, .lineNumbers, .bodyText,
        .commands, .environments, .math, .braces, .comments,
        .foldAccent, .bracketMatch,
    ]
}

/// Appearance preferences shared by Settings and the editor surface. Kept
/// out of the portable PersistedSettings package on purpose — these are
/// macOS-native surface concerns stored flat in UserDefaults.
@MainActor
final class AppearanceSettings: ObservableObject {
    static let shared = AppearanceSettings()

    enum Theme: String, CaseIterable, Identifiable {
        case system
        case light
        case dark
        var id: Self { self }
    }

    /// UI language override. `.system` follows macOS; the others pin
    /// `AppleLanguages` so the app loads that `.lproj` on next launch.
    enum AppLanguage: String, CaseIterable, Identifiable {
        case system
        case en
        case ko
        case ja
        case vi
        var id: Self { self }
        /// Language pickers conventionally show each option in its own
        /// language; `.system` is localized at the call site instead.
        var displayName: String {
            switch self {
            case .system: return ""
            case .en: return "English"
            case .ko: return "한국어"
            case .ja: return "日本語"
            case .vi: return "Tiếng Việt"
            }
        }
    }

    @Published var theme: Theme {
        didSet { UserDefaults.standard.set(theme.rawValue, forKey: "appearance.theme"); applyAppearance() }
    }
    @Published var language: AppLanguage {
        didSet {
            UserDefaults.standard.set(language.rawValue, forKey: "appearance.language")
            applyLanguage()
            languageRestartPending = language != launchedLanguage
        }
    }
    /// True while the chosen language differs from the localization the
    /// bundle resolved at launch — `Bundle.main` caches its table, so a
    /// change only takes effect on the next launch.
    @Published private(set) var languageRestartPending = false
    private let launchedLanguage: AppLanguage
    @Published var fontFamily: String {
        didSet { UserDefaults.standard.set(fontFamily, forKey: "appearance.fontFamily") }
    }
    @Published var fontSize: Double {
        didSet { UserDefaults.standard.set(fontSize, forKey: "appearance.fontSize") }
    }
    /// Bumped whenever any color changes so observers can re-apply.
    @Published private(set) var colorRevision = 0

    /// The preset whose palette matches the stored colors exactly, or nil
    /// when the user has customised individual swatches ("Custom" in the
    /// picker, mirroring the reference editor's Theme row).
    var matchingPreset: AppearanceThemePreset? {
        AppearanceThemePreset.allCases.first { preset in
            AppearanceColorRole.allCases.allSatisfy { role in
                storedHex(for: role).caseInsensitiveCompare(preset.hexValue(for: role)) == .orderedSame
            }
        }
    }

    /// Writes the preset's palette over every role color and switches the
    /// app-wide appearance to the preset's light/dark side.
    func applyPreset(_ preset: AppearanceThemePreset) {
        for role in AppearanceColorRole.allCases {
            UserDefaults.standard.set(preset.hexValue(for: role), forKey: "appearance.color.\(role.rawValue)")
        }
        theme = preset.isDark ? .dark : .light
        colorRevision += 1
    }

    /// The stored hex for a role, or its theme default when unset.
    func storedHex(for role: AppearanceColorRole) -> String {
        UserDefaults.standard.string(forKey: "appearance.color.\(role.rawValue)") ?? role.defaultHex
    }

    private init() {
        let storedLanguage = AppLanguage(rawValue: UserDefaults.standard.string(forKey: "appearance.language") ?? "system") ?? .system
        launchedLanguage = storedLanguage
        language = storedLanguage
        theme = Theme(rawValue: UserDefaults.standard.string(forKey: "appearance.theme") ?? "system") ?? .system
        fontFamily = UserDefaults.standard.string(forKey: "appearance.fontFamily") ?? ""
        fontSize = UserDefaults.standard.object(forKey: "appearance.fontSize") as? Double ?? 13
        applyAppearance()
        applyLanguage()
    }

    func color(for role: AppearanceColorRole) -> NSColor {
        if let hex = UserDefaults.standard.string(forKey: "appearance.color.\(role.rawValue)"),
           let color = NSColor(hexString: hex) {
            return color
        }
        return NSColor(hexString: role.defaultHex) ?? .textColor
    }

    func setColor(_ color: NSColor, for role: AppearanceColorRole) {
        if let hex = color.hexString {
            UserDefaults.standard.set(hex, forKey: "appearance.color.\(role.rawValue)")
        }
        colorRevision += 1
    }

    /// The editor font resolved from family+size; family empty = system mono.
    var editorFont: NSFont {
        if !fontFamily.isEmpty,
           let font = NSFont(name: fontFamily, size: fontSize) {
            return font
        }
        return NSFont.monospacedSystemFont(ofSize: fontSize, weight: .regular)
    }

    /// Installs the app-wide light/dark appearance for the theme choice.
    func applyAppearance() {
        switch theme {
        case .system: NSApp.appearance = nil
        case .light: NSApp.appearance = NSAppearance(named: .aqua)
        case .dark: NSApp.appearance = NSAppearance(named: .darkAqua)
        }
    }

    /// Pins (or clears) `AppleLanguages` in the app's defaults. `Bundle.main`
    /// resolves its `.lproj` table once at first access, so this takes
    /// effect on the next launch — `languageRestartPending` drives the
    /// settings hint.
    func applyLanguage() {
        if language == .system {
            UserDefaults.standard.removeObject(forKey: "AppleLanguages")
        } else {
            UserDefaults.standard.set([language.rawValue], forKey: "AppleLanguages")
        }
    }
}

extension NSColor {
    /// Parses `#RRGGBB` or `#RRGGBBAA` (the reference palettes encode the
    /// bracket-match wash with an alpha suffix).
    convenience init?(hexString: String) {
        var hex = hexString.trimmingCharacters(in: .whitespacesAndNewlines)
        if hex.hasPrefix("#") { hex.removeFirst() }
        guard hex.count == 6 || hex.count == 8, var value = UInt64(hex, radix: 16) else { return nil }
        var alpha: UInt64 = 255
        if hex.count == 8 {
            alpha = value & 0xFF
            value >>= 8
        }
        self.init(
            red: CGFloat((value >> 16) & 0xFF) / 255,
            green: CGFloat((value >> 8) & 0xFF) / 255,
            blue: CGFloat(value & 0xFF) / 255,
            alpha: CGFloat(alpha) / 255
        )
    }

    var hexString: String? {
        guard let rgb = usingColorSpace(.deviceRGB) else { return nil }
        let r = Int(round(rgb.redComponent * 255))
        let g = Int(round(rgb.greenComponent * 255))
        let b = Int(round(rgb.blueComponent * 255))
        let a = Int(round(rgb.alphaComponent * 255))
        if a < 255 {
            return String(format: "#%02X%02X%02X%02X", r, g, b, a)
        }
        return String(format: "#%02X%02X%02X", r, g, b)
    }
}
