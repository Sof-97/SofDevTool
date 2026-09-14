import AppKit
import Foundation
import SwiftUI
import Testing

@testable import SofDevTool

@Suite("Build Identity")
struct BuildIdentityTests {
    @Test func reportsConfiguredVersionBuildAndChannel() {
        let identity = BuildIdentity(
            channel: .debug,
            infoDictionary: [
                "CFBundleShortVersionString": "1.2.3",
                "CFBundleVersion": "42",
            ]
        )

        #expect(identity.channel == .debug)
        #expect(identity.isDevelopment)
        #expect(identity.versionDescription == "1.2.3 (42)")
    }

    @Test func missingBundleMetadataIsExplicit() {
        let identity = BuildIdentity(channel: .release, infoDictionary: [:])

        #expect(!identity.isDevelopment)
        #expect(identity.versionDescription == "Unknown (Unknown)")
    }
}

@Suite("Application Theme", .serialized)
@MainActor
struct ApplicationThemeTests {
    @Test func freshPreferencesUseGraphiteAndSelectionPersistsAcrossModels() throws {
        let suite = "SofDevToolTests.theme.\(UUID())"
        let defaults = try #require(UserDefaults(suiteName: suite))
        defer { defaults.removePersistentDomain(forName: suite) }

        let first = AppModel(defaults: defaults)
        #expect(first.theme == .graphite)

        first.theme = .frappe
        #expect(defaults.string(forKey: AppTheme.defaultsKey) == "frappe")
        #expect(AppModel(defaults: defaults).theme == .frappe)
    }

    @Test func unknownPersistedIdentityFallsBackToGraphite() throws {
        let suite = "SofDevToolTests.theme.invalid.\(UUID())"
        let defaults = try #require(UserDefaults(suiteName: suite))
        defer { defaults.removePersistentDomain(forName: suite) }
        defaults.set("mocha", forKey: AppTheme.defaultsKey)

        #expect(AppModel(defaults: defaults).theme == .graphite)
    }

    @Test func semanticPalettesPinApprovedCanonicalValues() {
        let graphite = AppTheme.graphite.palette
        #expect(graphite.windowBackground.hex == 0x09_0B_10)
        #expect(graphite.secondaryPane.hex == 0x11_15_1C)
        #expect(graphite.raisedSurface.hex == 0x17_1C_25)
        #expect(graphite.editorSurface.hex == 0x0D_11_18)
        #expect(graphite.primaryText.hex == 0xF3_F5_F8)
        #expect(graphite.appAccent.hex == 0x7C_83_FF)
        #expect(graphite.success.hex == 0x4B_D1_8B)
        #expect(graphite.warning.hex == 0xFF_B4_54)
        #expect(graphite.error.hex == 0xFF_6B_7A)

        let frappe = AppTheme.frappe.palette
        #expect(frappe.windowBackground.hex == 0x30_34_46)
        #expect(frappe.secondaryPane.hex == 0x29_2C_3C)
        #expect(frappe.raisedSurface.hex == 0x41_45_59)
        #expect(frappe.editorSurface.hex == 0x23_26_34)
        #expect(frappe.primaryText.hex == 0xC6_D0_F5)
        #expect(frappe.appAccent.hex == 0xCA_9E_E6)
        #expect(frappe.secondaryAccent.hex == 0x81_C8_BE)
        #expect(frappe.success.hex == 0xA6_D1_89)
        #expect(frappe.warning.hex == 0xE5_C8_90)
        #expect(frappe.error.hex == 0xE7_82_84)
    }
}

