import AppKit
import Foundation
import SwiftUI
import Testing

@testable import SofDevTool

@Suite("Utility Registry", .serialized)
@MainActor
struct RegistryTests {
    @Test func catalogHasStableUniqueMetadata() {
        let definitions = UtilityRegistry.standard.definitions
        #expect(
            definitions.map(\.id)
                == ["json", "base64", "identifiers", "random-string", "text-diff"])
        #expect(Set(definitions.map(\.id)).count == definitions.count)
        #expect(definitions.allSatisfy { !$0.name.isEmpty && !$0.aliases.isEmpty })
        #expect(definitions.allSatisfy { UtilityCategory.allCases.contains($0.category) })
    }

    @Test func searchUsesNamesAliasesAndCategories() {
        let registry = UtilityRegistry.standard
        #expect(registry.search("sortable").map(\.id) == ["identifiers"])
        #expect(registry.search("encode utf8").map(\.id) == ["base64"])
        #expect(registry.search("generate").map(\.id) == ["identifiers", "random-string"])
        #expect(registry.search("compare unified").map(\.id) == ["text-diff"])
        #expect(registry.search("not-present").isEmpty)
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
