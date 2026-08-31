import Foundation
import SwiftUI

struct RegexFlags: Codable, Equatable, Hashable, Sendable {
    var caseInsensitive = false
    var multiline = false
    var dotMatchesNewlines = false
    var commentsAndWhitespace = false
}

struct RegexRequest: Codable, Equatable, Hashable, Sendable {
    let pattern: String
    let text: String
    let replacement: String
    let flags: RegexFlags
}

struct RegexTextRange: Codable, Equatable, Sendable {
    let location: Int
    let length: Int
}

struct RegexCapture: Codable, Equatable, Sendable {
    let index: Int
    let name: String?
    let value: String?
    let range: RegexTextRange?
}

struct RegexMatch: Codable, Equatable, Sendable, Identifiable {
    let ordinal: Int
    let value: String
    let range: RegexTextRange
    let captures: [RegexCapture]
    var id: Int { ordinal }
}

struct RegexResult: Codable, Equatable, Sendable {
    let matches: [RegexMatch]
    let replacementPreview: String
}

enum RegexFailure: LocalizedError, Equatable, Sendable {
    case patternTooLarge
    case inputTooLarge
    case replacementTooLarge
    case invalidPattern(line: Int, offset: Int)
    case invalidReplacement
    case engineStepLimit
    case backtrackingLimit
    case cancelled
    case deadlineExceeded
    case matchLimit
    case captureLimit
    case replacementOutputLimit
    case backendFailure(Int32)

    var errorDescription: String? {
        switch self {
        case .patternTooLarge: return "Pattern exceeds the configured UTF-16 size limit."
        case .inputTooLarge: return "Test text exceeds the configured UTF-16 size limit."
        case .replacementTooLarge: return "Replacement template exceeds the configured UTF-16 size limit."
        case .invalidPattern(let line, let offset):
            return "Invalid ICU pattern at line \(line), offset \(offset)."
        case .invalidReplacement: return "Invalid ICU replacement template or capture reference."
        case .engineStepLimit:
            return "The ICU engine-step budget was exhausted; no partial result was published."
        case .backtrackingLimit:
            return "The ICU heap-backtracking budget was exhausted; no partial result was published."
        case .cancelled: return "The Regex operation was cancelled; no partial result was published."
        case .deadlineExceeded:
            return "The elapsed guard stopped the Regex operation; no partial result was published."
        case .matchLimit: return "The match-count limit was reached; no partial result was published."
        case .captureLimit: return "The capture-count limit was reached; no partial result was published."
        case .replacementOutputLimit: return "Replacement preview exceeds the configured output limit."
        case .backendFailure(let code): return "The ICU Regex backend failed with error code \(code)."
        }
    }
}

struct RegexExecutionPolicy: Equatable, Sendable {
    static let production = RegexExecutionPolicy(
        maximumPatternUTF16Length: 16_384,
        maximumTextUTF16Length: 1_048_576,
        maximumReplacementUTF16Length: 262_144,
        engineStepLimit: 2_000_000,
        heapBacktrackingLimitBytes: 8 * 1_024 * 1_024,
        elapsedDeadlineNanoseconds: 1_500_000_000,
        maximumMatches: 10_000,
        maximumCaptures: 50_000,
        maximumReplacementOutputUTF16Length: 2_097_152
    )

    let maximumPatternUTF16Length: Int
    let maximumTextUTF16Length: Int
    let maximumReplacementUTF16Length: Int
    let engineStepLimit: Int32
    let heapBacktrackingLimitBytes: Int32
    let elapsedDeadlineNanoseconds: UInt64
    let maximumMatches: Int
    let maximumCaptures: Int
    let maximumReplacementOutputUTF16Length: Int
}

final class RegexCancellation: @unchecked Sendable {
    private let lock = NSLock()
    private var cancelled = false

    func cancel() { lock.withLock { cancelled = true } }
    var isCancelled: Bool { lock.withLock { cancelled } }
}