@Suite("Utility Registry", .serialized)
@MainActor
struct RegistryTests {
    @Test func catalogHasStableUniqueMetadata() {
        let definitions = UtilityRegistry.standard.definitions
        #expect(
            definitions.map(\.id)
                == [
                    "json", "yaml-json", "base64", "url-encoding", "hashes", "identifiers",
                    "timestamps", "jwt-decoder", "regex", "case-conversion",
                    "whitespace-conversion", "color-conversion", "sample-data", "random-string",
                    "text-diff",
                ])
        #expect(Set(definitions.map(\.id)).count == definitions.count)
        #expect(
            definitions.allSatisfy {
                !$0.name.isEmpty && !$0.summary.isEmpty && $0.summary.count <= 72
                    && !$0.aliases.isEmpty
            })
        #expect(definitions.allSatisfy { UtilityCategory.allCases.contains($0.category) })
    }

    @Test func searchUsesNamesAliasesAndCategories() {
        let registry = UtilityRegistry.standard
        #expect(registry.search("sortable").map(\.id) == ["identifiers"])
        #expect(registry.search("encode utf8").map(\.id) == ["base64"])
        #expect(
            registry.search("generate").map(\.id)
                == ["identifiers", "sample-data", "random-string"])
        #expect(registry.search("compare unified").map(\.id) == ["text-diff"])
        #expect(registry.search("yaml alias").map(\.id) == ["yaml-json"])
        #expect(registry.search("percent query").map(\.id) == ["url-encoding"])
        #expect(registry.search("legacy md5").map(\.id) == ["hashes"])
        #expect(registry.search("icu capture").map(\.id) == ["regex"])
        #expect(registry.search("fictional csv").map(\.id) == ["sample-data"])
        #expect(registry.search("readable token segments").map(\.id) == ["jwt-decoder"])
        #expect(registry.definition(id: "jwt-decoder")?.historyEnabledByDefault == false)
        #expect(registry.search("not-present").isEmpty)
    }

    @Test func historyPreviewUsesTheOwningUtilitySnapshotDecoder() throws {
        let snapshot = URLEncodingSnapshot(
            options: URLEncodingOptions(), input: "café", output: "caf%C3%A9")
        let entry = UtilityHistoryEntry(
            id: UUID(), capturedAt: .now, utilityID: "url-encoding", schemaVersion: 1,
            payload: try JSONEncoder().encode(snapshot))
        let definition = try #require(UtilityRegistry.standard.definition(id: "url-encoding"))

        #expect(definition.historyPreview(entry).contains("caf%C3%A9"))

        let mismatched = UtilityHistoryEntry(
            id: UUID(), capturedAt: .now, utilityID: "url-encoding", schemaVersion: 1,
            payload: Data(#"{"unrelated":true}"#.utf8))
        #expect(definition.historyPreview(mismatched) == UtilityHistoryPreview.unavailable)
    }
}

@Suite("Text Diff Utility")
@MainActor
struct TextDiffTests {
    @Test func preprocessingOwnsIgnoreCaseAndWhitespaceSemantics() {
        var options = TextDiffOptions()
        options.ignoresCase = true
        options.ignoresWhitespace = true

        let prepared = TextDiffEngine.prepare(
            old: "let café = \"Straße\"\nalpha beta",
            new: "LET CAFÉ=\"STRASSE\"\nalphabeta",
            options: options
        )

        #expect(prepared.old == "letcafé=\"strasse\"\nalphabeta")
        #expect(prepared.new == "letcafé=\"strasse\"\nalphabeta")
    }

    @Test func preprocessingPreservesLineStructureAndUnicodeWithoutIgnoreOptions() {
        let prepared = TextDiffEngine.prepare(
            old: "café 👩🏽‍💻\n\nvalue",
            new: "CAFÉ 👩🏽‍🚀\n\nvalue",
            options: TextDiffOptions()
        )

        #expect(prepared.old == "café 👩🏽‍💻\n\nvalue")
        #expect(prepared.new == "CAFÉ 👩🏽‍🚀\n\nvalue")
    }

    @Test func complexEmojiUsesDisclosedWholeLineFallback() {
        #expect(
            TextDiffRenderPolicy.requiresWholeLineHighlighting(
                old: "developer 👩🏽‍💻", new: "astronaut 👩🏽‍🚀"))
        #expect(TextDiffRenderPolicy.requiresWholeLineHighlighting(old: "flag 🇮🇹", new: "flag 🇪🇺"))
        #expect(TextDiffRenderPolicy.requiresWholeLineHighlighting(old: "key 1️⃣", new: "key 2️⃣"))
        #expect(!TextDiffRenderPolicy.requiresWholeLineHighlighting(old: "ok ✅", new: "go 🚀"))
        #expect(!TextDiffRenderPolicy.requiresWholeLineHighlighting(old: "alpha", new: "beta"))
    }

    @Test func rendererSeamCarriesOnlyApplicationOwnedTypesAndEvents() {
        let renderer = RecordingTextDiffRenderer()
        let request = TextDiffRenderRequest(
            id: UUID(), oldText: "café", newText: "CAFÉ", filename: "Sample.swift",
            displayMode: .unified)
        var receivedEvent: TextDiffRendererEvent?

        _ = renderer.render(request: request) { receivedEvent = $0 }

        #expect(renderer.requests == [request])
        #expect(receivedEvent == .ready)
    }
}

