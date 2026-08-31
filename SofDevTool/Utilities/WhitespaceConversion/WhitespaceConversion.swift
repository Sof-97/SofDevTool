import Foundation
import SwiftUI

enum WhitespaceConversionOperation: String, CaseIterable, Codable, Identifiable {
    case edgeTrim = "Edge Trim"
    case perLineTrim = "Per-line Trim"
    case collapseHorizontalWhitespace = "Collapse Horizontal Whitespace"
    case collapseAllWhitespace = "Collapse All Whitespace"
    case normalizeLineEndings = "Normalize Line Endings"
    case tabsToSpaces = "Tabs to Spaces"
    case spacesToTabs = "Spaces to Tabs"
    case removeBlankLines = "Remove Blank Lines"
    case dedent = "Dedent"

    var id: Self { self }
}

enum WhitespaceLineEnding: String, CaseIterable, Codable, Identifiable {
    case lf = "LF"
    case crlf = "CRLF"
    case cr = "CR"

    var id: Self { self }
    var value: String {
        switch self {
        case .lf: "\n"
        case .crlf: "\r\n"
        case .cr: "\r"
        }
    }
}

struct WhitespaceConversionSnapshot: Codable, Equatable {
    let input: String
    let output: String
    let operation: WhitespaceConversionOperation
    let lineEnding: WhitespaceLineEnding
    let tabWidth: Int
}

enum WhitespaceConversionEngine {
    static func transform(
        _ input: String,
        operation: WhitespaceConversionOperation,
        lineEnding: WhitespaceLineEnding = .lf,
        tabWidth: Int = 4
    ) throws -> String {
        guard (1...8).contains(tabWidth) else { throw UtilityError("Tab width must be from 1 through 8.") }
        switch operation {
        case .edgeTrim:
            return input.trimmingCharacters(in: .whitespacesAndNewlines)
        case .perLineTrim:
            return lines(in: input).map { line in
                line.content.trimmingCharacters(in: .whitespaces) + line.ending
            }.joined()
        case .collapseHorizontalWhitespace:
            return lines(in: input).map {
                collapse($0.content, includingLineEndings: false) + $0.ending
            }.joined()
        case .collapseAllWhitespace:
            return collapse(input, includingLineEndings: true)
        case .normalizeLineEndings:
            return lines(in: input).map { $0.content + ($0.ending.isEmpty ? "" : lineEnding.value) }.joined()
        case .tabsToSpaces:
            return expandTabs(input, width: tabWidth)
        case .spacesToTabs:
            return compressLeadingSpaces(input, width: tabWidth)
        case .removeBlankLines:
            return lines(in: input).filter { !$0.content.allSatisfy(\.isWhitespace) }.map {
                $0.content + $0.ending
            }.joined()
        case .dedent:
            return dedent(input, width: tabWidth)
        }
    }

    private struct Line {
        let content: String
        let ending: String
    }

    private static func lines(in input: String) -> [Line] {
        guard !input.isEmpty else { return [] }
        var result: [Line] = []
        var content = ""
        var index = input.unicodeScalars.startIndex
        while index < input.unicodeScalars.endIndex {
            let scalar = input.unicodeScalars[index]
            if scalar.value == 10 {
                result.append(Line(content: content, ending: "\n"))
                content = ""
                index = input.unicodeScalars.index(after: index)
            } else if scalar.value == 13 {
                let next = input.unicodeScalars.index(after: index)
                if next < input.unicodeScalars.endIndex, input.unicodeScalars[next].value == 10 {
                    result.append(Line(content: content, ending: "\r\n"))
                    index = input.unicodeScalars.index(after: next)
                } else {
                    result.append(Line(content: content, ending: "\r"))
                    index = next
                }
                content = ""
            } else {
                content.unicodeScalars.append(scalar)
                index = input.unicodeScalars.index(after: index)
            }
        }
        if !content.isEmpty { result.append(Line(content: content, ending: "")) }
        return result
    }

