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
    let summary: String
    let category: UtilityCategory
    let aliases: [String]
    let symbol: String
    let historyEnabledByDefault: Bool
    let historyPreview: (UtilityHistoryEntry) -> String
    let workspaceFactory: (UtilityWorkspaceContext) -> AnyView
}

enum UtilityHistoryPreview {
    static let unavailable = "This snapshot is unavailable to this app version."

    static func decode<Snapshot: Codable>(
        _ type: Snapshot.Type, schemaVersion: Int = 1
    ) -> (UtilityHistoryEntry) -> String {
        { entry in
            guard entry.schemaVersion == schemaVersion,
                let snapshot = try? JSONDecoder().decode(type, from: entry.payload)
            else { return unavailable }
            let encoder = JSONEncoder()
            encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
            guard let data = try? encoder.encode(snapshot),
                let text = String(data: data, encoding: .utf8)
            else { return unavailable }
            return text
        }
    }
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
            let haystack =
                ([definition.name, definition.summary, definition.category.rawValue] + definition.aliases)
                .joined(separator: " ").lowercased()
            return terms.allSatisfy(haystack.contains)
        }
    }

    static let standard = UtilityRegistry(definitions: [
        UtilityDefinition(
            id: "json", name: "JSON", summary: "Format, minify, validate, and query JSON",
            category: .formatConvert,
            aliases: ["format", "validate", "minify", "query"], symbol: "curlybraces",
            historyEnabledByDefault: true,
            historyPreview: UtilityHistoryPreview.decode(JSONSnapshot.self)
        ) { AnyView(JSONWorkspace(context: $0)) },
        UtilityDefinition(
            id: "yaml-json", name: "YAML / JSON", summary: "Convert YAML and JSON documents",
            category: .formatConvert,
            aliases: ["yaml", "json", "convert", "anchor", "alias"],
            symbol: "arrow.left.arrow.right.square",
            historyEnabledByDefault: true,
            historyPreview: UtilityHistoryPreview.decode(YAMLJSONSnapshot.self)
        ) { AnyView(YAMLJSONWorkspace(context: $0)) },
        UtilityDefinition(
            id: "base64", name: "Base64", summary: "Encode and decode UTF-8 text",
            category: .encodeInspect,
            aliases: ["encode", "decode", "utf8", "url safe"], symbol: "textformat.abc",
            historyEnabledByDefault: true,
            historyPreview: UtilityHistoryPreview.decode(Base64Snapshot.self)
        ) { AnyView(Base64Workspace(context: $0)) },
        UtilityDefinition(
            id: "url-encoding", name: "URL Encoding", summary: "Encode path segments and query values",
            category: .encodeInspect,
            aliases: ["percent", "uri", "path segment", "query value", "encode", "decode"],
            symbol: "percent",
            historyEnabledByDefault: true,
            historyPreview: UtilityHistoryPreview.decode(URLEncodingSnapshot.self)
        ) { AnyView(URLEncodingWorkspace(context: $0)) },
        UtilityDefinition(
            id: "hashes", name: "Hashes", summary: "Create cryptographic and legacy text digests",
            category: .encodeInspect,
            aliases: [
                "hash", "digest", "checksum", "sha", "sha256", "sha384", "sha512", "sha1",
                "md5", "legacy",
            ],
            symbol: "number",
            historyEnabledByDefault: true,
            historyPreview: UtilityHistoryPreview.decode(HashesSnapshot.self)
        ) { AnyView(HashesWorkspace(context: $0)) },
        UtilityDefinition(
            id: "identifiers", name: "Identifier Generator", summary: "Generate UUID, ULID, and KSUID values",
            category: .generate,
            aliases: ["uuid", "ulid", "ksuid", "sortable"], symbol: "number.square",
            historyEnabledByDefault: true,
            historyPreview: UtilityHistoryPreview.decode(IdentifierSnapshot.self)
        ) { AnyView(IdentifierWorkspace(context: $0)) },
        UtilityDefinition(
            id: "timestamps", name: "Timestamps", summary: "Convert Unix, ISO 8601, and local times",
            category: .formatConvert,
            aliases: ["date", "time", "unix", "epoch", "iso 8601", "utc", "timezone"],
            symbol: "clock",
            historyEnabledByDefault: true,
            historyPreview: UtilityHistoryPreview.decode(TimestampsSnapshot.self)
        ) { AnyView(TimestampsWorkspace(context: $0)) },
        UtilityDefinition(
            id: "jwt-decoder", name: "JWT Decoder",
            summary: "Inspect readable token segments without verification",
            category: .encodeInspect,
            aliases: ["jwt", "token", "header", "payload", "base64url"],
            symbol: "doc.text.magnifyingglass",
            historyEnabledByDefault: false,
            historyPreview: UtilityHistoryPreview.decode(JWTDecoderSnapshot.self)
        ) { AnyView(JWTDecoderWorkspace(context: $0)) },
        UtilityDefinition(
            id: "regex", name: "Regex", summary: "Test ICU patterns, captures, and replacements",
            category: .compareTest,
            aliases: ["regular expression", "icu", "match", "capture", "replace"],
            symbol: "text.magnifyingglass",
            historyEnabledByDefault: true,
            historyPreview: UtilityHistoryPreview.decode(RegexSnapshot.self)
        ) { AnyView(RegexWorkspace(context: $0)) },
        UtilityDefinition(
            id: "case-conversion", name: "Case Conversion",
            summary: "Inspect words and convert developer text casing",
            category: .formatConvert,
            aliases: ["camel", "pascal", "snake", "kebab", "title", "words"],
            symbol: "textformat",
            historyEnabledByDefault: true,
            historyPreview: UtilityHistoryPreview.decode(CaseConversionSnapshot.self)
        ) { AnyView(CaseConversionWorkspace(context: $0)) },
        UtilityDefinition(
            id: "whitespace-conversion", name: "Whitespace Conversion",
            summary: "Transform spacing, indentation, and line endings",
            category: .formatConvert,
            aliases: [
                "trim", "collapse", "tabs", "spaces", "indent", "line endings", "crlf", "dedent",
            ],
            symbol: "text.alignleft",
            historyEnabledByDefault: true,
            historyPreview: UtilityHistoryPreview.decode(WhitespaceConversionSnapshot.self)
        ) { AnyView(WhitespaceConversionWorkspace(context: $0)) },
        UtilityDefinition(
            id: "color-conversion", name: "Color Conversion",
            summary: "Convert sRGB colors between CSS representations",
            category: .formatConvert,
            aliases: ["hex", "rgb", "rgba", "hsl", "hsla", "srgb", "css"],
            symbol: "paintpalette",
            historyEnabledByDefault: true,
            historyPreview: UtilityHistoryPreview.decode(ColorConversionSnapshot.self)
        ) { AnyView(ColorConversionWorkspace(context: $0)) },
        UtilityDefinition(
            id: "sample-data", name: "Sample Data", summary: "Generate fictional structured JSON or CSV rows",
            category: .generate,
            aliases: ["fictional", "mock", "fake", "json", "csv", "rows", "uuid", "email"],
            symbol: "tablecells",
            historyEnabledByDefault: true,
            historyPreview: UtilityHistoryPreview.decode(SampleDataSnapshot.self)
        ) { AnyView(SampleDataWorkspace(context: $0)) },
        UtilityDefinition(
            id: "random-string", name: "Random String", summary: "Generate cryptographically random text",
            category: .generate,
            aliases: ["secure", "token", "characters", "entropy"], symbol: "dice",
            historyEnabledByDefault: true,
            historyPreview: UtilityHistoryPreview.decode(RandomStringSnapshot.self)
        ) { AnyView(RandomStringWorkspace(context: $0)) },
        UtilityDefinition(
            id: "text-diff", name: "Text Diff", summary: "Compare text with split or unified changes",
            category: .compareTest,
            aliases: ["compare", "difference", "unified", "split"],
            symbol: "arrow.left.arrow.right",
            historyEnabledByDefault: true,
            historyPreview: UtilityHistoryPreview.decode(TextDiffSnapshot.self)
        ) { AnyView(TextDiffWorkspace(context: $0)) },
    ])
}
