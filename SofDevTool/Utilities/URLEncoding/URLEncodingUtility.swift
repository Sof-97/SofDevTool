import Foundation
import SwiftUI

enum URLEncodingDirection: String, CaseIterable, Codable, Identifiable {
    case encode = "Encode"
    case decode = "Decode"

    var id: Self { self }
}

enum URLEncodingMode: String, CaseIterable, Codable, Identifiable {
    case pathSegment = "Path Segment"
    case queryValue = "Query Value"

    var id: Self { self }
}

struct URLEncodingOptions: Codable, Equatable {
    var direction: URLEncodingDirection = .encode
    var mode: URLEncodingMode = .pathSegment
}

struct URLEncodingSnapshot: Codable, Equatable {
    let options: URLEncodingOptions
    let input: String
    let output: String
}

enum URLEncodingEngine {
    static func transform(_ input: String, options: URLEncodingOptions) throws -> String {
        switch options.direction {
        case .encode:
            return encode(input, mode: options.mode)
        case .decode:
            return try decode(input)
        }
    }

    private static func encode(_ input: String, mode: URLEncodingMode) -> String {
        let unreserved = Set("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~".utf8)
        let pathPChar = Set(":@!$&'()*+,;=".utf8)
        let hexadecimal = Array("0123456789ABCDEF".utf8)
        var result = [UInt8]()

        for byte in input.utf8 {
            if unreserved.contains(byte) || (mode == .pathSegment && pathPChar.contains(byte)) {
                result.append(byte)
            } else {
                result.append(0x25)
                result.append(hexadecimal[Int(byte >> 4)])
                result.append(hexadecimal[Int(byte & 0x0F)])
            }
        }
        return String(decoding: result, as: UTF8.self)
    }

    private static func decode(_ input: String) throws -> String {
        let source = Array(input.utf8)
        var decoded = [UInt8]()
        var index = 0

        while index < source.count {
            if source[index] != 0x25 {
                decoded.append(source[index])
                index += 1
                continue
            }
            guard index + 2 < source.count,
                let high = hexadecimalValue(source[index + 1]),
                let low = hexadecimalValue(source[index + 2])
            else {
                throw UtilityError(
                    "Percent encoding contains an incomplete or invalid %HH triplet."
                )
            }
            decoded.append((high << 4) | low)
            index += 3
        }

        guard let result = String(bytes: decoded, encoding: .utf8) else {
            throw UtilityError("Decoded bytes are not valid UTF-8 text.")
        }
        return result
    }

    private static func hexadecimalValue(_ byte: UInt8) -> UInt8? {
        switch byte {
        case 0x30...0x39: byte - 0x30
        case 0x41...0x46: byte - 0x41 + 10
        case 0x61...0x66: byte - 0x61 + 10
        default: nil
        }
    }
}

struct URLEncodingSession {
    var options = URLEncodingOptions()
    var input = ""
    var output = ""
    var diagnostic: String?
    private var revision = 0
    private var pendingSnapshot: URLEncodingSnapshot?

    mutating func beginRevision() -> Int {
        revision += 1
        return revision
    }

    func accepts(_ candidate: Int) -> Bool { candidate == revision }

    mutating func evaluate(revision candidate: Int) {
        guard accepts(candidate) else { return }
        guard !input.isEmpty else {
            output = ""
            diagnostic = nil
            pendingSnapshot = nil
            return
        }
        do {
            let transformed = try URLEncodingEngine.transform(input, options: options)
            guard accepts(candidate) else { return }
            output = transformed
            diagnostic = nil
            pendingSnapshot = URLEncodingSnapshot(options: options, input: input, output: transformed)
        } catch {
            output = ""
            diagnostic = error.localizedDescription
            pendingSnapshot = nil
        }
    }

    mutating func consumeSnapshot() -> URLEncodingSnapshot? {
        defer { pendingSnapshot = nil }
        return pendingSnapshot
    }

    mutating func restore(_ snapshot: URLEncodingSnapshot) {
        revision += 1
        options = snapshot.options
        input = snapshot.input
        output = snapshot.output
        diagnostic = nil
        pendingSnapshot = nil
    }

    mutating func clear() {
        revision += 1
        input = ""
        output = ""
        diagnostic = nil
        pendingSnapshot = nil
    }
}