    private static func collapse(_ input: String, includingLineEndings: Bool) -> String {
        var result = ""
        var inRun = false
        for character in input {
            let isLineEnding = character == "\n" || character == "\r"
            let shouldCollapse = character.isWhitespace && (includingLineEndings || !isLineEnding)
            if shouldCollapse {
                if !inRun { result.append(" ") }
                inRun = true
            } else {
                result.append(character)
                inRun = false
            }
        }
        return result
    }

    private static func expandTabs(_ input: String, width: Int) -> String {
        lines(in: input).map { expandTabs(in: $0.content, width: width) + $0.ending }.joined()
    }

    private static func expandTabs(in input: String, width: Int) -> String {
        var result = ""
        var column = 0
        for character in input {
            switch character {
            case "\t":
                let count = width - column % width
                result += String(repeating: " ", count: count)
                column += count
            default:
                result.append(character)
                column += 1
            }
        }
        return result
    }

    private static func compressLeadingSpaces(_ input: String, width: Int) -> String {
        lines(in: input).map { line in
            var result = ""
            var column = 0
            var index = line.content.startIndex
            while index < line.content.endIndex {
                let character = line.content[index]
                if character == "\t" {
                    result.append(character)
                    column += width - column % width
                    index = line.content.index(after: index)
                } else if character == " " {
                    let stop = width - column % width
                    var cursor = index
                    var available = 0
                    while cursor < line.content.endIndex, line.content[cursor] == " ", available < stop {
                        available += 1
                        cursor = line.content.index(after: cursor)
                    }
                    if available == stop {
                        result.append("\t")
                        column += stop
                    } else {
                        result += String(repeating: " ", count: available)
                        column += available
                    }
                    index = cursor
                } else {
                    break
                }
            }
            result += line.content[index...]
            return result + line.ending
        }.joined()
    }

    private static func dedent(_ input: String, width: Int) -> String {
        let parsed = lines(in: input)
        let nonBlank = parsed.filter { !$0.content.allSatisfy(\.isWhitespace) }
        guard !nonBlank.isEmpty else { return input }
        let removable = nonBlank.map { indentationWidth($0.content, tabWidth: width) }.min() ?? 0
        guard removable > 0 else { return input }
        return parsed.map { line in
            guard !line.content.allSatisfy(\.isWhitespace) else { return line.content + line.ending }
            return removingIndent(removable, from: line.content, tabWidth: width) + line.ending
        }.joined()
    }

    private static func indentationWidth(_ value: String, tabWidth: Int) -> Int {
        var column = 0
        for character in value {
            if character == " " {
                column += 1
            } else if character == "\t" {
                column += tabWidth - column % tabWidth
            } else {
                break
            }
        }
        return column
    }

    private static func removingIndent(_ amount: Int, from value: String, tabWidth: Int) -> String {
        var removed = 0
        var index = value.startIndex
        var replacement = ""
        while index < value.endIndex, removed < amount {
            let character = value[index]
            let advance: Int
            if character == " " {
                advance = 1
            } else if character == "\t" {
                advance = tabWidth - removed % tabWidth
            } else {
                break
            }
            if removed + advance > amount {
                replacement = String(repeating: " ", count: removed + advance - amount)
            }
            removed += advance
            index = value.index(after: index)
        }
        return replacement + value[index...]
    }
}

struct WhitespaceConversionWorkspace: View {
    let context: UtilityWorkspaceContext
    @State private var input = ""
    @State private var output = ""
    @State private var operation = WhitespaceConversionOperation.edgeTrim
    @State private var lineEnding = WhitespaceLineEnding.lf
    @State private var tabWidth = 4
    @State private var diagnostic: String?
    @State private var pendingRestore: UtilityHistoryEntry?

