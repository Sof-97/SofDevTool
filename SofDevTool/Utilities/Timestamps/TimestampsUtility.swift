import Foundation
import SwiftUI

enum TimestampInputMode: String, CaseIterable, Codable, Identifiable {
    case auto = "Auto"
    case unixSeconds = "Unix Seconds"
    case unixMilliseconds = "Unix Milliseconds"
    case iso8601 = "ISO 8601"
    case localTime = "Local Time"

    var id: Self { self }
}

enum TimestampInference: String, Codable, Equatable {
    case unixSeconds = "Unix Seconds"
    case unixMilliseconds = "Unix Milliseconds"
    case iso8601 = "ISO 8601 with explicit offset"
    case localTime = "Local Time with named timezone"
    case now = "Now action"
}

struct TimestampResult: Codable, Equatable {
    let instant: Date
    let inference: TimestampInference
    let iso8601: String
    let utc: String
    let selectedTimeZone: String
    let timeZoneIdentifier: String

    var displayText: String {
        """
        Inferred input: \(inference.rawValue)
        ISO 8601: \(iso8601)
        UTC: \(utc)
        \(timeZoneIdentifier): \(selectedTimeZone)
        """
    }
}

struct TimestampsSnapshot: Codable, Equatable {
    let mode: TimestampInputMode
    let input: String
    let result: TimestampResult
    let timeZoneIdentifier: String
}

enum TimestampsEngine {
    static func parse(
        _ input: String,
        mode: TimestampInputMode,
        timeZone: TimeZone
    ) throws -> TimestampResult {
        let trimmed = input.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { throw UtilityError("Enter a timestamp to convert.") }

        switch mode {
        case .auto:
            return try parseAuto(trimmed, timeZone: timeZone)
        case .unixSeconds:
            return try parseSeconds(trimmed, timeZone: timeZone)
        case .unixMilliseconds:
            return try parseMilliseconds(trimmed, timeZone: timeZone)
        case .iso8601:
            return try parseISO8601(trimmed, timeZone: timeZone)
        case .localTime:
            return try parseLocalTime(trimmed, timeZone: timeZone)
        }
    }

    static func render(
        instant: Date,
        inference: TimestampInference,
        timeZone: TimeZone
    ) -> TimestampResult {
        TimestampResult(
            instant: instant,
            inference: inference,
            iso8601: ISO8601Formatting.string(from: instant),
            utc: wallTimeString(instant, timeZone: TimeZone(secondsFromGMT: 0)!, label: "UTC"),
            selectedTimeZone: wallTimeString(
                instant, timeZone: timeZone, label: timeZone.identifier),
            timeZoneIdentifier: timeZone.identifier
        )
    }

    private static func parseAuto(_ input: String, timeZone: TimeZone) throws -> TimestampResult {
        if let digitCount = signedIntegerDigitCount(input) {
            switch digitCount {
            case 0...10:
                return try parseSeconds(input, timeZone: timeZone)
            case 11...12:
                throw UtilityError(
                    "An 11- or 12-digit integer is ambiguous. Choose Unix Seconds or Unix Milliseconds."
                )
            case 13:
                return try parseMilliseconds(input, timeZone: timeZone)
            default:
                throw UtilityError(
                    "Auto recognizes at most 10 digits as Unix seconds and exactly 13 digits as Unix milliseconds."
                )
            }
        }
        guard containsExplicitISOOffset(input) else {
            throw UtilityError(
                "Auto requires ISO 8601 input to include Z or an explicit offset. Choose Local Time for a named timezone."
            )
        }
        return try parseISO8601(input, timeZone: timeZone)
    }

    private static func parseSeconds(_ input: String, timeZone: TimeZone) throws -> TimestampResult {
        guard isDecimalSeconds(input), let value = Double(input), value.isFinite else {
            throw UtilityError("Unix Seconds requires a finite integer or decimal number.")
        }
        return render(
            instant: Date(timeIntervalSince1970: value),
            inference: .unixSeconds,
            timeZone: timeZone
        )
    }

    private static func parseMilliseconds(_ input: String, timeZone: TimeZone) throws
        -> TimestampResult
    {
        guard signedIntegerDigitCount(input) != nil, let value = Double(input), value.isFinite else {
            throw UtilityError("Unix Milliseconds requires a finite integer number.")
        }
        return render(
            instant: Date(timeIntervalSince1970: value / 1_000),
            inference: .unixMilliseconds,
            timeZone: timeZone
        )
    }