@MainActor
private final class RecordingTextDiffRenderer: TextDiffRenderer {
    var requests: [TextDiffRenderRequest] = []

    func render(
        request: TextDiffRenderRequest,
        onEvent: @escaping (TextDiffRendererEvent) -> Void
    ) -> AnyView {
        requests.append(request)
        onEvent(.ready)
        return AnyView(EmptyView())
    }
}

@Suite("Launcher integration", .serialized)
@MainActor
struct LauncherTests {
    @Test func panelCanRemainVisibleWhileAnotherApplicationIsActive() throws {
        let suite = "SofDevToolTests.\(UUID())"
        let defaults = try #require(UserDefaults(suiteName: suite))
        defer { defaults.removePersistentDomain(forName: suite) }

        let controller = LauncherWindowController(model: AppModel(defaults: defaults))
        let panel = try #require(controller.window as? NSPanel)

        #expect(panel.hidesOnDeactivate == false)
        #expect(panel.collectionBehavior.contains(.fullScreenAuxiliary))
        #expect(panel.collectionBehavior.contains(.canJoinAllSpaces))
    }

    @Test func conflictingRegistrationPreservesTheLastWorkingShortcut() throws {
        let firstSuite = "SofDevToolTests.first.\(UUID())"
        let secondSuite = "SofDevToolTests.second.\(UUID())"
        let firstDefaults = try #require(UserDefaults(suiteName: firstSuite))
        let secondDefaults = try #require(UserDefaults(suiteName: secondSuite))
        defer {
            firstDefaults.removePersistentDomain(forName: firstSuite)
            secondDefaults.removePersistentDomain(forName: secondSuite)
        }

        let occupied = ShortcutConfiguration(
            keyCode: 80, carbonModifiers: 6_656, displayName: "⌃⌥⇧F19")
        let first = GlobalShortcutController(defaults: firstDefaults) {}
        let second = GlobalShortcutController(defaults: secondDefaults) {}

        #expect(first.register(occupied) == nil)
        #expect(second.register(occupied) != nil)
        #expect(second.configuration == .defaultLauncher)
        #expect(secondDefaults.object(forKey: "launcher.shortcut.keyCode") == nil)
    }

    @Test func savedShortcutIsRestoredOnRelaunch() throws {
        let suite = "SofDevToolTests.saved.\(UUID())"
        let defaults = try #require(UserDefaults(suiteName: suite))
        defer { defaults.removePersistentDomain(forName: suite) }
        defaults.set(79, forKey: "launcher.shortcut.keyCode")
        defaults.set(6_656, forKey: "launcher.shortcut.modifiers")
        defaults.set("⌃⌥⇧F18", forKey: "launcher.shortcut.displayName")

        let controller = GlobalShortcutController(defaults: defaults) {}

        #expect(controller.registerSavedOrDefault() == nil)
        #expect(controller.configuration.displayName == "⌃⌥⇧F18")
    }
}

@Suite("JSON Utility")
struct JSONTests {
    @Test func formatMinifySortAndQuery() throws {
        let input = #"{"z":1,"a":[true,{"name":"Sof"}]}"#
        let formatted = try JSONEngine.transform(input, options: JSONOptions())
        #expect(formatted.contains("\n"))
        var minify = JSONOptions()
        minify.mode = .minify
        #expect(try JSONEngine.transform(input, options: minify) == input)
        var sorted = minify
        sorted.sortedKeys = true
        #expect(try JSONEngine.transform(input, options: sorted).hasPrefix(#"{"a"#))
        var pointer = JSONOptions()
        pointer.mode = .query
        pointer.query = "/a/1/name"
        #expect(try JSONEngine.transform(input, options: pointer) == "Sof")
        pointer.query = "a[0]"
        #expect(try JSONEngine.transform(input, options: pointer) == "1")
    }

    @Test func malformedAndMissingPathsFailWithoutOutput() {
        #expect(throws: (any Error).self) { try JSONEngine.transform("{", options: JSONOptions()) }
        var options = JSONOptions()
        options.mode = .query
        options.query = "/missing"
        #expect(throws: UtilityError.self) { try JSONEngine.transform(#"{"ok":true}"#, options: options) }
    }

