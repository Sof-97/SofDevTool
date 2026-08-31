import Foundation
import SwiftUI

struct TextDiffOptions: Codable, Hashable {
    var displayMode: TextDiffDisplayMode = .split
    var ignoresWhitespace = false
    var ignoresCase = false
    var filename = "Comparison.txt"
}

struct TextDiffPreparedContent: Equatable {
    let old: String
    let new: String
}

enum TextDiffEngine {
    static func prepare(old: String, new: String, options: TextDiffOptions) -> TextDiffPreparedContent {
        TextDiffPreparedContent(
            old: normalize(old, options: options),
            new: normalize(new, options: options)
        )
    }

    private static func normalize(_ value: String, options: TextDiffOptions) -> String {
        var result = value
        if options.ignoresCase {
            result = result.folding(
                options: [.caseInsensitive],
                locale: Locale(identifier: "en_US_POSIX")
            )
        }
        if options.ignoresWhitespace {
            result =
                result
                .split(separator: "\n", omittingEmptySubsequences: false)
                .map { line in line.filter { !$0.isWhitespace } }
                .joined(separator: "\n")
        }
        return result
    }
}

struct TextDiffSnapshot: Codable {
    let oldText: String
    let newText: String
    let options: TextDiffOptions
}

private struct TextDiffOperationKey: Hashable {
    let oldText: String
    let newText: String
    let options: TextDiffOptions
}

struct TextDiffWorkspace: View {
    let context: UtilityWorkspaceContext
    private let renderer: any TextDiffRenderer

    @State private var oldText = ""
    @State private var newText = ""
    @State private var options = TextDiffOptions()
    @State private var renderRequest: TextDiffRenderRequest?
    @State private var diagnostic: String?
    @State private var suppressNextRecord = false
    @State private var pendingRestore: UtilityHistoryEntry?

    init(
        context: UtilityWorkspaceContext,
        renderer: any TextDiffRenderer = PierreTextDiffRenderer()
    ) {
        self.context = context
        self.renderer = renderer
    }

    var body: some View {
        VStack(spacing: 12) {
            controls
            inputs
            actions
            renderedDifference
            if let diagnostic { DiagnosticBanner(message: diagnostic) }
        }
        .padding(16)
        .task(id: operationKey) {
            try? await Task.sleep(for: .milliseconds(300))
            guard !Task.isCancelled else { return }
            updateRenderRequest()
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
            Text("Restoring this History entry replaces the current non-empty Text Diff session.")
        }
    }

    private var controls: some View {
        HStack(spacing: 12) {
            Picker("Layout", selection: $options.displayMode) {
                ForEach(TextDiffDisplayMode.allCases) { mode in
                    Text(mode.rawValue).tag(mode)
                }
            }
            .pickerStyle(.segmented)
            .frame(width: 200)
            .accessibilityIdentifier("text-diff.layout")

            Toggle("Ignore whitespace", isOn: $options.ignoresWhitespace)
            Toggle("Ignore case", isOn: $options.ignoresCase)
            Spacer()
            TextField("Filename", text: $options.filename)
                .textFieldStyle(.roundedBorder)
                .frame(width: 180)
                .accessibilityIdentifier("text-diff.filename")
        }
    }

    private var inputs: some View {
        HSplitView {
            TextEditorCard(
                title: "Original",
                text: $oldText,
                accessibilityID: "text-diff.original"
            )
            TextEditorCard(
                title: "Updated",
                text: $newText,
                accessibilityID: "text-diff.updated"
            )
        }
        .frame(minHeight: 150, idealHeight: 210, maxHeight: 260)
    }

    private var actions: some View {
        HStack {
            Button("Paste Original") { oldText = context.clipboard.readText() ?? oldText }
                .accessibilityIdentifier("text-diff.paste-original")
            Button("Paste Updated") { newText = context.clipboard.readText() ?? newText }
                .accessibilityIdentifier("text-diff.paste-updated")
            Button("Swap") { (oldText, newText) = (newText, oldText) }
            Button("Clear") {
                oldText = ""
                newText = ""
                diagnostic = nil
                renderRequest = nil
            }
            Spacer()
            Button("Copy Updated") { context.clipboard.writeText(newText) }
                .disabled(newText.isEmpty)
                .accessibilityIdentifier("text-diff.copy-updated")
        }
    }

    @ViewBuilder
    private var renderedDifference: some View {
        if let request = renderRequest {
            VStack(alignment: .leading, spacing: 6) {
                HStack {
                    Text("Difference").font(.caption).foregroundStyle(.secondary)
                    if TextDiffRenderPolicy.requiresWholeLineHighlighting(
                        old: request.oldText,
                        new: request.newText
                    ) {
                        Text("Emoji-safe whole-line highlighting")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                            .help(
                                "Complex emoji are highlighted by whole line so every grapheme remains intact."
                            )
                    }
                }
                renderer.render(request: request) { event in
                    handle(event, for: request)
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        } else {
            ContentUnavailableView(
                "Enter text to compare",
                systemImage: "text.page.badge.magnifyingglass",
                description: Text("The comparison stays entirely on this Mac.")
            )
            .frame(maxWidth: .infinity, maxHeight: .infinity)
        }
    }

    private var operationKey: TextDiffOperationKey {
        TextDiffOperationKey(oldText: oldText, newText: newText, options: options)
    }

    private func updateRenderRequest() {
        guard !oldText.isEmpty || !newText.isEmpty else {
            renderRequest = nil
            diagnostic = nil
            return
        }
        let prepared = TextDiffEngine.prepare(old: oldText, new: newText, options: options)
        renderRequest = TextDiffRenderRequest(
            id: UUID(),
            oldText: prepared.old,
            newText: prepared.new,
            filename: normalizedFilename,
            displayMode: options.displayMode
        )
        diagnostic = nil
    }

    private var normalizedFilename: String {
        let candidate = options.filename.trimmingCharacters(in: .whitespacesAndNewlines)
        return candidate.isEmpty ? "Comparison.txt" : candidate
    }

    private func handle(_ event: TextDiffRendererEvent, for request: TextDiffRenderRequest) {
        guard renderRequest?.id == request.id else { return }
        switch event {
        case .ready:
            diagnostic = nil
            if suppressNextRecord {
                suppressNextRecord = false
            } else if let payload = try? JSONEncoder().encode(
                TextDiffSnapshot(oldText: oldText, newText: newText, options: options)
            ) {
                context.record(.init(utilityID: "text-diff", schemaVersion: 1, payload: payload))
            }
        case .failed(let message):
            diagnostic = "The difference could not be rendered: \(message)"
        }
    }

    private func restore(_ entry: UtilityHistoryEntry?) {
        guard let entry, entry.utilityID == "text-diff",
            let snapshot = try? JSONDecoder().decode(TextDiffSnapshot.self, from: entry.payload)
        else { return }
        suppressNextRecord = true
        oldText = snapshot.oldText
        newText = snapshot.newText
        options = snapshot.options
        diagnostic = nil
    }

    private func requestRestore(_ entry: UtilityHistoryEntry?) {
        guard let entry, entry.utilityID == "text-diff",
            let snapshot = try? JSONDecoder().decode(TextDiffSnapshot.self, from: entry.payload)
        else { return }
        if (!oldText.isEmpty || !newText.isEmpty)
            && (oldText != snapshot.oldText || newText != snapshot.newText)
        {
            pendingRestore = entry
        } else {
            restore(entry)
        }
    }
}
