import Foundation
import SwiftUI

struct JWTDecodedToken: Codable, Equatable {
    let headerJSON: String
    let payloadJSON: String
    let signature: String
}

enum JWTDecoderFailure: LocalizedError, Equatable {
    case segmentCount(Int)
    case invalidBase64URL(segment: String)
    case invalidUTF8(segment: String)
    case invalidJSON(segment: String)

    var errorDescription: String? {
        switch self {
        case .segmentCount(let count):
            return "JWT must contain exactly three dot-separated segments; found \(count)."
        case .invalidBase64URL(let segment):
            return "The \(segment) segment is not valid Base64URL."
        case .invalidUTF8(let segment):
            return "The \(segment) segment is not valid UTF-8."
        case .invalidJSON(let segment):
            return "The \(segment) segment is not valid JSON."
        }
    }
}

enum JWTDecoderEngine {
    static func decode(_ token: String) throws -> JWTDecodedToken {
        let segments = token.split(separator: ".", omittingEmptySubsequences: false)
        guard segments.count == 3 else { throw JWTDecoderFailure.segmentCount(segments.count) }

        let header = try decodeJSON(String(segments[0]), named: "header")
        let payload = try decodeJSON(String(segments[1]), named: "payload")
        return JWTDecodedToken(
            headerJSON: header,
            payloadJSON: payload,
            signature: String(segments[2])
        )
    }

    private static func decodeJSON(_ segment: String, named name: String) throws -> String {
        guard let data = decodeBase64URL(segment) else {
            throw JWTDecoderFailure.invalidBase64URL(segment: name)
        }
        guard let text = String(data: data, encoding: .utf8) else {
            throw JWTDecoderFailure.invalidUTF8(segment: name)
        }
        guard
            let value = try? JSONSerialization.jsonObject(
                with: Data(text.utf8), options: [.fragmentsAllowed]),
            JSONSerialization.isValidJSONObject(value)
        else {
            throw JWTDecoderFailure.invalidJSON(segment: name)
        }
        let formatted = try JSONSerialization.data(
            withJSONObject: value,
            options: [.prettyPrinted, .sortedKeys, .withoutEscapingSlashes]
        )
        return String(decoding: formatted, as: UTF8.self)
    }

    private static func decodeBase64URL(_ value: String) -> Data? {
        guard !value.isEmpty else { return nil }
        let alphabet = CharacterSet(
            charactersIn: "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_=")
        let trailingPadding = value.reversed().prefix(while: { $0 == "=" }).count
        guard value.unicodeScalars.allSatisfy(alphabet.contains),
            !value.dropLast(trailingPadding).contains("=")
        else { return nil }

        var normalized = value.replacingOccurrences(of: "-", with: "+")
            .replacingOccurrences(of: "_", with: "/")
        let remainder = normalized.count % 4
        guard remainder != 1 else { return nil }
        if remainder > 0 { normalized += String(repeating: "=", count: 4 - remainder) }
        return Data(base64Encoded: normalized, options: [])
    }
}

struct JWTDecoderSnapshot: Codable, Equatable {
    let input: String
    let decoded: JWTDecodedToken
}

struct JWTDecoderRevisionGate {
    private(set) var current = 0

    mutating func begin() -> Int {
        current += 1
        return current
    }

    func accepts(_ revision: Int) -> Bool { revision == current }
}

struct JWTDecoderWorkspace: View {
    let context: UtilityWorkspaceContext
    @State private var input = ""
    @State private var decoded: JWTDecodedToken?
    @State private var diagnostic: String?
    @State private var skipNextEvaluation = false
    @State private var pendingRestore: UtilityHistoryEntry?
    @State private var revisionGate = JWTDecoderRevisionGate()

    var body: some View {
        VStack(spacing: 12) {
            Text(
                "Decoding only: this does not verify the signature, interpret claims, judge validity, establish trust, or determine fitness for use."
            )
            .font(.headline)
            .foregroundStyle(.orange)
            .frame(maxWidth: .infinity, alignment: .leading)

            TextEditorCard(title: "JWT", text: $input, accessibilityID: "jwt-decoder.input")
                .frame(minHeight: 90, maxHeight: 140)

            HSplitView {
                TextEditorCard(
                    title: "Header JSON",
                    text: .constant(decoded?.headerJSON ?? ""),
                    editable: false,
                    accessibilityID: "jwt-decoder.header"
                )
                TextEditorCard(
                    title: "Payload JSON",
                    text: .constant(decoded?.payloadJSON ?? ""),
                    editable: false,
                    accessibilityID: "jwt-decoder.payload"
                )
            }

            Text("Signature is opaque and unverified: \(decoded?.signature ?? "—")")
                .font(.caption)
                .frame(maxWidth: .infinity, alignment: .leading)

            if let diagnostic { DiagnosticBanner(message: diagnostic) }
            actions
        }
        .padding(16)
        .task(id: input) {
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
            Text("Restoring this History entry replaces the current non-empty JWT Decoder session.")
        }
    }

    private var actions: some View {
        HStack {
            Button("Paste") { input = context.clipboard.readText() ?? input }
                .accessibilityIdentifier("jwt-decoder.paste")
            Button("Clear") {
                input = ""
                decoded = nil
                diagnostic = nil
            }
            Spacer()
            ConfirmedCopyButton(
                title: "Copy Header", isEnabled: decoded != nil,
                accessibilityID: "jwt-decoder.copy-header"
            ) {
                if let decoded { context.clipboard.writeText(decoded.headerJSON) }
            }
            ConfirmedCopyButton(
                title: "Copy Payload", isEnabled: decoded != nil,
                accessibilityID: "jwt-decoder.copy-payload"
            ) {
                if let decoded { context.clipboard.writeText(decoded.payloadJSON) }
            }
        }
    }

    private func evaluate(revision: Int) {
        guard !input.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
            decoded = nil
            diagnostic = nil
            return
        }
        do {
            let value = try JWTDecoderEngine.decode(input)
            guard revisionGate.accepts(revision) else { return }
            decoded = value
            diagnostic = nil
            if let payload = try? JSONEncoder().encode(
                JWTDecoderSnapshot(input: input, decoded: value)
            ) {
                context.record(.init(utilityID: "jwt-decoder", schemaVersion: 1, payload: payload))
            }
        } catch {
            decoded = nil
            diagnostic = error.localizedDescription
        }
    }

    private func restore(_ entry: UtilityHistoryEntry?) {
        guard let entry, entry.utilityID == "jwt-decoder",
            let snapshot = try? JSONDecoder().decode(JWTDecoderSnapshot.self, from: entry.payload)
        else { return }
        skipNextEvaluation = true
        input = snapshot.input
        decoded = snapshot.decoded
        diagnostic = nil
    }

    private func requestRestore(_ entry: UtilityHistoryEntry?) {
        guard let entry, entry.utilityID == "jwt-decoder",
            let snapshot = try? JSONDecoder().decode(JWTDecoderSnapshot.self, from: entry.payload)
        else { return }
        if !input.isEmpty, input != snapshot.input { pendingRestore = entry } else { restore(entry) }
    }
}