    var body: some View {
        VStack(spacing: 12) {
            HStack {
                Picker("Operation", selection: operationBinding) {
                    ForEach(WhitespaceConversionOperation.allCases) { Text($0.rawValue).tag($0) }
                }
                if operation == .normalizeLineEndings {
                    Picker("Line ending", selection: lineEndingBinding) {
                        ForEach(WhitespaceLineEnding.allCases) { Text($0.rawValue).tag($0) }
                    }.frame(width: 130)
                }
                if operation == .tabsToSpaces || operation == .spacesToTabs || operation == .dedent {
                    Stepper("Tab width: \(tabWidth)", value: tabWidthBinding, in: 1...8)
                }
            }
            HSplitView {
                TextEditorCard(
                    title: "Input", text: inputBinding, accessibilityID: "whitespace-conversion.input")
                TextEditorCard(
                    title: "Result", text: $output, editable: false,
                    accessibilityID: "whitespace-conversion.result")
            }
            if let diagnostic { DiagnosticBanner(message: diagnostic) }
            HStack {
                Button("Paste") { inputBinding.wrappedValue = context.clipboard.readText() ?? input }
                    .accessibilityIdentifier("whitespace-conversion.paste")
                Button("Clear") {
                    input = ""
                    output = ""
                    diagnostic = nil
                }
                Button("Transform") { transform() }.keyboardShortcut(.return, modifiers: .command).disabled(
                    input.isEmpty
                ).accessibilityIdentifier("whitespace-conversion.transform")
                Spacer()
                ConfirmedCopyButton(title: "Copy Result", isEnabled: !output.isEmpty) {
                    context.clipboard.writeText(output)
                }
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
            Text("Restoring this History entry replaces the current whitespace session.")
        }
    }

    private var inputBinding: Binding<String> {
        Binding(
            get: { input },
            set: {
                input = $0
                invalidateOutput()
            })
    }
    private var operationBinding: Binding<WhitespaceConversionOperation> {
        Binding(
            get: { operation },
            set: {
                operation = $0
                invalidateOutput()
            })
    }
    private var lineEndingBinding: Binding<WhitespaceLineEnding> {
        Binding(
            get: { lineEnding },
            set: {
                lineEnding = $0
                invalidateOutput()
            })
    }
    private var tabWidthBinding: Binding<Int> {
        Binding(
            get: { tabWidth },
            set: {
                tabWidth = $0
                invalidateOutput()
            })
    }
    private func invalidateOutput() {
        output = ""
        diagnostic = nil
    }

    private func transform() {
        do {
            output = try WhitespaceConversionEngine.transform(
                input, operation: operation, lineEnding: lineEnding, tabWidth: tabWidth)
            diagnostic = nil
            let snapshot = WhitespaceConversionSnapshot(
                input: input, output: output, operation: operation, lineEnding: lineEnding, tabWidth: tabWidth
            )
            if let payload = try? JSONEncoder().encode(snapshot) {
                context.record(.init(utilityID: "whitespace-conversion", schemaVersion: 1, payload: payload))
            }
        } catch {
            output = ""
            diagnostic = error.localizedDescription
        }
    }

    private func restore(_ entry: UtilityHistoryEntry?) {
        guard let entry, entry.utilityID == "whitespace-conversion",
            let snapshot = try? JSONDecoder().decode(WhitespaceConversionSnapshot.self, from: entry.payload)
        else { return }
        input = snapshot.input
        output = snapshot.output
        operation = snapshot.operation
        lineEnding = snapshot.lineEnding
        tabWidth = snapshot.tabWidth
        diagnostic = nil
    }

    private func requestRestore(_ entry: UtilityHistoryEntry?) {
        guard let entry, entry.utilityID == "whitespace-conversion",
            let snapshot = try? JSONDecoder().decode(WhitespaceConversionSnapshot.self, from: entry.payload)
        else { return }
        if !input.isEmpty, input != snapshot.input { pendingRestore = entry } else { restore(entry) }
    }
}
