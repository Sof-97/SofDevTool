import Foundation
import Testing

@testable import SofDevTool

private final class SampleEntropyFixture: SampleDataEntropySource, @unchecked Sendable {
    private var next: UInt8 = 0
    func bytes(count: Int) throws -> [UInt8] {
        defer { next &+= 1 }
        return (0..<count).map { next &+ UInt8($0 % 251) }
    }
}

@Suite("Sample Data Utility")
struct SampleDataTests {
    @Test func deterministicGenerationCoversEveryFieldTypeAndPreservesOrder() throws {
        let schema = SampleDataSchema(
            rowCount: 2, format: .json,
            fields: [
                .init(name: "name", type: .fictionalName), .init(name: "email", type: .fictionalEmail),
                .init(
                    name: "score", type: .number, numberMinimum: 10, numberMaximum: 20, numberIsInteger: true),
                .init(
                    name: "ratio", type: .number, numberMinimum: -1, numberMaximum: 1,
                    numberIsInteger: false),
                .init(name: "active", type: .boolean),
                .init(
                    name: "created", type: .date, dateMinimum: Date(timeIntervalSince1970: 0),
                    dateMaximum: Date(timeIntervalSince1970: 86_400)),
                .init(name: "id", type: .uuid),
                .init(name: "mood", type: .enumeration, enumChoices: ["calm", "👩🏽\u{200D}💻"]),
            ])
        let result = try SampleDataEngine.generate(schema: schema, entropy: SampleEntropyFixture())
        let object = try #require(
            JSONSerialization.jsonObject(with: Data(result.output.utf8)) as? [[String: Any]])
        #expect(object.count == 2)
        #expect(result.rows[0].map(\.fieldName) == schema.fields.map(\.name))
        #expect(result.output.contains("Fictional"))
        #expect(result.output.contains("090a0b0c-0d0e-4f10-9112-131415161718"))
    }

    @Test func CSVAlwaysQuotesAndEscapesRFC4180SensitiveValues() throws {
        let schema = SampleDataSchema(
            rowCount: 1, format: .csv,
            fields: [
                .init(name: "value", type: .enumeration, enumChoices: ["comma, quote \" and\nnewline"]),
                .init(name: "emoji", type: .enumeration, enumChoices: ["👩🏽\u{200D}💻"]),
            ])
        let output = try SampleDataEngine.generate(schema: schema, entropy: SampleEntropyFixture()).output
        #expect(output == "\"value\",\"emoji\"\n\"comma, quote \"\" and\nnewline\",\"👩🏽\u{200D}💻\"\n")
    }

    @Test func invalidSchemasFailBeforeGeneration() {
        let invalid = [
            SampleDataSchema(rowCount: 0, fields: [.init(name: "ok", type: .boolean)]),
            SampleDataSchema(rowCount: 1, fields: [.init(name: "bad name", type: .boolean)]),
            SampleDataSchema(
                rowCount: 1, fields: [.init(name: "same", type: .boolean), .init(name: "same", type: .uuid)]),
            SampleDataSchema(
                rowCount: 1, fields: [.init(name: "n", type: .number, numberMinimum: 2, numberMaximum: 1)]),
            SampleDataSchema(
                rowCount: 1, fields: [.init(name: "choice", type: .enumeration, enumChoices: [])]),
            SampleDataSchema(rowCount: 1_001, fields: [.init(name: "ok", type: .boolean)]),
            SampleDataSchema(
                rowCount: 1,
                fields: (0...50).map { .init(name: "field\($0)", type: .boolean) }),
        ]
        for schema in invalid {
            #expect(throws: UtilityError.self) {
                try SampleDataEngine.generate(schema: schema, entropy: SampleEntropyFixture())
            }
        }
    }

    @Test func documentedSchemaLimitsAcceptTheirUpperBound() throws {
        let schema = SampleDataSchema(
            rowCount: 1_000,
            fields: (0..<50).map { .init(name: "field\($0)", type: .boolean) })
        try SampleDataEngine.validate(schema)
    }

    @Test func snapshotAndRevisionGateDoNotRegenerate() throws {
        let schema = SampleDataSchema(rowCount: 1, fields: [.init(name: "id", type: .uuid)])
        let result = try SampleDataEngine.generate(schema: schema, entropy: SampleEntropyFixture())
        let snapshot = SampleDataSnapshot(schema: schema, rows: result.rows, output: result.output)
        #expect(
            try JSONDecoder().decode(SampleDataSnapshot.self, from: JSONEncoder().encode(snapshot))
                == snapshot)
        var gate = SampleDataRevisionGate()
        let old = gate.begin()
        let current = gate.begin()
        #expect(!gate.accepts(old))
        #expect(gate.accepts(current))
    }
}
