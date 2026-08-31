import CryptoKit
import Foundation
import SwiftUI

enum HashAlgorithm: String, CaseIterable, Codable, Identifiable {
    case sha256 = "SHA-256"
    case sha384 = "SHA-384"
    case sha512 = "SHA-512"
    case sha1 = "SHA-1 (Legacy)"
    case md5 = "MD5 (Legacy)"

    var id: Self { self }
    var isLegacy: Bool { self == .sha1 || self == .md5 }
    var notice: String {
        isLegacy
            ? "Legacy algorithm — not for security, password hashing, authentication, or new designs."
            : ""
    }
}

enum HashOutputFormat: String, CaseIterable, Codable, Identifiable {
    case lowercaseHex = "Lowercase hexadecimal"
    case uppercaseHex = "Uppercase hexadecimal"
    case base64 = "Base64"

    var id: Self { self }
}

struct HashesSnapshot: Codable, Equatable {
    let algorithm: HashAlgorithm
    let format: HashOutputFormat
    let input: String
    let output: String
}

enum HashEngine {
    static func hash(
        _ input: String,
        algorithm: HashAlgorithm,
        format: HashOutputFormat
    ) -> String {
        let bytes = digest(Data(input.utf8), algorithm: algorithm)
        switch format {
        case .lowercaseHex:
            return bytes.map { String(format: "%02x", $0) }.joined()
        case .uppercaseHex:
            return bytes.map { String(format: "%02X", $0) }.joined()
        case .base64:
            return Data(bytes).base64EncodedString()
        }
    }

    private static func digest(_ data: Data, algorithm: HashAlgorithm) -> [UInt8] {
        switch algorithm {
        case .sha256: Array(SHA256.hash(data: data))
        case .sha384: Array(SHA384.hash(data: data))
        case .sha512: Array(SHA512.hash(data: data))
        case .sha1: Array(Insecure.SHA1.hash(data: data))
        case .md5: Array(Insecure.MD5.hash(data: data))
        }
    }
}

struct HashesSession {
    var algorithm = HashAlgorithm.sha256
    var format = HashOutputFormat.lowercaseHex
    var input = ""
    var output = ""
    private var pendingSnapshot: HashesSnapshot?

    mutating func hash() {
        output = HashEngine.hash(input, algorithm: algorithm, format: format)
        pendingSnapshot = HashesSnapshot(
            algorithm: algorithm,
            format: format,
            input: input,
            output: output
        )
    }

    mutating func consumeSnapshot() -> HashesSnapshot? {
        defer { pendingSnapshot = nil }
        return pendingSnapshot
    }

    mutating func restore(_ snapshot: HashesSnapshot) {
        algorithm = snapshot.algorithm
        format = snapshot.format
        input = snapshot.input
        output = snapshot.output
        pendingSnapshot = nil
    }

    mutating func clear() {
        input = ""
        output = ""
        pendingSnapshot = nil
    }
}

struct HashesWorkspace: View {
    let context: UtilityWorkspaceContext
    @State private var session = HashesSession()
    @State private var pendingRestore: UtilityHistoryEntry?
    @State private var suppressedOperationKey: String?

    var body: some View {
        VStack(spacing: 12) {
            HStack {
                Picker("Algorithm", selection: $session.algorithm) {
                    ForEach(HashAlgorithm.allCases) { Text($0.rawValue).tag($0) }
                }
                .frame(width: 180)
                Picker("Output", selection: $session.format) {
                    ForEach(HashOutputFormat.allCases) { Text($0.rawValue).tag($0) }
                }
                .frame(width: 220)
                Button("Hash") { hash() }
                    .keyboardShortcut(.return, modifiers: [.command])
                    .accessibilityIdentifier("hashes.hash")
            }
            if session.algorithm.isLegacy {
                DiagnosticBanner(message: session.algorithm.notice)
            }
            Text(
                "Hashes the exact UTF-8 bytes of the text. Files, HMAC, and password hashing are not included."
            )
            .font(.caption)
            .foregroundStyle(.secondary)
            .frame(maxWidth: .infinity, alignment: .leading)
            HSplitView {
                TextEditorCard(
                    title: "UTF-8 Input", text: $session.input, accessibilityID: "hashes.input"
                )
                TextEditorCard(
                    title: "Digest", text: $session.output, editable: false,
                    accessibilityID: "hashes.result"
                )
            }
            CopyPasteActions(
                canCopy: !session.output.isEmpty,
                paste: { session.input = context.clipboard.readText() ?? session.input },
                copy: { context.clipboard.writeText(session.output) },
                clear: { session.clear() }
            )
        }
        .padding(16)
        .onChange(of: operationKey) { _, newValue in
            if suppressedOperationKey == newValue {
                suppressedOperationKey = nil
            } else {
                suppressedOperationKey = nil
                session.output = ""
            }
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
            Text("Restoring this History entry replaces the current non-empty Hashes session.")
        }
    }

    private var operationKey: String {
        session.algorithm.rawValue + "|" + session.format.rawValue + "|" + session.input
    }

    private func hash() {
        session.hash()
        guard let snapshot = session.consumeSnapshot(),
            let payload = try? JSONEncoder().encode(snapshot)
        else { return }
        context.record(.init(utilityID: "hashes", schemaVersion: 1, payload: payload))
    }

    private func restore(_ entry: UtilityHistoryEntry?) {
        guard let entry, entry.utilityID == "hashes",
            let snapshot = try? JSONDecoder().decode(HashesSnapshot.self, from: entry.payload)
        else { return }
        session.restore(snapshot)
        suppressedOperationKey = operationKey
    }

    private func requestRestore(_ entry: UtilityHistoryEntry?) {
        guard let entry, entry.utilityID == "hashes",
            let snapshot = try? JSONDecoder().decode(HashesSnapshot.self, from: entry.payload)
        else { return }
        if !session.input.isEmpty, session.input != snapshot.input {
            pendingRestore = entry
        } else {
            session.restore(snapshot)
            suppressedOperationKey = operationKey
        }
    }
}
