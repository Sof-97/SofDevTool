import Foundation
import Testing

@testable import SofDevTool

@Suite("Case Conversion Utility")
struct CaseConversionTests {
    @Test(arguments: [
        ("HTTPServer", ["HTTP", "Server"]),
        ("version2Value", ["version", "2", "Value"]),
        ("already_snake-kebab title", ["already", "snake", "kebab", "title"]),
        ("XMLHttpRequest", ["XML", "Http", "Request"]),
        ("café e\u{301}lan", ["café", "e\u{301}lan"]),
        ("東京Value", ["東京", "Value"]),
        ("go👩🏽‍💻Now", ["go", "👩🏽‍💻", "Now"]),
    ])
    func segmentationMatrix(input: String, expected: [String]) {
        #expect(CaseConversionEngine.segment(input) == expected)
    }

    @Test func everyStyleUsesTheSameDetectedWords() throws {
        let input = "HTTPServer version2Value"
        let expectedWords = ["HTTP", "Server", "version", "2", "Value"]
        let expected: [CaseConversionStyle: String] = [
            .camel: "httpServerVersion2Value",
            .pascal: "HttpServerVersion2Value",
            .snake: "http_server_version_2_value",
            .screamingSnake: "HTTP_SERVER_VERSION_2_VALUE",
            .kebab: "http-server-version-2-value",
            .title: "Http Server Version 2 Value",
            .sentence: "Http server version 2 value",
            .lowercase: "http server version 2 value",
            .uppercase: "HTTP SERVER VERSION 2 VALUE",
        ]

        for style in CaseConversionStyle.allCases {
            let result = try CaseConversionEngine.convert(input, to: style)
            #expect(result.words == expectedWords)
            #expect(result.output == expected[style])
        }
    }

    @Test func localeIndependentCasingAndGraphemesArePreserved() throws {
        let result = try CaseConversionEngine.convert("İSTANBUL café 👩🏽‍💻", to: .snake)
        #expect(result.words.last == "👩🏽‍💻")
        #expect(result.output.contains("café"))
        #expect(result.output.hasSuffix("_👩🏽‍💻"))
    }

    @Test func inputCapAndSnapshotAreExplicit() throws {
        let policy = CaseConversionPolicy(maximumInputUTF16Length: 3)
        #expect(throws: UtilityError.self) {
            try CaseConversionEngine.convert("four", to: .camel, policy: policy)
        }

        let result = try CaseConversionEngine.convert("helloWorld", to: .snake)
        let snapshot = CaseConversionSnapshot(input: "helloWorld", style: .snake, result: result)
        let restored = try JSONDecoder().decode(
            CaseConversionSnapshot.self,
            from: JSONEncoder().encode(snapshot)
        )
        #expect(restored == snapshot)
    }

    @Test func revisionGateRejectsSupersededConversion() {
        var gate = CaseConversionRevisionGate()
        let old = gate.begin()
        let current = gate.begin()
        #expect(!gate.accepts(old))
        #expect(gate.accepts(current))
    }
}