    private static func parseISO8601(_ input: String, timeZone: TimeZone) throws -> TimestampResult {
        guard containsExplicitISOOffset(input) else {
            throw UtilityError(
                "ISO 8601 input must include Z or an explicit offset so it identifies one instant."
            )
        }
        guard let date = ISO8601Formatting.date(from: input) else {
            throw UtilityError("Input is not a valid ISO 8601 timestamp with an explicit offset.")
        }
        return render(instant: date, inference: .iso8601, timeZone: timeZone)
    }

    private static func parseLocalTime(_ input: String, timeZone: TimeZone) throws
        -> TimestampResult
    {
        guard let values = localWallTimeComponents(input) else {
            throw UtilityError("Local Time requires YYYY-MM-DD HH:mm:ss and a named timezone.")
        }

        var calendar = Calendar(identifier: .gregorian)
        calendar.locale = Locale(identifier: "en_US_POSIX")
        calendar.timeZone = timeZone
        let requested = DateComponents(
            calendar: calendar,
            timeZone: timeZone,
            year: values.year,
            month: values.month,
            day: values.day,
            hour: values.hour,
            minute: values.minute,
            second: values.second
        )
        let dayStartComponents = DateComponents(
            calendar: calendar,
            timeZone: timeZone,
            year: values.year,
            month: values.month,
            day: values.day
        )
        guard let dayStart = calendar.date(from: dayStartComponents),
            let searchStart = calendar.date(byAdding: .second, value: -1, to: dayStart),
            let first = calendar.nextDate(
                after: searchStart,
                matching: requested,
                matchingPolicy: .strict,
                repeatedTimePolicy: .first,
                direction: .forward
            ),
            sameWallTime(first, requested: requested, calendar: calendar)
        else {
            throw UtilityError(
                "That local wall time does not exist in \(timeZone.identifier) because of a daylight-saving transition. Use an explicit offset."
            )
        }
        let last = calendar.nextDate(
            after: searchStart,
            matching: requested,
            matchingPolicy: .strict,
            repeatedTimePolicy: .last,
            direction: .forward
        )
        guard last == first else {
            throw UtilityError(
                "That local wall time repeats in \(timeZone.identifier) because of a daylight-saving transition. Use an explicit offset."
            )
        }
        return render(instant: first, inference: .localTime, timeZone: timeZone)
    }

    private static func signedIntegerDigitCount(_ value: String) -> Int? {
        let digits: Substring
        if value.first == "+" || value.first == "-" {
            digits = value.dropFirst()
        } else {
            digits = value[...]
        }
        guard !digits.isEmpty, digits.utf8.allSatisfy({ (0x30...0x39).contains($0) }) else {
            return nil
        }
        return digits.count
    }

    private static func isDecimalSeconds(_ value: String) -> Bool {
        let unsigned = (value.first == "+" || value.first == "-") ? value.dropFirst() : value[...]
        let components = unsigned.split(separator: ".", omittingEmptySubsequences: false)
        guard components.count <= 2,
            components.contains(where: { !$0.isEmpty }),
            components.allSatisfy({ $0.utf8.allSatisfy { (0x30...0x39).contains($0) } })
        else { return false }
        return components.count == 1 || !components[1].isEmpty
    }

    private static func localWallTimeComponents(_ input: String) -> (
        year: Int, month: Int, day: Int, hour: Int, minute: Int, second: Int
    )? {
        guard input.count == 19 else { return nil }
        let fields = input.replacingOccurrences(of: "T", with: " ").split(separator: " ")
        guard fields.count == 2 else { return nil }
        let date = fields[0].split(separator: "-", omittingEmptySubsequences: false)
        let time = fields[1].split(separator: ":", omittingEmptySubsequences: false)
        guard date.count == 3, time.count == 3,
            date[0].count == 4, date[1].count == 2, date[2].count == 2,
            time.allSatisfy({ $0.count == 2 }),
            let year = Int(date[0]), let month = Int(date[1]), let day = Int(date[2]),
            let hour = Int(time[0]), let minute = Int(time[1]), let second = Int(time[2])
        else { return nil }
        return (year, month, day, hour, minute, second)
    }

