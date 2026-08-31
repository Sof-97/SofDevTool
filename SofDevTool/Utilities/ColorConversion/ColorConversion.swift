import AppKit
import Foundation
import SwiftUI

struct SRGBColor: Codable, Equatable {
    let red: Double
    let green: Double
    let blue: Double
    let alpha: Double

    init(red: Double, green: Double, blue: Double, alpha: Double = 1) {
        self.red = Self.quantize(red)
        self.green = Self.quantize(green)
        self.blue = Self.quantize(blue)
        self.alpha = Self.quantize(alpha)
    }

    private static func quantize(_ value: Double) -> Double { Double((value * 255).rounded()) / 255 }
}

struct ColorConversionOutputs: Codable, Equatable {
    let hex: String
    let rgb: String
    let hsl: String
}

struct ColorConversionSnapshot: Codable, Equatable {
    let source: String
    let color: SRGBColor
    let outputs: ColorConversionOutputs
}

enum ColorConversionEngine {
    /// All accepted values are normalized to 8-bit sRGB channels. CSS decimals use at most four places,
    /// sufficient to parse every emitted representation back to the same normalized color.
    static func parse(_ source: String) throws -> SRGBColor {
        let value = source.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !value.isEmpty else { throw UtilityError("Enter a HEX, RGB, RGBA, HSL, or HSLA color.") }
        if value.hasPrefix("#") { return try parseHex(value) }
        guard let open = value.firstIndex(of: "("), value.last == ")" else {
            throw UtilityError("Use #RGB, #RGBA, #RRGGBB, #RRGGBBAA, rgb/rgba, or hsl/hsla syntax.")
        }
        let name = value[..<open].lowercased()
        let arguments = String(value[value.index(after: open)..<value.index(before: value.endIndex)])
        switch name {
        case "rgb", "rgba": return try parseRGB(arguments)
        case "hsl", "hsla": return try parseHSL(arguments)
        default: throw UtilityError("Unsupported color function. Use rgb, rgba, hsl, or hsla.")
        }
    }

    static func outputs(for color: SRGBColor) -> ColorConversionOutputs {
        let channels = [color.red, color.green, color.blue, color.alpha].map { Int(($0 * 255).rounded()) }
        let hex = channels.prefix(color.alpha == 1 ? 3 : 4).map { String(format: "%02x", $0) }.joined()
        let alpha = color.alpha == 1 ? "" : " / \(decimal(color.alpha))"
        let hsl = rgbToHSL(color)
        return ColorConversionOutputs(
            hex: "#\(hex)",
            rgb: "rgb(\(channels[0]) \(channels[1]) \(channels[2])\(alpha))",
            hsl:
                "hsl(\(decimal(hsl.hue))deg \(decimal(hsl.saturation * 100))% \(decimal(hsl.lightness * 100))%\(alpha))"
        )
    }

    private static func parseHex(_ value: String) throws -> SRGBColor {
        let digits = String(value.dropFirst())
        guard [3, 4, 6, 8].contains(digits.count), digits.allSatisfy(\.isHexDigit) else {
            throw UtilityError("HEX must contain 3, 4, 6, or 8 hexadecimal digits.")
        }
        let expanded = digits.count <= 4 ? digits.map { "\($0)\($0)" }.joined() : digits
        func channel(_ offset: Int) -> Double {
            let start = expanded.index(expanded.startIndex, offsetBy: offset)
            return Double(Int(expanded[start..<expanded.index(start, offsetBy: 2)], radix: 16)!) / 255
        }
        return SRGBColor(
            red: channel(0), green: channel(2), blue: channel(4), alpha: expanded.count == 8 ? channel(6) : 1)
    }

    private static func parseRGB(_ arguments: String) throws -> SRGBColor {
        let parts = components(arguments)
        guard parts.values.count == 3 else {
            throw UtilityError("RGB requires exactly three color components.")
        }
        let usesPercent = parts.values.allSatisfy { $0.hasSuffix("%") }
        guard usesPercent || parts.values.allSatisfy({ !$0.hasSuffix("%") }) else {
            throw UtilityError("RGB components must be all integers or all percentages.")
        }
        let divisor = usesPercent ? 100.0 : 255.0
        let rgb = try parts.values.map { token -> Double in
            let raw = usesPercent ? String(token.dropLast()) : token
            let number = try finite(raw, named: "RGB component")
            guard (0...divisor).contains(number), usesPercent || number.rounded() == number else {
                throw UtilityError(
                    usesPercent
                        ? "RGB percentages must be from 0% through 100%."
                        : "RGB integer components must be from 0 through 255.")
            }
            return number / divisor
        }
        return SRGBColor(red: rgb[0], green: rgb[1], blue: rgb[2], alpha: try alpha(parts.alpha))
    }

