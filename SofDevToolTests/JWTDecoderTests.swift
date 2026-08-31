import Foundation
import Testing

@testable import SofDevTool

@Suite("JWT Decoder Utility")
struct JWTDecoderTests {
    @Test func decodesUnicodeJSONWithoutInterpretingClaims() throws {
        let header = encoded(#"{"alg":"none","typ":"JWT"}"#)
        let payload = encoded(#"{"admin":true,"exp":0,"name":"café 👩🏽‍💻"}"#)

        let result = try JWTDecoderEngine.decode("\(header).\(payload).opaque-signature")

        #expect(result.headerJSON.contains(#""alg" : "none""#))
        #expect(result.payloadJSON.contains("café 👩🏽‍💻"))
        #expect(result.payloadJSON.contains(#""exp" : 0"#))
        #expect(result.signature == "opaque-signature")
    }

    @Test func failuresIdentifyTheExactStage() {
        #expect(throws: JWTDecoderFailure.segmentCount(2)) {
            try JWTDecoderEngine.decode("only.two")
        }
        #expect(throws: JWTDecoderFailure.invalidBase64URL(segment: "header")) {
            try JWTDecoderEngine.decode("*.e30.signature")
        }
        #expect(throws: JWTDecoderFailure.invalidUTF8(segment: "header")) {
            try JWTDecoderEngine.decode("_w.e30.signature")
        }
        #expect(throws: JWTDecoderFailure.invalidJSON(segment: "header")) {
            try JWTDecoderEngine.decode("\(encoded("not json")).e30.signature")
        }
        #expect(throws: JWTDecoderFailure.invalidJSON(segment: "payload")) {
            try JWTDecoderEngine.decode("e30.\(encoded("not json")).signature")
        }
    }

    @Test func snapshotRoundTripPreservesOpaqueValues() throws {
        let decoded = try JWTDecoderEngine.decode("e30.e30.")
        let snapshot = JWTDecoderSnapshot(input: "e30.e30.", decoded: decoded)
        let restored = try JSONDecoder().decode(
            JWTDecoderSnapshot.self,
            from: JSONEncoder().encode(snapshot)
        )
        #expect(restored == snapshot)
    }

    @Test func revisionGateRejectsSupersededDecode() {
        var gate = JWTDecoderRevisionGate()
        let old = gate.begin()
        let current = gate.begin()
        #expect(!gate.accepts(old))
        #expect(gate.accepts(current))
    }

    private func encoded(_ value: String) -> String {
        Data(value.utf8).base64EncodedString()
            .replacingOccurrences(of: "+", with: "-")
            .replacingOccurrences(of: "/", with: "_")
            .replacingOccurrences(of: "=", with: "")
    }
}
