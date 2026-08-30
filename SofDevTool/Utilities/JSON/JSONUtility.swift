import Foundation
import SwiftUI

enum JSONMode: String, CaseIterable, Codable, Identifiable {
    case format = "Format", minify = "Minify", query = "Query"
    var id: Self { self }
}

struct JSONOptions: Codable {
    var mode: JSONMode = .format
    var indentation = 2
    var sortedKeys = false
    var query = ""
}
struct JSONSnapshot: Codable {
    let input: String
    let output: String
    let options: JSONOptions
}

enum JSONEngine {
    static func transform(_ input: String, options: JSONOptions) throws -> String {
        let data = Data(input.utf8)
        let object: Any
        do { object = try JSONSerialization.jsonObject(with: data, options: [.fragmentsAllowed]) } catch {
            let nsError = error as NSError
            throw UtilityError(nsError.userInfo[NSDebugDescriptionErrorKey] as? String ?? "Invalid JSON.")
        }
        if options.mode == .query {
            let value = try query(object, path: options.query)
            if JSONSerialization.isValidJSONObject(value) {
                return String(
                    decoding: try JSONSerialization.data(
                        withJSONObject: value,
                        options: [.prettyPrinted, .sortedKeys, .withoutEscapingSlashes]), as: UTF8.self)
            }
            if value is NSNull { return "null" }
            if let text = value as? String { return text }
            return String(describing: value)
        }
        var writing: JSONSerialization.WritingOptions = [.fragmentsAllowed, .withoutEscapingSlashes]
        if options.mode == .format { writing.insert(.prettyPrinted) }
        if options.sortedKeys { writing.insert(.sortedKeys) }
        var output = String(
            decoding: try JSONSerialization.data(withJSONObject: object, options: writing), as: UTF8.self)
        if options.mode == .format, options.indentation == 4 {
            output = output.components(separatedBy: "\n").map { line in
                let spaces = line.prefix(while: { $0 == " " }).count
                return String(repeating: " ", count: spaces * 2) + line.dropFirst(spaces)
            }.joined(separator: "\n")
        }
        return output
    }

    private static func query(_ root: Any, path: String) throws -> Any {
        guard !path.isEmpty else { return root }
        let parts: [String]
        if path.hasPrefix("/") {
            parts = path.dropFirst().split(separator: "/", omittingEmptySubsequences: false).map {
                $0.replacingOccurrences(of: "~1", with: "/").replacingOccurrences(of: "~0", with: "~")
            }
        } else {
            parts = path.replacingOccurrences(of: "[", with: ".").replacingOccurrences(of: "]", with: "")
                .split(separator: ".").map(String.init)
        }
        return try parts.reduce(root) { current, part in
            if let object = current as? [String: Any], let next = object[part] { return next }
            if let array = current as? [Any], let index = Int(part), array.indices.contains(index) {
                return array[index]
            }
            throw UtilityError("Query path not found at ‘\(part)’.")
        }
    }
}

struct UtilityError: LocalizedError {
    let message: String
    init(_ message: String) { self.message = message }
    var errorDescription: String? { message }
}

struct JSONWorkspace: View {
    let context: UtilityWorkspaceContext
    @State private var input = ""
    @State private var output = ""
    @State private var diagnostic: String?
    @State private var options = JSONOptions()
    @State private var suppressNextRecord = false
    @State private var pendingRestore: UtilityHistoryEntry?

    var body: some View {
        VStack(spacing: 12) {
            HStack {
                Picker("Operation", selection: $options.mode) {
                    ForEach(JSONMode.allCases) { Text($0.rawValue).tag($0) }
                }.pickerStyle(.segmented)
                if options.mode == .format {
                    Picker("Indent", selection: $options.indentation) {
                        Text("2 spaces").tag(2)
                        Text("4 spaces").tag(4)
                    }.frame(width: 130)
                }
                Toggle("Sort keys", isOn: $options.sortedKeys)
            }
            if options.mode == .query {
                TextField("JSON Pointer or dot/bracket path", text: $options.query).textFieldStyle(
                    .roundedBorder)
            }
            HSplitView {
                TextEditorCard(title: "Input", text: $input, accessibilityID: "json.input")
                TextEditorCard(
                    title: "Result", text: $output, editable: false, accessibilityID: "json.result")
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
        }
        .padding(16)
        .task(id: operationKey) {
            try? await Task.sleep(for: .milliseconds(250))
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
            Text("Restoring this History entry replaces the current non-empty JSON session.")
        }
    }

    private var operationKey: String {
        input + "|\(options.mode.rawValue)|\(options.indentation)|\(options.sortedKeys)|\(options.query)"
    }
    private func evaluate() {
        guard !input.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
            output = ""
            diagnostic = nil
            return
        }
        do {
            output = try JSONEngine.transform(input, options: options)
            diagnostic = nil
            if suppressNextRecord {
                suppressNextRecord = false
            } else if let payload = try? JSONEncoder().encode(
                JSONSnapshot(input: input, output: output, options: options))
            {
                context.record(.init(utilityID: "json", schemaVersion: 1, payload: payload))
            }
        } catch {
            output = ""
            diagnostic = error.localizedDescription
        }
    }
    private func restore(_ entry: UtilityHistoryEntry?) {
        guard let entry, entry.utilityID == "json",
            let snapshot = try? JSONDecoder().decode(JSONSnapshot.self, from: entry.payload)
        else { return }
        suppressNextRecord = true
        input = snapshot.input
        output = snapshot.output
        options = snapshot.options
        diagnostic = nil
    }

    private func requestRestore(_ entry: UtilityHistoryEntry?) {
        guard let entry, entry.utilityID == "json" else { return }
        guard let snapshot = try? JSONDecoder().decode(JSONSnapshot.self, from: entry.payload) else { return }
        if !input.isEmpty, input != snapshot.input { pendingRestore = entry } else { restore(entry) }
    }
}