    private static func parseHSL(_ arguments: String) throws -> SRGBColor {
        let parts = components(arguments)
        guard parts.values.count == 3 else {
            throw UtilityError("HSL requires hue, saturation, and lightness.")
        }
        let hueToken = parts.values[0].lowercased()
        let hue = try finite(
            hueToken.hasSuffix("deg") ? String(hueToken.dropLast(3)) : hueToken, named: "Hue")
        guard parts.values[1].hasSuffix("%"), parts.values[2].hasSuffix("%") else {
            throw UtilityError("HSL saturation and lightness require percentages.")
        }
        let saturation = try finite(String(parts.values[1].dropLast()), named: "Saturation")
        let lightness = try finite(String(parts.values[2].dropLast()), named: "Lightness")
        guard (0...100).contains(saturation), (0...100).contains(lightness) else {
            throw UtilityError("HSL saturation and lightness must be from 0% through 100%.")
        }
        let normalizedHue =
            ((hue.truncatingRemainder(dividingBy: 360)) + 360).truncatingRemainder(dividingBy: 360) / 360
        let s = saturation / 100, l = lightness / 100
        let c = (1 - abs(2 * l - 1)) * s
        let h = normalizedHue * 6
        let x = c * (1 - abs(h.truncatingRemainder(dividingBy: 2) - 1))
        let tuple: (Double, Double, Double)
        switch h {
        case 0..<1: tuple = (c, x, 0)
        case 1..<2: tuple = (x, c, 0)
        case 2..<3: tuple = (0, c, x)
        case 3..<4: tuple = (0, x, c)
        case 4..<5: tuple = (x, 0, c)
        default: tuple = (c, 0, x)
        }
        let m = l - c / 2
        return SRGBColor(
            red: tuple.0 + m, green: tuple.1 + m, blue: tuple.2 + m, alpha: try alpha(parts.alpha))
    }

    private static func components(_ input: String) -> (values: [String], alpha: String?) {
        let slashParts = input.split(separator: "/", omittingEmptySubsequences: false).map(String.init)
        guard slashParts.count <= 2 else { return ([], nil) }
        var values =
            slashParts[0].contains(",")
            ? slashParts[0].split(separator: ",").map { $0.trimmingCharacters(in: .whitespaces) }
            : slashParts[0].split(whereSeparator: \.isWhitespace).map(String.init)
        var alphaValue = slashParts.count == 2 ? slashParts[1].trimmingCharacters(in: .whitespaces) : nil
        if values.count == 4, alphaValue == nil { alphaValue = values.removeLast() }
        return (values, alphaValue)
    }

    private static func alpha(_ token: String?) throws -> Double {
        guard let token else { return 1 }
        let percent = token.hasSuffix("%")
        let number = try finite(percent ? String(token.dropLast()) : token, named: "Alpha")
        let maximum = percent ? 100.0 : 1.0
        guard (0...maximum).contains(number) else {
            throw UtilityError(
                percent ? "Alpha must be from 0% through 100%." : "Alpha must be from 0 through 1.")
        }
        return number / maximum
    }

    private static func finite(_ token: String, named name: String) throws -> Double {
        guard let value = Double(token), value.isFinite else {
            throw UtilityError("\(name) must be a finite number.")
        }
        return value
    }

    private static func rgbToHSL(_ color: SRGBColor) -> (hue: Double, saturation: Double, lightness: Double) {
        let maximum = max(color.red, color.green, color.blue),
            minimum = min(color.red, color.green, color.blue)
        let delta = maximum - minimum, lightness = (maximum + minimum) / 2
        guard delta != 0 else { return (0, 0, lightness) }
        let saturation = delta / (1 - abs(2 * lightness - 1))
        let raw: Double
        if maximum == color.red {
            raw = ((color.green - color.blue) / delta).truncatingRemainder(dividingBy: 6)
        } else if maximum == color.green {
            raw = (color.blue - color.red) / delta + 2
        } else {
            raw = (color.red - color.green) / delta + 4
        }
        return ((((raw * 60) + 360).truncatingRemainder(dividingBy: 360)), saturation, lightness)
    }

    private static func decimal(_ value: Double) -> String {
        var result = String(format: "%.4f", locale: Locale(identifier: "en_US_POSIX"), value)
        while result.contains("."), result.last == "0" { result.removeLast() }
        if result.last == "." { result.removeLast() }
        return result == "-0" ? "0" : result
    }
}

