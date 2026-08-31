import SwiftUI

enum TextDiffDisplayMode: String, CaseIterable, Codable, Hashable, Identifiable {
    case split = "Split"
    case unified = "Unified"

    var id: Self { self }
}

struct TextDiffRenderRequest: Equatable, Identifiable {
    let id: UUID
    let oldText: String
    let newText: String
    let filename: String
    let displayMode: TextDiffDisplayMode
}

enum TextDiffRendererEvent: Equatable {
    case ready
    case failed(String)
}

@MainActor
protocol TextDiffRenderer {
    func render(
        request: TextDiffRenderRequest,
        onEvent: @escaping (TextDiffRendererEvent) -> Void
    ) -> AnyView
}

enum TextDiffRenderPolicy {
    static func requiresWholeLineHighlighting(old: String, new: String) -> Bool {
        containsComplexEmoji(old) || containsComplexEmoji(new)
    }

    private static func containsComplexEmoji(_ value: String) -> Bool {
        value.contains { character in
            character.unicodeScalars.count > 1
                && character.unicodeScalars.contains { $0.properties.isEmoji }
        }
    }
}
