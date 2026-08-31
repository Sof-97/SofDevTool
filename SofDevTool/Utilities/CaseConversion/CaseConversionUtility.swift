import Foundation
import SwiftUI

enum CaseConversionStyle: String, CaseIterable, Codable, Identifiable {
    case camel = "camelCase"
    case pascal = "PascalCase"
    case snake = "snake_case"
    case screamingSnake = "SCREAMING_SNAKE_CASE"
    case kebab = "kebab-case"
    case title = "Title Case"
    case sentence = "sentence case"
    case lowercase = "lowercase"
    case uppercase = "UPPERCASE"

    var id: Self { self }
}

struct CaseConversionResult: Codable, Equatable {
    let words: [String]
    let output: String
}

struct CaseConversionPolicy: Equatable {
    static let production = CaseConversionPolicy(maximumInputUTF16Length: 1_048_576)
    let maximumInputUTF16Length: Int
}

enum CaseConversionEngine {
    private static let locale = Locale(identifier: "en_US_POSIX")

    static func convert(
        _ input: String,
        to style: CaseConversionStyle,
        policy: CaseConversionPolicy = .production
    ) throws -> CaseConversionResult {
        guard input.utf16.count <= policy.maximumInputUTF16Length else {
            throw UtilityError("Input exceeds the 1,048,576 UTF-16-unit conversion limit.")
        }
        let words = segment(input)
        return CaseConversionResult(words: words, output: render(words, as: style))
    }

    static func segment(_ input: String) -> [String] {
        var words: [String] = []
        var current: [Character] = []
        var currentKinds: [CharacterKind] = []

        func flush(_ keepingLast: Bool = false) {
            let kept = keepingLast ? current.suffix(1) : []
            let keptKinds = keepingLast ? currentKinds.suffix(1) : []
            let emitted = keepingLast ? current.dropLast() : current[...]
            if !emitted.isEmpty { words.append(String(emitted)) }
            current = Array(kept)
            currentKinds = Array(keptKinds)
        }

        for character in input {
            let kind = classify(character)
            guard kind != .separator else {
                flush()
                continue
            }
            if kind == .symbol {
                flush()
                words.append(String(character))
                continue
            }
            if let previous = currentKinds.last {
                if (previous == .lowercase || previous == .uncasedLetter) && kind == .uppercase
                    || previous == .digit && kind != .digit
                    || previous != .digit && kind == .digit
                {
                    flush()
                } else if previous == .uppercase, kind == .lowercase,
                    currentKinds.dropLast().last == .uppercase
                {
                    flush(true)
                }
            }
            current.append(character)
            currentKinds.append(kind)
        }
        flush()
        return words
    }

    private static func render(_ words: [String], as style: CaseConversionStyle) -> String {
        let lower = words.map { $0.lowercased(with: locale) }
        let capitalized = lower.map(capitalize)
        switch style {
        case .camel:
            return (Array(lower.prefix(1)) + Array(capitalized.dropFirst())).joined()
        case .pascal:
            return capitalized.joined()
        case .snake:
            return lower.joined(separator: "_")
        case .screamingSnake:
            return words.map { $0.uppercased(with: locale) }.joined(separator: "_")
        case .kebab:
            return lower.joined(separator: "-")
        case .title:
            return capitalized.joined(separator: " ")
        case .sentence:
            guard let first = lower.first else { return "" }
            return ([capitalize(first)] + lower.dropFirst()).joined(separator: " ")
        case .lowercase:
            return lower.joined(separator: " ")
        case .uppercase:
            return words.map { $0.uppercased(with: locale) }.joined(separator: " ")
        }
    }

    private static func capitalize(_ word: String) -> String {
        guard let first = word.first else { return word }
        return String(first).uppercased(with: locale) + word.dropFirst()
    }

    private enum CharacterKind: Equatable {
        case uppercase, lowercase, uncasedLetter, digit, separator, symbol
    }

    private static func classify(_ character: Character) -> CharacterKind {
        let scalars = character.unicodeScalars
        if scalars.contains(where: CharacterSet.decimalDigits.contains) { return .digit }
        if scalars.contains(where: CharacterSet.uppercaseLetters.contains) { return .uppercase }
        if scalars.contains(where: CharacterSet.lowercaseLetters.contains) { return .lowercase }
        if scalars.contains(where: CharacterSet.letters.contains) { return .uncasedLetter }
        if scalars.allSatisfy({
            CharacterSet.whitespacesAndNewlines.contains($0)
                || CharacterSet.punctuationCharacters.contains($0)
                || CharacterSet.symbols.contains($0) && !$0.properties.isEmojiPresentation
        }) {
            return .separator
        }
        return .symbol
    }
}