protocol RegexBackend: Sendable {
    func execute(
        _ request: RegexRequest,
        policy: RegexExecutionPolicy,
        cancellation: RegexCancellation
    ) throws -> RegexResult
}

enum RegexEngine {
    static func run(
        _ request: RegexRequest,
        policy: RegexExecutionPolicy = .production,
        cancellation: RegexCancellation = RegexCancellation(),
        backend: any RegexBackend = ICURegexBackend()
    ) throws -> RegexResult {
        guard request.pattern.utf16.count <= policy.maximumPatternUTF16Length else {
            throw RegexFailure.patternTooLarge
        }
        guard request.text.utf16.count <= policy.maximumTextUTF16Length else {
            throw RegexFailure.inputTooLarge
        }
        guard request.replacement.utf16.count <= policy.maximumReplacementUTF16Length else {
            throw RegexFailure.replacementTooLarge
        }
        guard policy.engineStepLimit > 0, policy.heapBacktrackingLimitBytes > 0 else {
            preconditionFailure("Regex production and injected policies require nonzero ICU budgets.")
        }
        return try backend.execute(request, policy: policy, cancellation: cancellation)
    }
}

actor RegexSerialWorker {
    private let backend: any RegexBackend

    init(backend: any RegexBackend = ICURegexBackend()) { self.backend = backend }

    func execute(
        _ request: RegexRequest,
        policy: RegexExecutionPolicy = .production,
        cancellation: RegexCancellation
    ) async throws -> RegexResult {
        try RegexEngine.run(request, policy: policy, cancellation: cancellation, backend: backend)
    }
}

struct RegexSnapshot: Codable, Equatable {
    let request: RegexRequest
    let result: RegexResult
}

struct RegexRevisionGate {
    private(set) var current = 0
    mutating func begin() -> Int {
        current += 1
        return current
    }
    func accepts(_ revision: Int) -> Bool { revision == current }
}

struct RegexWorkspace: View {
    let context: UtilityWorkspaceContext
    private let worker = RegexSerialWorker()
    @State private var pattern = ""
    @State private var text = ""
    @State private var replacement = ""
    @State private var flags = RegexFlags()
    @State private var result: RegexResult?
    @State private var diagnostic: String?
    @State private var publishedRequest: RegexRequest?
    @State private var revisionGate = RegexRevisionGate()
    @State private var cancellation: RegexCancellation?
    @State private var pendingRestore: UtilityHistoryEntry?

    var body: some View {
        VStack(spacing: 12) {
            Text("Platform ICU-compatible regular-expression dialect")
                .font(.caption).foregroundStyle(.secondary)
                .frame(maxWidth: .infinity, alignment: .leading)
            flagControls
            TextField("Pattern", text: $pattern)
                .textFieldStyle(.roundedBorder)
                .accessibilityIdentifier("regex.pattern")
            HSplitView {
                TextEditorCard(title: "Test Text", text: $text, accessibilityID: "regex.text")
                matchesView
            }
            TextField("Replacement Template", text: $replacement)
                .textFieldStyle(.roundedBorder)
                .accessibilityIdentifier("regex.replacement")
            TextEditorCard(
                title: "Replacement Preview", text: .constant(currentResult?.replacementPreview ?? ""),
                editable: false, accessibilityID: "regex.replacement-preview"
            ).frame(minHeight: 80, maxHeight: 130)
            if let currentDiagnostic { DiagnosticBanner(message: currentDiagnostic) }
            actions
        }
        .padding(16)
        .onChange(of: currentRequest) { _, _ in
            cancellation?.cancel()
            cancellation = nil
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
            Text("Restoring this History entry replaces the current non-empty Regex session.")
        }
    }

    private var flagControls: some View {
        HStack {
            Toggle("Case insensitive", isOn: $flags.caseInsensitive)
            Toggle("Multiline", isOn: $flags.multiline)
            Toggle("Dot matches newlines", isOn: $flags.dotMatchesNewlines)
            Toggle("Comments and whitespace", isOn: $flags.commentsAndWhitespace)
        }
    }