    private static func containsExplicitISOOffset(_ value: String) -> Bool {
        if value.hasSuffix("Z") || value.hasSuffix("z") { return true }
        guard value.count >= 6 else { return false }
        let suffix = value.suffix(6)
        return (suffix.first == "+" || suffix.first == "-")
            && suffix.dropFirst(3).first == ":"
            && suffix.dropFirst().prefix(2).allSatisfy(\.isNumber)
            && suffix.suffix(2).allSatisfy(\.isNumber)
    }

    private static func sameWallTime(
        _ date: Date,
        requested: DateComponents,
        calendar: Calendar
    ) -> Bool {
        let actual = calendar.dateComponents(
            [.year, .month, .day, .hour, .minute, .second], from: date)
        return actual.year == requested.year && actual.month == requested.month
            && actual.day == requested.day && actual.hour == requested.hour
            && actual.minute == requested.minute && actual.second == requested.second
    }

    private static func wallTimeString(_ date: Date, timeZone: TimeZone, label: String) -> String {
        let formatter = DateFormatter()
        formatter.calendar = Calendar(identifier: .gregorian)
        formatter.locale = Locale(identifier: "en_US_POSIX")
        formatter.timeZone = timeZone
        formatter.dateFormat = "yyyy-MM-dd HH:mm:ss"
        return formatter.string(from: date) + " " + label
    }
}

private enum ISO8601Formatting {
    static func date(from input: String) -> Date? {
        fractionalFormatter().date(from: input) ?? wholeSecondsFormatter().date(from: input)
    }

    static func string(from date: Date) -> String {
        let milliseconds = abs(date.timeIntervalSince1970 * 1_000).truncatingRemainder(dividingBy: 1_000)
        return milliseconds < 0.000_001
            ? wholeSecondsFormatter().string(from: date)
            : fractionalFormatter().string(from: date)
    }

    private static func wholeSecondsFormatter() -> ISO8601DateFormatter {
        let formatter = ISO8601DateFormatter()
        formatter.formatOptions = [.withInternetDateTime]
        formatter.timeZone = TimeZone(secondsFromGMT: 0)
        return formatter
    }

    private static func fractionalFormatter() -> ISO8601DateFormatter {
        let formatter = ISO8601DateFormatter()
        formatter.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
        formatter.timeZone = TimeZone(secondsFromGMT: 0)
        return formatter
    }
}

struct TimestampsSession {
    var mode = TimestampInputMode.auto
    var input = ""
    var timeZone: TimeZone
    var result: TimestampResult?
    var diagnostic: String?
    private let clock: () -> Date
    private var revision = 0
    private var pendingSnapshot: TimestampsSnapshot?

    init(clock: @escaping () -> Date = Date.init, timeZone: TimeZone = .current) {
        self.clock = clock
        self.timeZone = timeZone
    }

    mutating func beginRevision() -> Int {
        revision += 1
        return revision
    }

    func accepts(_ candidate: Int) -> Bool { candidate == revision }

    mutating func evaluate(revision candidate: Int) {
        guard accepts(candidate) else { return }
        guard !input.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
            result = nil
            diagnostic = nil
            pendingSnapshot = nil
            return
        }
        do {
            let parsed = try TimestampsEngine.parse(input, mode: mode, timeZone: timeZone)
            guard accepts(candidate) else { return }
            result = parsed
            diagnostic = nil
            pendingSnapshot = snapshot(for: parsed)
        } catch {
            result = nil
            diagnostic = error.localizedDescription
            pendingSnapshot = nil
        }
    }

    mutating func now() {
        revision += 1
        let instant = clock()
        mode = .iso8601
        input = ISO8601Formatting.string(from: instant)
        let rendered = TimestampsEngine.render(
            instant: instant,
            inference: .now,
            timeZone: timeZone
        )
        result = rendered
        diagnostic = nil
        pendingSnapshot = snapshot(for: rendered)
    }

    mutating func consumeSnapshot() -> TimestampsSnapshot? {
        defer { pendingSnapshot = nil }
        return pendingSnapshot
    }

    mutating func restore(_ snapshot: TimestampsSnapshot) {
        revision += 1
        mode = snapshot.mode
        input = snapshot.input
        timeZone = TimeZone(identifier: snapshot.timeZoneIdentifier) ?? timeZone
        result = snapshot.result
        diagnostic = nil
        pendingSnapshot = nil
    }

    mutating func clear() {
        revision += 1
        input = ""
        result = nil
        diagnostic = nil
        pendingSnapshot = nil
    }

    private func snapshot(for result: TimestampResult) -> TimestampsSnapshot {
        TimestampsSnapshot(
            mode: mode,
            input: input,
            result: result,
            timeZoneIdentifier: timeZone.identifier
        )
    }
}