struct ColorConversionWorkspace: View {
    let context: UtilityWorkspaceContext
    @State private var source = ""
    @State private var color = SRGBColor(red: 1, green: 1, blue: 1)
    @State private var outputs: ColorConversionOutputs?
    @State private var diagnostic: String?
    @State private var pendingRestore: UtilityHistoryEntry?

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text(
                "Bounded CSS syntax: HEX, integer or percentage RGB(A), and degree/percentage HSL(A). Values normalize to 8-bit sRGB; wide gamut is out of scope."
            ).font(.caption).foregroundStyle(.secondary)
            HStack {
                TextField("#rgb, rgb(...), or hsl(...)", text: sourceBinding).textFieldStyle(.roundedBorder)
                    .accessibilityIdentifier("color-conversion.input")
                ColorPicker("sRGB picker", selection: pickerBinding, supportsOpacity: true).frame(width: 130)
                    .accessibilityIdentifier("color-conversion.picker")
                Button("Convert") { convertSource() }.keyboardShortcut(.return, modifiers: .command).disabled(
                    source.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
            }
            if let outputs {
                outputRow("HEX", outputs.hex)
                outputRow("RGB", outputs.rgb)
                outputRow("HSL", outputs.hsl)
            } else {
                ContentUnavailableView(
                    "Enter a color", systemImage: "paintpalette",
                    description: Text("Conversion stays entirely on this Mac."))
            }
            if let diagnostic { DiagnosticBanner(message: diagnostic) }
            HStack {
                Button("Paste") { sourceBinding.wrappedValue = context.clipboard.readText() ?? source }
                Button("Clear") {
                    source = ""
                    outputs = nil
                    diagnostic = nil
                }
                Spacer()
            }
        }
        .padding(16)
        .onReceive(NotificationCenter.default.publisher(for: .restoreUtilitySnapshot)) {
            requestRestore($0.object as? UtilityHistoryEntry)
        }
        .alert(
            "Replace current workspace?",
            isPresented: Binding(get: { pendingRestore != nil }, set: { if !$0 { pendingRestore = nil } })
        ) {
            Button("Restore", role: .destructive) {
                let entry = pendingRestore
                pendingRestore = nil
                restore(entry)
            }
            Button("Cancel", role: .cancel) { pendingRestore = nil }
        } message: {
            Text("Restoring this History entry replaces the current color session.")
        }
    }

    private var sourceBinding: Binding<String> {
        Binding(
            get: { source },
            set: {
                source = $0
                outputs = nil
                diagnostic = nil
            })
    }

    private func outputRow(_ label: String, _ value: String) -> some View {
        HStack {
            Text(label).frame(width: 45, alignment: .leading)
            Text(value).textSelection(.enabled)
            Spacer()
            ConfirmedCopyButton(title: "Copy \(label)", isEnabled: true) {
                context.clipboard.writeText(value)
            }
        }
    }

    private var pickerBinding: Binding<Color> {
        Binding(
            get: { Color(.sRGB, red: color.red, green: color.green, blue: color.blue, opacity: color.alpha) },
            set: { selected in
                guard let converted = NSColor(selected).usingColorSpace(.sRGB) else { return }
                let next = SRGBColor(
                    red: converted.redComponent, green: converted.greenComponent,
                    blue: converted.blueComponent, alpha: converted.alphaComponent)
                guard next != color else { return }
                color = next
                source = ColorConversionEngine.outputs(for: next).hex
                publish(next)
            })
    }

    private func convertSource() {
        do {
            let parsed = try ColorConversionEngine.parse(source)
            color = parsed
            publish(parsed)
        } catch {
            outputs = nil
            diagnostic = error.localizedDescription
        }
    }

    private func publish(_ parsed: SRGBColor) {
        let nextOutputs = ColorConversionEngine.outputs(for: parsed)
        outputs = nextOutputs
        diagnostic = nil
        let snapshot = ColorConversionSnapshot(source: source, color: parsed, outputs: nextOutputs)
        if let payload = try? JSONEncoder().encode(snapshot) {
            context.record(.init(utilityID: "color-conversion", schemaVersion: 1, payload: payload))
        }
    }

    private func restore(_ entry: UtilityHistoryEntry?) {
        guard let entry, entry.utilityID == "color-conversion",
            let snapshot = try? JSONDecoder().decode(ColorConversionSnapshot.self, from: entry.payload)
        else { return }
        source = snapshot.source
        color = snapshot.color
        outputs = snapshot.outputs
        diagnostic = nil
    }

    private func requestRestore(_ entry: UtilityHistoryEntry?) {
        guard let entry, entry.utilityID == "color-conversion",
            let snapshot = try? JSONDecoder().decode(ColorConversionSnapshot.self, from: entry.payload)
        else { return }
        if !source.isEmpty, source != snapshot.source { pendingRestore = entry } else { restore(entry) }
    }
}
