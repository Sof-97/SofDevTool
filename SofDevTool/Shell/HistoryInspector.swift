import SwiftUI

extension Notification.Name {
    static let restoreUtilitySnapshot = Notification.Name("restoreUtilitySnapshot")
}

struct HistoryInspector: View {
    @EnvironmentObject private var model: AppModel
    let utilityID: String
    @Binding var expanded: Bool
    @State private var selectedEntryID: UUID?
    @State private var showClearConfirmation = false

    private var entries: [UtilityHistoryEntry] { model.history.entries(for: utilityID) }

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack {
                Text("History").font(.headline)
                Spacer()
                Button {
                    expanded = false
                } label: {
                    Image(systemName: "sidebar.trailing")
                }.buttonStyle(.plain)
            }.padding(12)
            Divider()
            if model.history.malformedUtilityIDs.contains(utilityID) {
                Label(
                    "History file is unreadable. It was left untouched.",
                    systemImage: "exclamationmark.triangle"
                )
                .font(.caption).foregroundStyle(.orange).padding()
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
                        Text("Snapshot schema \(entry.schemaVersion)").font(.caption).foregroundStyle(
                            .secondary)
                    }.tag(entry.id)
                }
                if let entry = entries.first(where: { $0.id == selectedEntryID }) {
                    Divider()
                    ScrollView {
                        Text(historyPreview(for: entry)).font(.caption.monospaced()).textSelection(
                            .enabled
                        ).frame(maxWidth: .infinity, alignment: .leading).padding(10)
                    }
                    .frame(maxHeight: 150)
                    Button("Restore") {
                        NotificationCenter.default.post(name: .restoreUtilitySnapshot, object: entry)
                    }.padding(.horizontal).padding(.bottom, 8)
                }
            }
            Divider()
            HoldToConfirmButton(title: "Hold to Clear Utility", duration: 1) {
                try? model.history.clear(utilityID)
            }
            .padding(10)
            .accessibilityAction { showClearConfirmation = true }
        }
        .background(Color(nsColor: .controlBackgroundColor))
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
