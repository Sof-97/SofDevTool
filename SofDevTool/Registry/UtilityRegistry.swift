import SwiftUI

enum UtilityCategory: String, CaseIterable, Codable {
    case formatConvert = "Format & Convert"
    case encodeInspect = "Encode & Inspect"
    case generate = "Generate"
    case compareTest = "Compare & Test"
}

struct UtilityWorkspaceContext {
    let clipboard: ClipboardAdapter
    let record: (UtilityOperationSnapshot) -> Void
}

struct UtilityDefinition: Identifiable {
    let id: String
    let name: String
    let category: UtilityCategory
    let aliases: [String]
    let symbol: String
    let historyEnabledByDefault: Bool
    let workspaceFactory: (UtilityWorkspaceContext) -> AnyView
}

@MainActor
struct UtilityRegistry {
    let definitions: [UtilityDefinition]

    func definition(id: String) -> UtilityDefinition? { definitions.first { $0.id == id } }

    func search(_ query: String, within source: [UtilityDefinition]? = nil) -> [UtilityDefinition] {
        let source = source ?? definitions
        let terms = query.lowercased().split(whereSeparator: \.isWhitespace)
        guard !terms.isEmpty else { return source }
        return source.filter { definition in
            let haystack = ([definition.name, definition.category.rawValue] + definition.aliases).joined(
                separator: " "
            ).lowercased()
            return terms.allSatisfy(haystack.contains)
        }
    }

    static let standard = UtilityRegistry(definitions: [
        UtilityDefinition(
            id: "json", name: "JSON", category: .formatConvert,
            aliases: ["format", "validate", "minify", "query"], symbol: "curlybraces",
            historyEnabledByDefault: true
        ) { AnyView(JSONWorkspace(context: $0)) },
        UtilityDefinition(
            id: "base64", name: "Base64", category: .encodeInspect,
            aliases: ["encode", "decode", "utf8", "url safe"], symbol: "textformat.abc",
            historyEnabledByDefault: true
        ) { AnyView(Base64Workspace(context: $0)) },
        UtilityDefinition(
            id: "identifiers", name: "Identifier Generator", category: .generate,
            aliases: ["uuid", "ulid", "ksuid", "sortable"], symbol: "number.square",
            historyEnabledByDefault: true
        ) { AnyView(IdentifierWorkspace(context: $0)) },
        UtilityDefinition(
            id: "random-string", name: "Random String", category: .generate,
            aliases: ["secure", "token", "characters", "entropy"], symbol: "dice",
            historyEnabledByDefault: true
        ) { AnyView(RandomStringWorkspace(context: $0)) },
        UtilityDefinition(
            id: "text-diff", name: "Text Diff", category: .compareTest,
            aliases: ["compare", "difference", "unified", "split"],
            symbol: "arrow.left.arrow.right",
            historyEnabledByDefault: true
        ) { AnyView(TextDiffWorkspace(context: $0)) },
    ])
}
