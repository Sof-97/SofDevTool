import Foundation
import Testing

@testable import SofDevTool

@Suite("Timestamps Utility")
struct TimestampsUtilityTests {
    private let rome = TimeZone(identifier: "Europe/Rome")!

    @Test func autoUsesDocumentedDigitRulesAndRequiresOffsetForISO() throws {
        #expect(try TimestampsEngine.parse("0", mode: .auto, timeZone: rome).inference == .unixSeconds)
        #expect(
            try TimestampsEngine.parse("1700000000000", mode: .auto, timeZone: rome).inference
                == .unixMilliseconds
        )
        #expect(throws: UtilityError.self) {
            try TimestampsEngine.parse("17000000000", mode: .auto, timeZone: rome)
        }
        #expect(throws: UtilityError.self) {
            try TimestampsEngine.parse("170000000000", mode: .auto, timeZone: rome)
        }
        #expect(throws: UtilityError.self) {
            try TimestampsEngine.parse("2026-08-31T12:00:00", mode: .auto, timeZone: rome)
        }
        #expect(
            try TimestampsEngine.parse("2026-08-31T12:00:00+02:00", mode: .auto, timeZone: rome)
                .inference == .iso8601
        )
    }

    @Test func explicitSecondsAcceptFractionsAndNegativeEpochs() throws {
        let result = try TimestampsEngine.parse("-0.5", mode: .unixSeconds, timeZone: rome)

        #expect(result.instant.timeIntervalSince1970 == -0.5)
        #expect(result.inference == .unixSeconds)
        #expect(throws: UtilityError.self) {
            try TimestampsEngine.parse("1.5", mode: .unixMilliseconds, timeZone: rome)
        }
        #expect(throws: UtilityError.self) {
            try TimestampsEngine.parse("1e3", mode: .unixSeconds, timeZone: rome)
        }
    }

    @Test func renderingIsLocaleStableAndNamesSelectedTimezone() throws {
        let result = try TimestampsEngine.parse("0", mode: .unixSeconds, timeZone: rome)

        #expect(result.iso8601 == "1970-01-01T00:00:00Z")
        #expect(result.utc == "1970-01-01 00:00:00 UTC")
        #expect(result.selectedTimeZone == "1970-01-01 01:00:00 Europe/Rome")
        #expect(result.timeZoneIdentifier == "Europe/Rome")
    }

    @Test func localTimeRejectsNonexistentAndRepeatedDSTWallTimes() {
        #expect(throws: UtilityError.self) {
            try TimestampsEngine.parse("2026-03-29 02:30:00", mode: .localTime, timeZone: rome)
        }
        #expect(throws: UtilityError.self) {
            try TimestampsEngine.parse("2026-10-25 02:30:00", mode: .localTime, timeZone: rome)
        }
        let ordinary = try? TimestampsEngine.parse(
            "2026-01-15 12:30:00", mode: .localTime, timeZone: rome)
        #expect(ordinary != nil)
    }

    @Test func nowUsesInjectedClockAndRestoreRetainsCapturedInstant() throws {
        let fixed = Date(timeIntervalSince1970: 1_700_000_000.125)
        var session = TimestampsSession(clock: { fixed }, timeZone: rome)

        session.now()
        let pendingSnapshot = session.consumeSnapshot()
        let snapshot = try #require(pendingSnapshot)
        let encoded = try JSONEncoder().encode(snapshot)
        let decoded = try JSONDecoder().decode(TimestampsSnapshot.self, from: encoded)
        var restored = TimestampsSession(
            clock: { Date(timeIntervalSince1970: 2_000_000_000) },
            timeZone: TimeZone(identifier: "UTC")!
        )
        restored.restore(decoded)

        #expect(snapshot.result.instant == fixed)
        #expect(restored.result?.instant == fixed)
        #expect(restored.timeZone.identifier == "Europe/Rome")
        #expect(restored.consumeSnapshot() == nil)
    }

    @Test func onlyLatestRevisionCanPublish() {
        var session = TimestampsSession(timeZone: rome)
        let first = session.beginRevision()
        let second = session.beginRevision()

        #expect(!session.accepts(first))
        #expect(session.accepts(second))
    }
}
