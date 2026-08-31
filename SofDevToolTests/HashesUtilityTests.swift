import Foundation
import Testing

@testable import SofDevTool

@Suite("Hashes Utility")
struct HashesUtilityTests {
    @Test(
        arguments: [
            (
                HashAlgorithm.sha256,
                "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
                "ungWv48Bz+pBQUDeXa4iI7ADYaOWF3qctBD/YfIAFa0="
            ),
            (
                HashAlgorithm.sha384,
                "cb00753f45a35e8bb5a03d699ac65007272c32ab0eded1631a8b605a43ff5bed8086072ba1e7cc2358baeca134c825a7",
                "ywB1P0WjXou1oD1pmsZQBycsMqsO3tFjGotgWkP/W+2AhgcroefMI1i67KE0yCWn"
            ),
            (
                HashAlgorithm.sha512,
                "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f",
                "3a81oZNherrMQXNJriBBMRLm+k6JqX6iCp7u5ktV05ohkpkqJ0/BqDa6PCOj/uu9RU1EI2Q86A4qmslPpUyknw=="
            ),
            (
                HashAlgorithm.sha1,
                "a9993e364706816aba3e25717850c26c9cd0d89d",
                "qZk+NkcGgWq6PiVxeFDCbJzQ2J0="
            ),
            (
                HashAlgorithm.md5,
                "900150983cd24fb0d6963f7d28e17f72",
                "kAFQmDzST7DWlj99KOF/cg=="
            ),
        ]
    )
    func publishedABCVectors(algorithm: HashAlgorithm, hexadecimal: String, base64: String) {
        #expect(HashEngine.hash("abc", algorithm: algorithm, format: .lowercaseHex) == hexadecimal)
        #expect(
            HashEngine.hash("abc", algorithm: algorithm, format: .uppercaseHex)
                == hexadecimal.uppercased()
        )
        #expect(HashEngine.hash("abc", algorithm: algorithm, format: .base64) == base64)
    }

    @Test func exactUnicodeUTF8BytesAndEveryOutputFormatAreCanonical() {
        let input = "café 👩🏽‍💻"

        #expect(
            HashEngine.hash(input, algorithm: .sha256, format: .lowercaseHex)
                == "2df44393dea8fecf872bddf5ae1bd776d959c1a4f93655f5555de284bcf83773"
        )
        #expect(
            HashEngine.hash(input, algorithm: .sha256, format: .uppercaseHex)
                == "2DF44393DEA8FECF872BDDF5AE1BD776D959C1A4F93655F5555DE284BCF83773"
        )
        #expect(
            HashEngine.hash(input, algorithm: .sha256, format: .base64)
                == "LfRDk96o/s+HK931rhvXdtlZwaT5NlX1VV3ihLz4N3M="
        )
    }

    @Test(
        arguments: [
            (HashAlgorithm.sha256, "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"),
            (
                HashAlgorithm.sha384,
                "38b060a751ac96384cd9327eb1b1e36a21fdb71114be07434c0cc7bf63f6e1da274edebfe76f65fbd51ad2f14898b95b"
            ),
            (
                HashAlgorithm.sha512,
                "cf83e1357eefb8bdf1542850d66d8007d620e4050b5715dc83f4a921d36ce9ce47d0d13c5d85f2b0ff8318d2877eec2f63b931bd47417a81a538327af927da3e"
            ),
            (HashAlgorithm.sha1, "da39a3ee5e6b4b0d3255bfef95601890afd80709"),
            (HashAlgorithm.md5, "d41d8cd98f00b204e9800998ecf8427e"),
        ]
    )
    func emptyInputIsAValidExplicitHashRequest(algorithm: HashAlgorithm, expected: String) {
        #expect(HashEngine.hash("", algorithm: algorithm, format: .lowercaseHex) == expected)
    }

    @Test(
        arguments: [
            (HashAlgorithm.sha256, "2df44393dea8fecf872bddf5ae1bd776d959c1a4f93655f5555de284bcf83773"),
            (
                HashAlgorithm.sha384,
                "9eec581cf74a0324ed7720dd1c3b7fff86ec1c93875fac32e2f7a899187738b1fc458993a1566d351b93131dd72a3d74"
            ),
            (
                HashAlgorithm.sha512,
                "9b503fdad948a40e64b30b4ee8c81fa15bfdbd4e1b6ce238b7a3753045123d6720266aa19529b441834cbe69dd6cb26d4816ef90f0027b2130f86af4999c8f06"
            ),
            (HashAlgorithm.sha1, "79a78ecdc143354466be7d937183fc45199a4439"),
            (HashAlgorithm.md5, "096743d25698360d5876d4ed9e294eb4"),
        ]
    )
    func everyAlgorithmHashesExactUnicodeUTF8Bytes(algorithm: HashAlgorithm, expected: String) {
        #expect(
            HashEngine.hash("café 👩🏽‍💻", algorithm: algorithm, format: .lowercaseHex) == expected
        )
    }

    @Test func legacyAlgorithmsCarryVisibleNonSecurityLabels() {
        #expect(HashAlgorithm.sha1.isLegacy)
        #expect(HashAlgorithm.md5.isLegacy)
        #expect(HashAlgorithm.sha1.notice.contains("Legacy"))
        #expect(HashAlgorithm.md5.notice.contains("not for security"))
        #expect(!HashAlgorithm.sha256.isLegacy)
    }

    @Test func repeatedExplicitRequestsEachOfferOneSnapshot() {
        var session = HashesSession()
        session.input = "abc"

        session.hash()
        let first = session.consumeSnapshot()
        session.hash()
        let second = session.consumeSnapshot()

        #expect(first?.output == second?.output)
        #expect(first != nil)
        #expect(second != nil)
        #expect(session.consumeSnapshot() == nil)
    }

    @Test func restorePreservesSettledDigestWithoutRerunningOrRecording() throws {
        let snapshot = HashesSnapshot(
            algorithm: .sha512,
            format: .base64,
            input: "restored",
            output: "captured digest"
        )
        let decoded = try JSONDecoder().decode(HashesSnapshot.self, from: JSONEncoder().encode(snapshot))
        var session = HashesSession()

        session.restore(decoded)

        #expect(decoded == snapshot)
        #expect(session.output == "captured digest")
        #expect(session.consumeSnapshot() == nil)
    }
}