    @Test func unicodeAndArrayOrderSurvive() throws {
        var options = JSONOptions()
        options.sortedKeys = true
        let output = try JSONEngine.transform(#"{"text":"café 👩🏽‍💻","items":[3,2,1]}"#, options: options)
        #expect(output.contains("café 👩🏽‍💻"))
        #expect(output.contains("3"))
        #expect(output.firstIndex(of: "3")! < output.firstIndex(of: "2")!)
    }
}

@Suite("Base64 Utility")
struct Base64Tests {
    @Test func standardAndURLSafeVectors() throws {
        var options = Base64Options()
        #expect(try Base64Engine.transform("hello", options: options) == "aGVsbG8=")
        options.mode = .decode
        #expect(try Base64Engine.transform("Y2Fmw6kg8J+RqfCfj73igI3wn5K7", options: options) == "café 👩🏽‍💻")
        options.mode = .encode
        options.alphabet = .urlSafe
        options.padded = false
        let encoded = try Base64Engine.transform("?ÿ", options: options)
        #expect(!encoded.contains("+") && !encoded.contains("/") && !encoded.contains("="))
        options.mode = .decode
        #expect(try Base64Engine.transform(encoded, options: options) == "?ÿ")
    }

    @Test func invalidAlphabetLengthAndUTF8Fail() {
        var options = Base64Options()
        options.mode = .decode
        #expect(throws: UtilityError.self) { try Base64Engine.transform("ab c", options: options) }
        #expect(throws: UtilityError.self) { try Base64Engine.transform("a", options: options) }
        #expect(throws: UtilityError.self) { try Base64Engine.transform("/w==", options: options) }
    }
}

@Suite("YAML and JSON Conversion Utility")
struct YAMLJSONConversionTests {
    @Test func YAMLCoreScalarsConvertToCanonicalJSON() throws {
        let input = """
            enabled: true
            legacy: yes
            created: 2026-08-31
            message: café 👩🏽‍💻
            """

        let result = try YAMLJSONEngine.convert(input, direction: .yamlToJSON)

        #expect(
            result.output
                == """
                {
                  "created" : "2026-08-31",
                  "enabled" : true,
                  "legacy" : "yes",
                  "message" : "café 👩🏽‍💻"
                }
                """)
        #expect(result.warnings == [.lossyRoundTrip])
    }

    @Test func anchorsAliasesAndUnicodeExpandIntoJSONValues() throws {
        let input = """
            profile: &profile
              name: café 👩🏽‍💻
              active: true
            copy: *profile
            """

        let output = try YAMLJSONEngine.convert(input, direction: .yamlToJSON).output

        #expect(output.components(separatedBy: "café 👩🏽‍💻").count == 3)
        #expect(output.contains("\"copy\""))
        #expect(output.contains("\"profile\""))
    }

    @Test func JSONToYAMLRoundTripIsDeterministicAndKeepsStringScalars() throws {
        let json = #"{"z":1,"a":"yes","emoji":"👩🏽‍💻","items":[null,false,1.25]}"#

        let yaml = try YAMLJSONEngine.convert(json, direction: .jsonToYAML).output
        let roundTrip = try YAMLJSONEngine.convert(yaml, direction: .yamlToJSON).output

        #expect(yaml.firstMatch(of: /a:.*yes/) != nil)
        #expect(roundTrip.firstIndex(of: "a")! < roundTrip.firstIndex(of: "z")!)
        #expect(roundTrip.contains("\"a\" : \"yes\""))
        #expect(roundTrip.contains("👩🏽‍💻"))
        #expect(roundTrip.contains("1.25"))
    }

    @Test func malformedStreamsAndUnsupportedYAMLValuesAreDiagnosed() {
        expectYAMLFailure("---\na: 1\n---\nb: 2", contains: "another document")
        expectYAMLFailure("true: value", contains: "keys must resolve to unique strings")
        expectYAMLFailure("value: .inf", contains: "Non-finite")
        expectYAMLFailure("value: !widget hello", contains: "Unsupported YAML tag")
        expectYAMLFailure("a: 1\na: 2", contains: "Duplicate YAML mapping key")
        expectYAMLFailure("items: [1,", contains: "Invalid YAML")
    }

