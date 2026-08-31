import Foundation
import SwiftUI

enum Base64Mode: String, CaseIterable, Codable, Identifiable {
    case encode = "Encode", decode = "Decode"
    var id: Self { self }
}
enum Base64Alphabet: String, CaseIterable, Codable, Identifiable {
    case standard = "Standard", urlSafe = "URL-safe"
    var id: Self { self }
}
struct Base64Options: Codable {
    var mode: Base64Mode = .encode
    var alphabet: Base64Alphabet = .standard
    var padded = true
}
struct Base64Snapshot: Codable {
    let input: String
    let output: String
    let options: Base64Options
}

enum Base64Engine {
    static func transform(_ input: String, options: Base64Options) throws -> String {
        if options.mode == .encode {
            var result = Data(input.utf8).base64EncodedString()
            if options.alphabet == .urlSafe {
                result = result.replacingOccurrences(of: "+", with: "-").replacingOccurrences(
                    of: "/", with: "_")
            }
            if !options.padded { result = result.replacingOccurrences(of: "=", with: "") }
            return result
        }
        let allowed =
            options.alphabet == .standard
            ? CharacterSet(charactersIn: "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/=")
            : CharacterSet(charactersIn: "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_=")
        guard input.unicodeScalars.allSatisfy(allowed.contains), !input.contains(where: \.isWhitespace) else {
            throw UtilityError("Input contains characters outside the selected Base64 alphabet.")
        }
        var normalized = input
        if options.alphabet == .urlSafe {
            normalized = normalized.replacingOccurrences(of: "-", with: "+").replacingOccurrences(
                of: "_", with: "/")
        }
        let remainder = normalized.count % 4
        guard remainder != 1 else { throw UtilityError("Invalid Base64 length.") }
        if remainder > 0 { normalized += String(repeating: "=", count: 4 - remainder) }
        guard let data = Data(base64Encoded: normalized, options: []),
            let result = String(data: data, encoding: .utf8)
        else { throw UtilityError("Input is not valid Base64-encoded UTF-8 text.") }
        return result
    }
}

struct Base64Workspace: View {
    let context: UtilityWorkspaceContext
    @State private var input = ""
    @State private var output = ""
    @State private var diagnostic: String?
    @State private var options = Base64Options()
    @State private var suppressNextRecord = false
    @State private var pendingRestore: UtilityHistoryEntry?

    var body: some View {
        VStack(spacing: 12) {
            HStack {
                Picker("Operation", selection: $options.mode) {
                    ForEach(Base64Mode.allCases) { Text($0.rawValue).tag($0) }
                }.pickerStyle(.segmented).accessibilityIdentifier("base64.mode")
                Picker("Alphabet", selection: $options.alphabet) {
                    ForEach(Base64Alphabet.allCases) { Text($0.rawValue).tag($0) }
                }.frame(width: 150).accessibilityIdentifier("base64.alphabet")
                Toggle("Padding", isOn: $options.padded).accessibilityIdentifier("base64.padding")
            }
            HSplitView {
                TextEditorCard(title: "UTF-8 Input", text: $input, accessibilityID: "base64.input")
                TextEditorCard(
                    title: "Result", text: $output, editable: false, accessibilityID: "base64.result")
            }
            if let diagnostic { DiagnosticBanner(message: diagnostic) }
            CopyPasteActions(
                canCopy: !output.isEmpty, paste: { input = context.clipboard.readText() ?? input },
                copy: { context.clipboard.writeText(output) },
                clear: {
                    input = ""
                    output = ""
                    diagnostic = nil
                })
        }.padding(16)
            .task(id: operationKey) {
                try? await Task.sleep(for: .milliseconds(200))
                guard !Task.isCancelled else { return }
                evaluate()
            }
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
                Text("Restoring this History entry replaces the current non-empty Base64 session.")
            }
    }

    private var operationKey: String {
        input + options.mode.rawValue + options.alphabet.rawValue + String(options.padded)
    }
    private func evaluate() {
        guard !input.isEmpty else {
            output = ""
            diagnostic = nil
            return
        }
        do {
            output = try Base64Engine.transform(input, options: options)
            diagnostic = nil
            if suppressNextRecord {
                suppressNextRecord = false
            } else if let payload = try? JSONEncoder().encode(
                Base64Snapshot(input: input, output: output, options: options))
            {
                context.record(.init(utilityID: "base64", schemaVersion: 1, payload: payload))
            }
        } catch {
            output = ""
            diagnostic = error.localizedDescription
        }
    }
    private func restore(_ entry: UtilityHistoryEntry?) {
        guard let entry, entry.utilityID == "base64",
            let snapshot = try? JSONDecoder().decode(Base64Snapshot.self, from: entry.payload)
        else { return }
        suppressNextRecord = true
        input = snapshot.input
        output = snapshot.output
        options = snapshot.options
        diagnostic = nil
    }

    private func requestRestore(_ entry: UtilityHistoryEntry?) {
        guard let entry, entry.utilityID == "base64",
            let snapshot = try? JSONDecoder().decode(Base64Snapshot.self, from: entry.payload)
        else { return }
        if !input.isEmpty, input != snapshot.input { pendingRestore = entry } else { restore(entry) }
    }
}
