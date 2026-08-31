import PierreDiffsSwift
import SwiftUI

struct PierreTextDiffRenderer: TextDiffRenderer {
    func render(
        request: TextDiffRenderRequest,
        onEvent: @escaping (TextDiffRendererEvent) -> Void
    ) -> AnyView {
        AnyView(
            PierreTextDiffRendererView(request: request, onEvent: onEvent)
                .id(request.id)
        )
    }
}

private struct PierreTextDiffRendererView: View {
    let request: TextDiffRenderRequest
    let onEvent: (TextDiffRendererEvent) -> Void
    @State private var reportedReady = false

    var body: some View {
        PierreDiffView(
            oldContent: request.oldText,
            newContent: request.newText,
            fileName: request.filename,
            diffStyle: .constant(request.displayMode == .split ? .split : .unified),
            overflowMode: .constant(.scroll),
            renderOptions: PierreDiffRenderOptions(
                diffIndicators: .bars,
                hunkSeparators: .lineInfo,
                lineDiffType: TextDiffRenderPolicy.requiresWholeLineHighlighting(
                    old: request.oldText,
                    new: request.newText
                ) ? .none : .wordAlt,
                tokenizeMaxLength: 500_000,
                tokenizeMaxLineLength: 10_000
            ),
            onError: { onEvent(.failed($0)) },
            onReady: {
                guard !reportedReady else { return }
                reportedReady = true
                onEvent(.ready)
            }
        )
        .accessibilityLabel("Rendered text difference")
        .accessibilityIdentifier("text-diff.rendered")
    }
}
