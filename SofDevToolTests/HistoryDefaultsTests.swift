import Foundation
import Testing

@testable import SofDevTool

@Suite("Utility History defaults", .serialized)
@MainActor
struct HistoryDefaultsTests {
    @Test func utilityDefaultIsUsedUntilTheOwnerSavesAnOverride() throws {
        let suite = "SofDevToolTests.history-defaults.\(UUID())"
        let defaults = try #require(UserDefaults(suiteName: suite))
        let directory = FileManager.default.temporaryDirectory.appending(
            path: "SofDevToolTests-\(UUID())", directoryHint: .isDirectory)
        defer {
            defaults.removePersistentDomain(forName: suite)
            try? FileManager.default.removeItem(at: directory)
        }
        let repository = HistoryRepository(directory: directory, defaults: defaults)
        let snapshot = UtilityOperationSnapshot(
            utilityID: "jwt-decoder", schemaVersion: 1, payload: Data("private".utf8))

        try repository.record(snapshot, defaultEnabled: false)
        #expect(repository.entries(for: "jwt-decoder").isEmpty)

        repository.setEnabled(true, for: "jwt-decoder")
        try repository.record(snapshot, defaultEnabled: false)
        #expect(repository.entries(for: "jwt-decoder").count == 1)
    }
}