    private var matchesView: some View {
        VStack(alignment: .leading) {
            Text("All Matches and Captures").font(.caption).foregroundStyle(.secondary)
            List(currentResult?.matches ?? []) { match in
                VStack(alignment: .leading) {
                    Text("\(match.ordinal + 1). \(match.value)")
                    ForEach(match.captures, id: \.index) { capture in
                        Text("  \(capture.name ?? String(capture.index)): \(captureDescription(capture))")
                            .font(.caption).foregroundStyle(.secondary)
                    }
                }
            }
            .accessibilityIdentifier("regex.matches")
        }
    }

    private func captureDescription(_ capture: RegexCapture) -> String {
        guard let value = capture.value else { return "(unmatched)" }
        return value.isEmpty ? "(empty)" : value
    }

    private var actions: some View {
        HStack {
            Button("Paste Test Text") { text = context.clipboard.readText() ?? text }
            Button("Run") { run() }.keyboardShortcut(.return, modifiers: [.command])
                .disabled(pattern.isEmpty)
                .accessibilityIdentifier("regex.run")
            Button("Cancel") { cancellation?.cancel() }.disabled(cancellation == nil)
            Button("Clear") {
                cancellation?.cancel()
                pattern = ""
                text = ""
                replacement = ""
                result = nil
                diagnostic = nil
                publishedRequest = nil
            }
            Spacer()
            ConfirmedCopyButton(title: "Copy Replacement", isEnabled: currentResult != nil) {
                if let currentResult { context.clipboard.writeText(currentResult.replacementPreview) }
            }
        }
    }

    private var currentRequest: RegexRequest {
        RegexRequest(pattern: pattern, text: text, replacement: replacement, flags: flags)
    }

    private var currentResult: RegexResult? {
        publishedRequest == currentRequest ? result : nil
    }

    private var currentDiagnostic: String? {
        publishedRequest == currentRequest ? diagnostic : nil
    }

    private func run() {
        cancellation?.cancel()
        result = nil
        diagnostic = nil
        publishedRequest = nil
        guard !pattern.isEmpty else { return }
        let revision = revisionGate.begin()
        let token = RegexCancellation()
        cancellation = token
        let request = RegexRequest(pattern: pattern, text: text, replacement: replacement, flags: flags)
        Task {
            do {
                let completed = try await worker.execute(request, cancellation: token)
                guard revisionGate.accepts(revision), !token.isCancelled else { return }
                result = completed
                diagnostic = nil
                publishedRequest = request
                cancellation = nil
                if let payload = try? JSONEncoder().encode(
                    RegexSnapshot(request: request, result: completed))
                {
                    context.record(.init(utilityID: "regex", schemaVersion: 1, payload: payload))
                }
            } catch {
                guard revisionGate.accepts(revision), !token.isCancelled else { return }
                result = nil
                diagnostic = error.localizedDescription
                publishedRequest = request
                cancellation = nil
            }
        }
    }

    private func restore(_ entry: UtilityHistoryEntry?) {
        guard let entry, entry.utilityID == "regex",
            let snapshot = try? JSONDecoder().decode(RegexSnapshot.self, from: entry.payload)
        else { return }
        cancellation?.cancel()
        pattern = snapshot.request.pattern
        text = snapshot.request.text
        replacement = snapshot.request.replacement
        flags = snapshot.request.flags
        result = snapshot.result
        diagnostic = nil
        publishedRequest = snapshot.request
    }

    private func requestRestore(_ entry: UtilityHistoryEntry?) {
        guard let entry, entry.utilityID == "regex",
            let snapshot = try? JSONDecoder().decode(RegexSnapshot.self, from: entry.payload)
        else { return }
        let differs =
            !pattern.isEmpty || !text.isEmpty
            ? pattern != snapshot.request.pattern || text != snapshot.request.text : false
        if differs { pendingRestore = entry } else { restore(entry) }
    }
}
