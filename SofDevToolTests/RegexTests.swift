import Foundation
import Testing

@testable import SofDevTool

@Suite("Regex Utility", .serialized)
struct RegexTests {
    @Test func ICUFlagsMatchesCapturesAndReplacement() throws {
        let request = RegexRequest(
            pattern: #"(?<word>café)\s+(\d+)"#,
            text: "CAFÉ 12 and café 34",
            replacement: "$2:$1",
            flags: RegexFlags(caseInsensitive: true)
        )

        let result = try RegexEngine.run(request)

        #expect(result.matches.map(\.value) == ["CAFÉ 12", "café 34"])
        #expect(result.matches[0].captures[0].name == "word")
        #expect(result.matches[0].captures[0].value == "CAFÉ")
        #expect(result.matches[0].captures[1].value == "12")
        #expect(result.replacementPreview == "12:CAFÉ and 34:café")
    }

    @Test func unmatchedEmptyAndZeroLengthMatchesStayDistinct() throws {
        let captures = try RegexEngine.run(
            RegexRequest(
                pattern: #"(a)?(b*)"#, text: "x", replacement: "", flags: RegexFlags()
            ))
        #expect(captures.matches.first?.value == "")
        #expect(captures.matches.first?.captures[0].value == nil)
        #expect(captures.matches.first?.captures[1].value == "")

        let boundaries = try RegexEngine.run(
            RegexRequest(
                pattern: #"(?=.)"#, text: "👩🏽‍💻a", replacement: "|", flags: RegexFlags()
            ))
        #expect(boundaries.matches.count == 2)
        #expect(boundaries.matches.map(\.range.location) == [0, 7])
    }

    @Test func multilineDotAllAndCommentsFlagsAreICUSemantics() throws {
        var flags = RegexFlags()
        flags.multiline = true
        flags.dotMatchesNewlines = true
        flags.commentsAndWhitespace = true
        let result = try RegexEngine.run(
            RegexRequest(
                pattern: "^ a .+ z $ # comment", text: "a\nline\nz", replacement: "ok", flags: flags
            ))
        #expect(result.matches.map(\.value) == ["a\nline\nz"])
    }

    @Test func lookbehindAndInlineOptionsDoNotShiftNamedCaptures() throws {
        let result = try RegexEngine.run(
            RegexRequest(
                pattern: #"(?i)(?<=prefix:)(?<value>[a-z]+)-(\d+)"#,
                text: "PREFIX:Word-42", replacement: "$1", flags: RegexFlags()
            ))

        #expect(result.matches.count == 1)
        #expect(result.matches[0].captures.map(\.name) == ["value", nil])
        #expect(result.matches[0].captures.map(\.value) == ["Word", "42"])
    }