    @Test func CoreNumericFormsAreCanonicalAndOutOfRangeIntegersFail() throws {
        let output = try YAMLJSONEngine.convert(
            "decimal: 42\noctal: 0o17\nhex: 0x10\nfraction: 1.25\nexponent: 1e2\nnegativeZero: -0",
            direction: .yamlToJSON
        ).output

        #expect(output.contains("\"decimal\" : 42"))
        #expect(output.contains("\"octal\" : 15"))
        #expect(output.contains("\"hex\" : 16"))
        #expect(output.contains("\"fraction\" : 1.25"))
        #expect(output.contains("\"exponent\" : 100"))
        #expect(output.contains("\"negativeZero\" : 0"))
        expectYAMLFailure(
            "value: 18446744073709551616",
            contains: "outside the exact JSON conversion range"
        )
    }

    @Test func ResourcePolicyRejectsOversizeAndDeepDocumentsWithoutOutput() {
        let tiny = YAMLJSONResourcePolicy(maximumInputBytes: 4, maximumNestingDepth: 128)
        #expect(throws: UtilityError.self) {
            try YAMLJSONEngine.convert("value: 1", direction: .yamlToJSON, policy: tiny)
        }

        let shallow = YAMLJSONResourcePolicy(maximumInputBytes: 1_024, maximumNestingDepth: 2)
        #expect(throws: UtilityError.self) {
            try YAMLJSONEngine.convert("[[[]]]", direction: .jsonToYAML, policy: shallow)
        }
        #expect(throws: UtilityError.self) {
            try YAMLJSONEngine.convert(
                "items:\n  - child:\n      value: 1", direction: .yamlToJSON, policy: shallow)
        }
    }

    @Test func SnapshotRoundTripRestoresDirectionInputAndSettledOutput() throws {
        let snapshot = YAMLJSONSnapshot(
            direction: .jsonToYAML,
            input: #"{"message":"café 👩🏽‍💻"}"#,
            output: "message: café 👩🏽‍💻\n"
        )

        let restored = try JSONDecoder().decode(
            YAMLJSONSnapshot.self,
            from: JSONEncoder().encode(snapshot)
        )

        #expect(restored == snapshot)
    }

    @Test func SupersededRevisionCannotPublish() {
        var gate = YAMLJSONRevisionGate()
        let first = gate.begin()
        let second = gate.begin()

        #expect(!gate.accepts(first))
        #expect(gate.accepts(second))
    }

    private func expectYAMLFailure(_ input: String, contains message: String) {
        do {
            _ = try YAMLJSONEngine.convert(input, direction: .yamlToJSON)
            Issue.record("Expected YAML conversion to fail for: \(input)")
        } catch {
            #expect(error.localizedDescription.contains(message))
        }
    }
}

private struct PredictableRandom: RandomBytesSource {
    let seed: UInt8
    func bytes(count: Int) -> [UInt8] { (0..<count).map { seed &+ UInt8($0 % 97) } }
}

@Suite("Identifier Generator")
struct IdentifierTests {
    @Test(arguments: UUIDVersion.allCases) func UUIDVersionsHaveCorrectBits(version: UUIDVersion) throws {
        let bytes = try IdentifierEngine.uuid(
            version: version, namespace: "6ba7b810-9dad-11d1-80b4-00c04fd430c8", name: "example",
            date: Date(timeIntervalSince1970: 1_700_000_000), random: PredictableRandom(seed: 7))
        #expect(bytes.count == 16)
        #expect(Int(bytes[6] >> 4) == version.rawValue)
        #expect(bytes[8] >> 6 == 2)
        let formatted = IdentifierEngine.formatUUID(bytes, uppercase: false, hyphenated: true)
        #expect(
            try IdentifierEngine.normalizeUUID(formatted.uppercased(), uppercase: false, hyphenated: true)
                == formatted)
    }

    @Test func ULIDIsCanonicalSortableAndInspectable() throws {
        let date = Date(timeIntervalSince1970: 1_700_000_000.123)
        let first = try IdentifierEngine.ulid(date: date, random: PredictableRandom(seed: 2))
        let second = try IdentifierEngine.ulid(
            date: date, random: PredictableRandom(seed: 2), previous: first.1)
        #expect(first.0.count == 26)
        #expect(first.0 < second.0)
        #expect(try IdentifierEngine.inspectULID(first.0).contains("1700000000123 ms"))
        #expect(throws: UtilityError.self) {
            try IdentifierEngine.inspectULID("8" + String(first.0.dropFirst()))
        }
    }

    @Test func KSUIDIsCanonicalCaseSensitiveAndInspectable() throws {
        let value = try IdentifierEngine.ksuid(
            date: Date(timeIntervalSince1970: 1_700_000_000), random: PredictableRandom(seed: 4))
        #expect(value.count == 27)
        #expect(try IdentifierEngine.inspectKSUID(value).contains("Valid KSUID"))
        let changedCase = value.lowercased()
        #expect(changedCase != value)
        #expect(try IdentifierEngine.inspectKSUID(changedCase).contains("Valid KSUID"))
    }
}

