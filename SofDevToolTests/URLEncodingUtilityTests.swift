import Foundation
import Testing

@testable import SofDevTool

@Suite("URL Encoding Utility")
struct URLEncodingUtilityTests {
    @Test func pathSegmentPreservesPCharAndEncodesSegmentDelimiters() throws {
        let input = "alpha:@!$&'()*+,;= /?%+"

        let output = try URLEncodingEngine.transform(
            input,
            options: URLEncodingOptions(direction: .encode, mode: .pathSegment)
        )

        #expect(output == "alpha:@!$&'()*+,;=%20%2F%3F%25+")
    }

    @Test func queryValuePreservesOnlyUnreservedCharacters() throws {
        let output = try URLEncodingEngine.transform(
            "AZaz09-._~ & = + # ? / : café",
            options: URLEncodingOptions(direction: .encode, mode: .queryValue)
        )

        #expect(
            output
                == "AZaz09-._~%20%26%20%3D%20%2B%20%23%20%3F%20%2F%20%3A%20caf%C3%A9"
        )
    }

    @Test func percentEncodingIsDistinctFromURLSafeBase64() throws {
        let output = try URLEncodingEngine.transform(
            "?", options: URLEncodingOptions(direction: .encode, mode: .queryValue))

        #expect(output == "%3F")
        #expect(output != "Pw==")
    }

    @Test func strictDecodeUsesUTF8AndNeverTreatsPlusAsSpace() throws {
        let options = URLEncodingOptions(direction: .decode, mode: .queryValue)

        #expect(try URLEncodingEngine.transform("e%CC%81+%E6%BC%A2%E5%AD%97", options: options) == "é+漢字")
        #expect(
            try URLEncodingEngine.transform(
                "%F0%9F%91%A9%F0%9F%8F%BD%E2%80%8D%F0%9F%92%BB", options: options)
                == "👩🏽‍💻"
        )
    }

    @Test func malformedTripletsAndInvalidUTF8AreRejected() {
        let options = URLEncodingOptions(direction: .decode, mode: .pathSegment)

        #expect(throws: UtilityError.self) { try URLEncodingEngine.transform("%", options: options) }
        #expect(throws: UtilityError.self) { try URLEncodingEngine.transform("%2G", options: options) }
        #expect(throws: UtilityError.self) { try URLEncodingEngine.transform("%FF", options: options) }
    }

    @Test func snapshotAndSessionRestoreSettledOutputWithoutRecording() throws {
        let snapshot = URLEncodingSnapshot(
            options: URLEncodingOptions(direction: .decode, mode: .queryValue),
            input: "hello%20world",
            output: "hello world"
        )
        let decoded = try JSONDecoder().decode(
            URLEncodingSnapshot.self,
            from: JSONEncoder().encode(snapshot)
        )
        var session = URLEncodingSession()

        session.restore(decoded)

        #expect(decoded == snapshot)
        #expect(session.options == snapshot.options)
        #expect(session.input == snapshot.input)
        #expect(session.output == snapshot.output)
        #expect(session.consumeSnapshot() == nil)
    }

    @Test func onlyLatestRevisionCanPublish() {
        var session = URLEncodingSession()
        let first = session.beginRevision()
        let second = session.beginRevision()

        #expect(!session.accepts(first))
        #expect(session.accepts(second))
    }
}