struct TimestampsWorkspace: View {
    let context: UtilityWorkspaceContext
    @State private var session = TimestampsSession()
    @State private var pendingRestore: UtilityHistoryEntry?
    @State private var suppressedOperationKey: String?

    var body: some View {
        VStack(spacing: 12) {
            HStack {
                Picker("Input Mode", selection: $session.mode) {
                    ForEach(TimestampInputMode.allCases) { Text($0.rawValue).tag($0) }
                }
                .frame(width: 190)
                Picker("Timezone", selection: $session.timeZone) {
                    ForEach(TimeZone.knownTimeZoneIdentifiers, id: \.self) { identifier in
                        if let timeZone = TimeZone(identifier: identifier) {
                            Text(identifier).tag(timeZone)
                        }
                    }
                }
                .frame(width: 240)
                Button("Now") { useNow() }
                    .accessibilityIdentifier("timestamps.now")
            }
            Text(
                "Local Time uses the selected named timezone. Repeated or nonexistent daylight-saving times require an explicit offset."
            )
            .font(.caption)
            .foregroundStyle(.secondary)
            .frame(maxWidth: .infinity, alignment: .leading)
            HSplitView {
                TextEditorCard(
                    title: "Timestamp Input", text: $session.input,
                    accessibilityID: "timestamps.input"
                )
                TextEditorCard(
                    title: "Converted Instant",
                    text: .constant(session.result?.displayText ?? ""),
                    editable: false,
                    accessibilityID: "timestamps.result"
                )
            }
            if let diagnostic = session.diagnostic { DiagnosticBanner(message: diagnostic) }
            CopyPasteActions(
                canCopy: session.result != nil,
                paste: { session.input = context.clipboard.readText() ?? session.input },
                copy: { context.clipboard.writeText(session.result?.displayText ?? "") },
                clear: { session.clear() }
            )
        }
        .padding(16)
        .task(id: operationKey) {
            if suppressedOperationKey == operationKey {
                suppressedOperationKey = nil
                return
            }
            suppressedOperationKey = nil
            let revision = session.beginRevision()
            try? await Task.sleep(for: .milliseconds(200))
            guard !Task.isCancelled, session.accepts(revision) else { return }
            session.evaluate(revision: revision)
            recordPendingSnapshot()
        }
        .onReceive(NotificationCenter.default.publisher(for: .restoreUtilitySnapshot)) {
            requestRestore($0.object as? UtilityHistoryEntry)
        }
        .alert(
            "Replace current workspace?",
            isPresented: Binding(
                get: { pendingRestore != nil },
                set: { if !$0 { pendingRestore = nil } }
            )
        ) {
            Button("Restore", role: .destructive) {
                let entry = pendingRestore
                pendingRestore = nil
                restore(entry)
            }
            Button("Cancel", role: .cancel) { pendingRestore = nil }
        } message: {
            Text("Restoring this History entry replaces the current non-empty Timestamps session.")
        }
    }

    private var operationKey: String {
        session.mode.rawValue + "|" + session.timeZone.identifier + "|" + session.input
    }

    private func useNow() {
        session.now()
        suppressedOperationKey = operationKey
        recordPendingSnapshot()
    }

    private func recordPendingSnapshot() {
        guard let snapshot = session.consumeSnapshot(),
            let payload = try? JSONEncoder().encode(snapshot)
        else { return }
        context.record(.init(utilityID: "timestamps", schemaVersion: 1, payload: payload))
    }

    private func restore(_ entry: UtilityHistoryEntry?) {
        guard let entry, entry.utilityID == "timestamps",
            let snapshot = try? JSONDecoder().decode(TimestampsSnapshot.self, from: entry.payload)
        else { return }
        session.restore(snapshot)
        suppressedOperationKey = operationKey
    }

    private func requestRestore(_ entry: UtilityHistoryEntry?) {
        guard let entry, entry.utilityID == "timestamps",
            let snapshot = try? JSONDecoder().decode(TimestampsSnapshot.self, from: entry.payload)
        else { return }
        if !session.input.isEmpty, session.input != snapshot.input {
            pendingRestore = entry
        } else {
            session.restore(snapshot)
            suppressedOperationKey = operationKey
        }
    }
}