    @Test func invalidPatternAndAppLevelLimitsAreDistinct() {
        #expect(throws: RegexFailure.self) {
            try RegexEngine.run(RegexRequest(pattern: "(", text: "x", replacement: "", flags: RegexFlags()))
        }
        let policy = RegexExecutionPolicy(
            maximumPatternUTF16Length: 2,
            maximumTextUTF16Length: 2,
            maximumReplacementUTF16Length: 2,
            engineStepLimit: 100,
            heapBacktrackingLimitBytes: 10_000,
            elapsedDeadlineNanoseconds: 1_000_000_000,
            maximumMatches: 1,
            maximumCaptures: 1,
            maximumReplacementOutputUTF16Length: 2
        )
        #expect(throws: RegexFailure.patternTooLarge) {
            try RegexEngine.run(
                RegexRequest(pattern: "abc", text: "", replacement: "", flags: RegexFlags()), policy: policy)
        }
        #expect(throws: RegexFailure.inputTooLarge) {
            try RegexEngine.run(
                RegexRequest(pattern: "a", text: "abc", replacement: "", flags: RegexFlags()), policy: policy)
        }
        #expect(throws: RegexFailure.replacementTooLarge) {
            try RegexEngine.run(
                RegexRequest(pattern: "a", text: "a", replacement: "abc", flags: RegexFlags()), policy: policy
            )
        }
        #expect(throws: RegexFailure.matchLimit) {
            try RegexEngine.run(
                RegexRequest(pattern: ".", text: "ab", replacement: "", flags: RegexFlags()), policy: policy)
        }
        #expect(throws: RegexFailure.replacementOutputLimit) {
            let outputPolicy = RegexExecutionPolicy(
                maximumPatternUTF16Length: 2,
                maximumTextUTF16Length: 2,
                maximumReplacementUTF16Length: 2,
                engineStepLimit: 100,
                heapBacktrackingLimitBytes: 10_000,
                elapsedDeadlineNanoseconds: 1_000_000_000,
                maximumMatches: 10,
                maximumCaptures: 10,
                maximumReplacementOutputUTF16Length: 1
            )
            _ = try RegexEngine.run(
                RegexRequest(pattern: "a", text: "aa", replacement: "xx", flags: RegexFlags()),
                policy: outputPolicy
            )
        }
    }

    @Test func cancellationMapsThroughInstalledCallbacks() {
        let cancellation = RegexCancellation()
        cancellation.cancel()
        #expect(throws: RegexFailure.cancelled) {
            try RegexEngine.run(
                RegexRequest(
                    pattern: "z", text: String(repeating: "a", count: 100_000), replacement: "",
                    flags: RegexFlags()),
                cancellation: cancellation
            )
        }
    }

    @Test func stepAndHeapBudgetsAreNonzeroAndInjectable() {
        #expect(RegexExecutionPolicy.production.engineStepLimit > 0)
        #expect(RegexExecutionPolicy.production.heapBacktrackingLimitBytes > 0)

        var lowSteps = RegexExecutionPolicy.production
        lowSteps = RegexExecutionPolicy(
            maximumPatternUTF16Length: lowSteps.maximumPatternUTF16Length,
            maximumTextUTF16Length: lowSteps.maximumTextUTF16Length,
            maximumReplacementUTF16Length: lowSteps.maximumReplacementUTF16Length,
            engineStepLimit: 1,
            heapBacktrackingLimitBytes: lowSteps.heapBacktrackingLimitBytes,
            elapsedDeadlineNanoseconds: lowSteps.elapsedDeadlineNanoseconds,
            maximumMatches: lowSteps.maximumMatches,
            maximumCaptures: lowSteps.maximumCaptures,
            maximumReplacementOutputUTF16Length: lowSteps.maximumReplacementOutputUTF16Length
        )
        #expect(throws: RegexFailure.engineStepLimit) {
            try RegexEngine.run(
                RegexRequest(
                    pattern: "(a+)+b", text: String(repeating: "a", count: 500), replacement: "",
                    flags: RegexFlags()),
                policy: lowSteps
            )
        }

        let lowHeap = RegexExecutionPolicy(
            maximumPatternUTF16Length: 1_000,
            maximumTextUTF16Length: 10_000,
            maximumReplacementUTF16Length: 1_000,
            engineStepLimit: 2_000_000,
            heapBacktrackingLimitBytes: 1_024,
            elapsedDeadlineNanoseconds: 1_500_000_000,
            maximumMatches: 10_000,
            maximumCaptures: 50_000,
            maximumReplacementOutputUTF16Length: 20_000
        )
        #expect(throws: RegexFailure.backtrackingLimit) {
            try RegexEngine.run(
                RegexRequest(
                    pattern: "^(a|aa)*b$", text: String(repeating: "a", count: 2_000),
                    replacement: "", flags: RegexFlags()),
                policy: lowHeap
            )
        }
    }

    @Test func revisionGateAndSnapshotRejectStalePublicationAndRoundTrip() throws {
        var gate = RegexRevisionGate()
        let old = gate.begin()
        let current = gate.begin()
        #expect(!gate.accepts(old))
        #expect(gate.accepts(current))

        let request = RegexRequest(pattern: "(a)", text: "a", replacement: "$1", flags: RegexFlags())
        let snapshot = RegexSnapshot(request: request, result: try RegexEngine.run(request))
        let restored = try JSONDecoder().decode(RegexSnapshot.self, from: JSONEncoder().encode(snapshot))
        #expect(restored == snapshot)
    }
}

private extension RegexFlags {
    init(caseInsensitive: Bool) {
        self.init()
        self.caseInsensitive = caseInsensitive
    }
}