struct CaseConversionSnapshot: Codable, Equatable {
    let input: String
    let style: CaseConversionStyle
    let result: CaseConversionResult
}

struct CaseConversionRevisionGate {
    private(set) var current = 0
    mutating func begin() -> Int {
        current += 1
        return current
    }
    func accepts(_ revision: Int) -> Bool { revision == current }
}

struct CaseConversionWorkspace: View {
    let context: UtilityWorkspaceContext
    @State private var input = ""
    @State private var style = CaseConversionStyle.camel
    @State private var result: CaseConversionResult?
    @State private var diagnostic: String?
    @State private var skipNextEvaluation = false
    @State private var pendingRestore: UtilityHistoryEntry?
    @State private var revisionGate = CaseConversionRevisionGate()

    var body: some View {
        VStack(spacing: 12) {
            Picker("Case", selection: $style) {
                ForEach(CaseConversionStyle.allCases) { Text($0.rawValue).tag($0) }
            }
            .accessibilityIdentifier("case-conversion.style")

            HSplitView {
                TextEditorCard(title: "Input", text: $input, accessibilityID: "case-conversion.input")
                TextEditorCard(
                    title: "Result", text: .constant(result?.output ?? ""), editable: false,
                    accessibilityID: "case-conversion.result"
                )
            }
            VStack(alignment: .leading, spacing: 4) {
                Text("Detected Words").font(.caption).foregroundStyle(.secondary)
                Text(result?.words.joined(separator: " · ") ?? "—")
                    .textSelection(.enabled)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .accessibilityIdentifier("case-conversion.detected-words")
                Text(
                    "Acronym runs, letter/digit transitions, punctuation, mixed separators, and emoji boundaries are explicit; casing uses en_US_POSIX."
                )
                .font(.caption2).foregroundStyle(.secondary)
            }
            if let diagnostic { DiagnosticBanner(message: diagnostic) }
            CopyPasteActions(
                canCopy: !(result?.output.isEmpty ?? true),
                paste: { input = context.clipboard.readText() ?? input },
                copy: { if let result { context.clipboard.writeText(result.output) } },
                clear: {
                    input = ""
                    result = nil
                    diagnostic = nil
                }
            )
        }
        .padding(16)
        .task(id: operationKey) {
            let revision = revisionGate.begin()
            try? await Task.sleep(for: .milliseconds(200))
            guard !Task.isCancelled, revisionGate.accepts(revision) else { return }
            if skipNextEvaluation {
                skipNextEvaluation = false
                return
            }
            evaluate(revision: revision)
        }
        .onReceive(NotificationCenter.default.publisher(for: .restoreUtilitySnapshot)) {
            requestRestore($0.object as? UtilityHistoryEntry)
        }
        .alert(
            "Replace current workspace?",
            isPresented: Binding(
                get: { pendingRestore != nil }, set: { if !$0 { pendingRestore = nil } }
            )
        ) {
            Button("Restore", role: .destructive) {
                let entry = pendingRestore
                pendingRestore = nil
                restore(entry)
            }
            Button("Cancel", role: .cancel) { pendingRestore = nil }
        } message: {
            Text("Restoring this History entry replaces the current non-empty Case Conversion session.")
        }
    }

    private var operationKey: String { input + "|" + style.rawValue }

    private func evaluate(revision: Int) {
        guard !input.isEmpty else {
            result = nil
            diagnostic = nil
            return
        }
        do {
            let converted = try CaseConversionEngine.convert(input, to: style)
            guard revisionGate.accepts(revision) else { return }
            result = converted
            diagnostic = nil
            if let payload = try? JSONEncoder().encode(
                CaseConversionSnapshot(input: input, style: style, result: converted)
            ) {
                context.record(.init(utilityID: "case-conversion", schemaVersion: 1, payload: payload))
            }
        } catch {
            result = nil
            diagnostic = error.localizedDescription
        }
    }

    private func restore(_ entry: UtilityHistoryEntry?) {
        guard let entry, entry.utilityID == "case-conversion",
            let snapshot = try? JSONDecoder().decode(CaseConversionSnapshot.self, from: entry.payload)
        else { return }
        skipNextEvaluation = true
        input = snapshot.input
        style = snapshot.style
        result = snapshot.result
        diagnostic = nil
    }

    private func requestRestore(_ entry: UtilityHistoryEntry?) {
        guard let entry, entry.utilityID == "case-conversion",
            let snapshot = try? JSONDecoder().decode(CaseConversionSnapshot.self, from: entry.payload)
        else { return }
        if !input.isEmpty, input != snapshot.input { pendingRestore = entry } else { restore(entry) }
    }
}
