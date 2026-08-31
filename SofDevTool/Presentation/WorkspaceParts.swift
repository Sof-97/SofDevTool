import SwiftUI

struct TextEditorCard: View {
    @Environment(\.themePalette) private var palette
    let title: String
    @Binding var text: String
    var editable = true
    var accessibilityID: String

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(title).font(.caption.weight(.semibold)).foregroundStyle(palette.secondaryText.color)
            TextEditor(text: $text)
                .font(.body.monospaced())
                .scrollContentBackground(.hidden)
                .padding(8)
                .foregroundStyle(palette.codeText.color)
                .background(palette.editorSurface.color, in: RoundedRectangle(cornerRadius: 8))
                .overlay(
                    RoundedRectangle(cornerRadius: 8)
                        .stroke(palette.separator.color.opacity(0.46))
                )
                .disabled(!editable)
                .accessibilityIdentifier(accessibilityID)
        }
    }
}

struct DiagnosticBanner: View {
    @Environment(\.themePalette) private var palette
    let message: String
    var body: some View {
        Label(message, systemImage: "exclamationmark.triangle.fill")
            .font(.callout).foregroundStyle(palette.error.color)
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(10)
            .background(palette.error.color.opacity(0.11), in: RoundedRectangle(cornerRadius: 8))
            .overlay(
                RoundedRectangle(cornerRadius: 8).stroke(palette.error.color.opacity(0.38))
            )
            .accessibilityIdentifier("utility.diagnostic")
    }
}

struct CopyPasteActions: View {
    @Environment(\.themePalette) private var palette
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
        .tint(palette.appAccent.color)
    }
}

struct ConfirmedCopyButton: View {
    @Environment(\.themePalette) private var palette
    let title: String
    let isEnabled: Bool
    var accessibilityID: String?
    let action: () -> Void
    @State private var copied = false

    var body: some View {
        HStack(spacing: 6) {
            if copied {
                Label("Copied", systemImage: "checkmark.circle.fill")
                    .font(.caption)
                    .foregroundStyle(palette.success.color)
                    .transition(.opacity)
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
