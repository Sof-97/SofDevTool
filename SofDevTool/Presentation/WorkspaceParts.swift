import SwiftUI

struct TextEditorCard: View {
    let title: String
    @Binding var text: String
    var editable = true
    var accessibilityID: String

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(title).font(.caption).foregroundStyle(.secondary)
            TextEditor(text: $text)
                .font(.body.monospaced())
                .scrollContentBackground(.hidden)
                .padding(8)
                .background(Color(nsColor: .textBackgroundColor), in: RoundedRectangle(cornerRadius: 8))
                .overlay(RoundedRectangle(cornerRadius: 8).stroke(.separator))
                .disabled(!editable)
                .accessibilityIdentifier(accessibilityID)
        }
    }
}

struct DiagnosticBanner: View {
    let message: String
    var body: some View {
        Label(message, systemImage: "exclamationmark.triangle.fill")
            .font(.callout).foregroundStyle(.orange)
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(10).background(.orange.opacity(0.12), in: RoundedRectangle(cornerRadius: 8))
            .accessibilityIdentifier("utility.diagnostic")
    }
}

struct CopyPasteActions: View {
    let canCopy: Bool
    let paste: () -> Void
    let copy: () -> Void
    let clear: () -> Void

    var body: some View {
        HStack {
            Button("Paste", action: paste).accessibilityIdentifier("utility.paste")
            Button("Clear", action: clear).accessibilityIdentifier("utility.clear")
            Spacer()
            ConfirmedCopyButton(
                title: "Copy Result", isEnabled: canCopy,
                accessibilityID: "utility.copy", action: copy)
        }
    }
}

struct ConfirmedCopyButton: View {
    let title: String
    let isEnabled: Bool
    var accessibilityID: String?
    let action: () -> Void
    @State private var copied = false

    var body: some View {
        HStack(spacing: 6) {
            if copied {
                Text("Copied").font(.caption).foregroundStyle(.green).transition(.opacity)
            }
            Button(title) {
                action()
                withAnimation { copied = true }
                Task {
                    try? await Task.sleep(for: .seconds(1))
                    withAnimation { copied = false }
                }
            }
            .disabled(!isEnabled)
            .accessibilityIdentifier(accessibilityID ?? "")
        }
    }
}
