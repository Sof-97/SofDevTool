import SwiftUI

extension Notification.Name {
    static let restoreUtilitySnapshot = Notification.Name("restoreUtilitySnapshot")
}

struct HistoryInspector: View {
    @EnvironmentObject private var model: AppModel
    @Environment(\.colorSchemeContrast) private var contrast
    let utilityID: String
    @Binding var expanded: Bool
    @State private var selectedEntryID: UUID?
    @State private var showClearConfirmation = false

    private var entries: [UtilityHistoryEntry] { model.history.entries(for: utilityID) }
    private var palette: SemanticThemePalette { model.theme.palette }

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack {
                VStack(alignment: .leading, spacing: 2) {
                    Text("History").font(.headline)
                    Text("\(entries.count) operations")
                        .font(.caption2)
                        .foregroundStyle(palette.subtleText.color)
                }
                Spacer()
                Button {
                    expanded = false
                } label: {
                    Image(systemName: "sidebar.trailing")
                }.buttonStyle(.plain)
            }.padding(12)
            themedSeparator
            if model.history.malformedUtilityIDs.contains(utilityID) {
                Label(
                    "History file is unreadable. It was left untouched.",
                    systemImage: "exclamationmark.triangle"
                )
                .font(.caption).foregroundStyle(palette.warning.color).padding()
            }
            if entries.isEmpty {
                ContentUnavailableView(
                    "No History", systemImage: "clock",
                    description: Text("Completed valid operations appear here."))
            } else {
                List(entries, selection: $selectedEntryID) { entry in
                    VStack(alignment: .leading) {
                        Text(
                            entry.capturedAt, format: .dateTime.year().month().day().hour().minute().second())
                        Text("Snapshot schema \(entry.schemaVersion)")
                            .font(.caption)
                            .foregroundStyle(palette.secondaryText.color)
                    }.tag(entry.id)
                }
                if let entry = entries.first(where: { $0.id == selectedEntryID }) {
                    themedSeparator
                    ScrollView {
                        Text(historyPreview(for: entry))
                            .font(.caption.monospaced())
                            .foregroundStyle(palette.codeText.color)
                            .textSelection(.enabled)
                            .frame(maxWidth: .infinity, alignment: .leading)
                            .padding(10)
                    }
                    .frame(maxHeight: 150)
                    .background(
                        palette.editorSurface.color, in: RoundedRectangle(cornerRadius: 9)
                    )
                    .overlay(
                        RoundedRectangle(cornerRadius: 9)
                            .stroke(palette.separator.color.opacity(0.44))
                    )
                    .padding(.horizontal, 10)
                    Button("Restore") {
                        NotificationCenter.default.post(name: .restoreUtilitySnapshot, object: entry)
                    }.padding(.horizontal).padding(.bottom, 8)
                }
            }
            themedSeparator
            HoldToConfirmButton(title: "Hold to Clear Utility", duration: 1) {
                try? model.history.clear(utilityID)
            }
            .padding(10)
            .accessibilityAction { showClearConfirmation = true }
        }
        .background(palette.secondaryPane.color)
        .accessibilityIdentifier("history.inspector")
        .foregroundStyle(palette.primaryText.color)
        .tint(palette.appAccent.color)
        .overlay(alignment: .leading) {
            palette.separator.color.opacity(contrast == .increased ? 0.72 : 0.34).frame(width: 1)
        }
        .alert("Clear Utility History?", isPresented: $showClearConfirmation) {
            Button("Clear", role: .destructive) { try? model.history.clear(utilityID) }
            Button("Cancel", role: .cancel) {}
        } message: {
            Text("This permanently deletes this Utility’s History. The current workspace is preserved.")
        }
    }

    private func historyPreview(for entry: UtilityHistoryEntry) -> String {
        model.registry.definition(id: entry.utilityID)?.historyPreview(entry)
            ?? UtilityHistoryPreview.unavailable
    }

    private var themedSeparator: some View {
        palette.separator.color.opacity(contrast == .increased ? 0.72 : 0.34).frame(height: 1)
    }
}

struct HoldToConfirmButton: View {
    let title: String
    let duration: Double
    let action: () -> Void
    @State private var pressing = false

    var body: some View {
        Button(role: .destructive) {
        } label: {
            Label(pressing ? "Keep holding…" : title, systemImage: "trash")
                .frame(maxWidth: .infinity)
        }
        .simultaneousGesture(
            LongPressGesture(minimumDuration: duration).onChanged { _ in pressing = true }.onEnded { _ in
                pressing = false
                action()
            })
    }
}