struct URLEncodingWorkspace: View {
    let context: UtilityWorkspaceContext
    @State private var session = URLEncodingSession()
    @State private var pendingRestore: UtilityHistoryEntry?
    @State private var suppressedOperationKey: String?

    var body: some View {
        VStack(spacing: 12) {
            HStack {
                Picker("Operation", selection: $session.options.direction) {
                    ForEach(URLEncodingDirection.allCases) { Text($0.rawValue).tag($0) }
                }
                .pickerStyle(.segmented)
                Picker("Mode", selection: $session.options.mode) {
                    ForEach(URLEncodingMode.allCases) { Text($0.rawValue).tag($0) }
                }
                .frame(width: 180)
                Button("Swap") { swap() }
                    .disabled(session.output.isEmpty)
                    .accessibilityIdentifier("url-encoding.swap")
            }
            Text(
                "Spaces use %20. Decode keeps + as a literal plus; this is not form encoding or URL-safe Base64."
            )
            .font(.caption)
            .foregroundStyle(.secondary)
            .frame(maxWidth: .infinity, alignment: .leading)
            HSplitView {
                TextEditorCard(
                    title: "UTF-8 Input", text: $session.input,
                    accessibilityID: "url-encoding.input"
                )
                TextEditorCard(
                    title: "Result", text: $session.output, editable: false,
                    accessibilityID: "url-encoding.result"
                )
            }
            if let diagnostic = session.diagnostic { DiagnosticBanner(message: diagnostic) }
            CopyPasteActions(
                canCopy: !session.output.isEmpty,
                paste: { session.input = context.clipboard.readText() ?? session.input },
                copy: { context.clipboard.writeText(session.output) },
                clear: { session.clear() }
            )
        }
        .padding(16)
        .task(id: operationKey) {
            if suppressedOperationKey == operationKey {
                suppressedOperationKey = nil
                return
            }
            suppressedOperationKey = nil
            let revision = session.beginRevision()
            try? await Task.sleep(for: .milliseconds(200))
            guard !Task.isCancelled, session.accepts(revision) else { return }
            session.evaluate(revision: revision)
            recordPendingSnapshot()
        }
        .onReceive(NotificationCenter.default.publisher(for: .restoreUtilitySnapshot)) {
            requestRestore($0.object as? UtilityHistoryEntry)
        }
        .alert(
            "Replace current workspace?",
            isPresented: Binding(
                get: { pendingRestore != nil },
                set: { if !$0 { pendingRestore = nil } }
            )
        ) {
            Button("Restore", role: .destructive) {
                let entry = pendingRestore
                pendingRestore = nil
                restore(entry)
            }
            Button("Cancel", role: .cancel) { pendingRestore = nil }
        } message: {
            Text("Restoring this History entry replaces the current non-empty URL Encoding session.")
        }
    }

    private var operationKey: String {
        session.options.direction.rawValue + "|" + session.options.mode.rawValue + "|" + session.input
    }

    private func recordPendingSnapshot() {
        guard let snapshot = session.consumeSnapshot(),
            let payload = try? JSONEncoder().encode(snapshot)
        else { return }
        context.record(.init(utilityID: "url-encoding", schemaVersion: 1, payload: payload))
    }

    private func swap() {
        let priorOutput = session.output
        session.options.direction = session.options.direction == .encode ? .decode : .encode
        session.input = priorOutput
        session.output = ""
        session.diagnostic = nil
    }

    private func restore(_ entry: UtilityHistoryEntry?) {
        guard let entry, entry.utilityID == "url-encoding",
            let snapshot = try? JSONDecoder().decode(URLEncodingSnapshot.self, from: entry.payload)
        else { return }
        session.restore(snapshot)
        suppressedOperationKey = operationKey
    }

    private func requestRestore(_ entry: UtilityHistoryEntry?) {
        guard let entry, entry.utilityID == "url-encoding",
            let snapshot = try? JSONDecoder().decode(URLEncodingSnapshot.self, from: entry.payload)
        else { return }
        if !session.input.isEmpty, session.input != snapshot.input {
            pendingRestore = entry
        } else {
            session.restore(snapshot)
            suppressedOperationKey = operationKey
        }
    }
}
