import SwiftUI

struct YAMLJSONWorkspace: View {
    let context: UtilityWorkspaceContext
    @State private var direction = YAMLJSONDirection.yamlToJSON
    @State private var input = ""
    @State private var output = ""
    @State private var diagnostic: String?
    @State private var suppressNextRecord = false
    @State private var pendingRestore: UtilityHistoryEntry?
    @State private var revisionGate = YAMLJSONRevisionGate()

    var body: some View {
        VStack(spacing: 12) {
            HStack {
                Picker("Direction", selection: $direction) {
                    ForEach(YAMLJSONDirection.allCases) { Text($0.rawValue).tag($0) }
                }
                .pickerStyle(.segmented)

                Button("Swap") { swapDirection() }
                    .disabled(output.isEmpty)
                    .accessibilityIdentifier("yaml-json.swap")
            }

            Text(
                "Comments, aliases, formatting, mapping order, and exact key presentation may not survive a round trip. Conversion is not lossless."
            )
            .font(.caption)
            .foregroundStyle(.secondary)
            .frame(maxWidth: .infinity, alignment: .leading)

            HSplitView {
                TextEditorCard(
                    title: direction == .yamlToJSON ? "YAML Input" : "JSON Input",
                    text: $input,
                    accessibilityID: "yaml-json.input"
                )
                TextEditorCard(
                    title: direction == .yamlToJSON ? "JSON Result" : "YAML Result",
                    text: $output,
                    editable: false,
                    accessibilityID: "yaml-json.result"
                )
            }

            if let diagnostic { DiagnosticBanner(message: diagnostic) }
            CopyPasteActions(
                canCopy: !output.isEmpty,
                paste: { input = context.clipboard.readText() ?? input },
                copy: { context.clipboard.writeText(output) },
                clear: {
                    input = ""
                    output = ""
                    diagnostic = nil
                }
            )
        }
        .padding(16)
        .task(id: operationKey) {
            let revision = revisionGate.begin()
            try? await Task.sleep(for: .milliseconds(250))
            guard !Task.isCancelled, revisionGate.accepts(revision) else { return }
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
            Text("Restoring this History entry replaces the current non-empty YAML/JSON session.")
        }
    }

    private var operationKey: String { direction.rawValue + "|" + input }

    private func evaluate(revision: Int) {
        guard !input.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
            output = ""
            diagnostic = nil
            return
        }
        do {
            let result = try YAMLJSONEngine.convert(input, direction: direction)
            guard !Task.isCancelled, revisionGate.accepts(revision) else { return }
            output = result.output
            diagnostic = nil
            if suppressNextRecord {
                suppressNextRecord = false
            } else if let payload = try? JSONEncoder().encode(
                YAMLJSONSnapshot(direction: direction, input: input, output: output)
            ) {
                context.record(.init(utilityID: "yaml-json", schemaVersion: 1, payload: payload))
            }
        } catch {
            output = ""
            diagnostic = error.localizedDescription
        }
    }

    private func swapDirection() {
        let previousOutput = output
        direction = direction == .yamlToJSON ? .jsonToYAML : .yamlToJSON
        input = previousOutput
        output = ""
        diagnostic = nil
    }

    private func restore(_ entry: UtilityHistoryEntry?) {
        guard let entry, entry.utilityID == "yaml-json",
            let snapshot = try? JSONDecoder().decode(YAMLJSONSnapshot.self, from: entry.payload)
        else { return }
        suppressNextRecord = true
        direction = snapshot.direction
        input = snapshot.input
        output = snapshot.output
        diagnostic = nil
    }

    private func requestRestore(_ entry: UtilityHistoryEntry?) {
        guard let entry, entry.utilityID == "yaml-json",
            let snapshot = try? JSONDecoder().decode(YAMLJSONSnapshot.self, from: entry.payload)
        else { return }
        if !input.isEmpty, input != snapshot.input { pendingRestore = entry } else { restore(entry) }
    }
}