@Suite("Random String Utility")
struct RandomStringTests {
    @Test func respectsCountLengthAlphabetAndExclusions() throws {
        var options = RandomStringOptions()
        options.length = 32
        options.count = 4
        options.symbols = false
        let values = try RandomStringEngine.generate(options: options, random: PredictableRandom(seed: 0))
        #expect(values.count == 4)
        #expect(values.allSatisfy { $0.count == 32 })
        #expect(values.joined().allSatisfy { options.alphabet.contains($0) })
        #expect(!values.joined().contains(where: { "0O1lI".contains($0) }))
    }

    @Test func rejectsEmptyAndOutOfRangeConfiguration() {
        var options = RandomStringOptions()
        options.uppercase = false
        options.lowercase = false
        options.digits = false
        options.symbols = false
        #expect(throws: UtilityError.self) {
            try RandomStringEngine.generate(options: options, random: PredictableRandom(seed: 0))
        }
        options.length = 0
        #expect(throws: UtilityError.self) {
            try RandomStringEngine.generate(options: options, random: PredictableRandom(seed: 0))
        }
    }
}

@Suite("Utility History", .serialized)
@MainActor
struct HistoryTests {
    private func fixture() throws -> (HistoryRepository, URL, UserDefaults) {
        let directory = FileManager.default.temporaryDirectory.appending(path: UUID().uuidString)
        let suite = "SofDevToolTests.\(UUID())"
        let defaults = UserDefaults(suiteName: suite)!
        defaults.removePersistentDomain(forName: suite)
        return (HistoryRepository(directory: directory, defaults: defaults), directory, defaults)
    }

    @Test func recordsNewest25AndPersistsOpaquePayload() throws {
        let (repository, directory, defaults) = try fixture()
        defer {
            try? FileManager.default.removeItem(at: directory)
            defaults.removePersistentDomain(forName: defaults.volatileDomainNames.first ?? "")
        }
        for index in 0..<30 {
            try repository.record(
                .init(utilityID: "json", schemaVersion: 3, payload: Data("payload-\(index)".utf8)),
                at: Date(timeIntervalSince1970: Double(index)), id: UUID())
        }
        #expect(repository.entries(for: "json").count == 25)
        #expect(repository.entries(for: "json").first?.payload == Data("payload-29".utf8))
        let reloaded = HistoryRepository(directory: directory, defaults: defaults)
        #expect(reloaded.entries(for: "json").count == 25)
    }

    @Test func recordingControlsRetainThenDeletionRemoves() throws {
        let (repository, directory, _) = try fixture()
        defer { try? FileManager.default.removeItem(at: directory) }
        try repository.record(.init(utilityID: "base64", schemaVersion: 1, payload: Data([1])))
        repository.setEnabled(false, for: "base64")
        try repository.record(.init(utilityID: "base64", schemaVersion: 1, payload: Data([2])))
        #expect(repository.entries(for: "base64").count == 1)
        try repository.clear("base64")
        #expect(repository.entries(for: "base64").isEmpty)
    }

    @Test func writeFailurePreservesVisibleStateAndPauses() throws {
        let blocker = FileManager.default.temporaryDirectory.appending(path: UUID().uuidString)
        try Data("not a directory".utf8).write(to: blocker)
        defer { try? FileManager.default.removeItem(at: blocker) }
        let defaults = UserDefaults(suiteName: "SofDevToolTests.\(UUID())")!
        let repository = HistoryRepository(directory: blocker.appending(path: "History"), defaults: defaults)
        #expect(throws: (any Error).self) {
            try repository.record(.init(utilityID: "json", schemaVersion: 1, payload: Data([1])))
        }
        #expect(repository.entries(for: "json").isEmpty)
        try repository.record(.init(utilityID: "json", schemaVersion: 1, payload: Data([2])))
        #expect(repository.entries(for: "json").isEmpty)
    }
}
