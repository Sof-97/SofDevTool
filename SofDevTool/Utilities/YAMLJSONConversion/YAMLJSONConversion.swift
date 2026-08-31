import Foundation

enum YAMLJSONDirection: String, CaseIterable, Codable, Identifiable {
    case yamlToJSON = "YAML → JSON"
    case jsonToYAML = "JSON → YAML"

    var id: Self { self }
}

enum YAMLJSONWarning: String, Codable, Equatable {
    case lossyRoundTrip
}

struct YAMLJSONConversionResult: Equatable {
    let output: String
    let warnings: [YAMLJSONWarning]
}

struct YAMLJSONSnapshot: Codable, Equatable {
    let direction: YAMLJSONDirection
    let input: String
    let output: String
}

struct YAMLJSONDocument: Equatable {
    let value: YAMLJSONValue
}

struct YAMLJSONObjectMember: Equatable {
    let key: String
    let value: YAMLJSONValue
}

indirect enum YAMLJSONValue: Equatable {
    case null
    case boolean(Bool)
    case number(String)
    case string(String)
    case array([YAMLJSONValue])
    case object([YAMLJSONObjectMember])
}

protocol YAMLDocumentCodec {
    func decodeSingleDocument(_ source: String, policy: YAMLJSONResourcePolicy) throws
        -> YAMLJSONDocument
    func encodeDocument(_ document: YAMLJSONDocument) throws -> String
}

struct YAMLJSONResourcePolicy: Equatable {
    static let production = YAMLJSONResourcePolicy(
        maximumInputBytes: 1_048_576,
        maximumNestingDepth: 128
    )

    let maximumInputBytes: Int
    let maximumNestingDepth: Int
}

struct YAMLJSONRevisionGate {
    private(set) var current = 0

    mutating func begin() -> Int {
        current += 1
        return current
    }

    func accepts(_ revision: Int) -> Bool { revision == current }
}

enum YAMLJSONEngine {
    static func convert(
        _ input: String,
        direction: YAMLJSONDirection,
        policy: YAMLJSONResourcePolicy = .production,
        codec: any YAMLDocumentCodec = YamsYAMLDocumentCodec()
    ) throws -> YAMLJSONConversionResult {
        guard input.utf8.count <= policy.maximumInputBytes else {
            throw UtilityError(
                "Input exceeds the 1 MiB conversion limit; nothing was converted."
            )
        }

        let output: String
        switch direction {
        case .yamlToJSON:
            let document = try codec.decodeSingleDocument(input, policy: policy)
            output = YAMLJSONWriter.write(document.value)
        case .jsonToYAML:
            let value = try YAMLJSONParser.parse(input, maximumDepth: policy.maximumNestingDepth)
            output = try codec.encodeDocument(YAMLJSONDocument(value: value))
        }
        return YAMLJSONConversionResult(output: output, warnings: [.lossyRoundTrip])
    }
}

private enum YAMLJSONParser {
    static func parse(_ source: String, maximumDepth: Int) throws -> YAMLJSONValue {
        do {
            let value = try JSONDecoder().decode(DecodableValue.self, from: Data(source.utf8)).value
            guard depth(of: value) <= maximumDepth else {
                throw UtilityError(
                    "Document nesting exceeds the supported depth of \(maximumDepth); nothing was converted."
                )
            }
            return value
        } catch let error as UtilityError {
            throw error
        } catch {
            throw UtilityError("Invalid JSON: \(error.localizedDescription)")
        }
    }

    private static func depth(of value: YAMLJSONValue) -> Int {
        switch value {
        case .null, .boolean, .number, .string:
            return 1
        case .array(let values):
            return 1 + (values.map(depth).max() ?? 0)
        case .object(let members):
            return 1 + (members.map { depth(of: $0.value) }.max() ?? 0)
        }
    }

    private struct DecodableValue: Decodable {
        let value: YAMLJSONValue

        init(from decoder: any Decoder) throws {
            if let container = try? decoder.container(keyedBy: DynamicKey.self) {
                value = .object(
                    try container.allKeys.sorted { $0.stringValue < $1.stringValue }.map {
                        YAMLJSONObjectMember(
                            key: $0.stringValue,
                            value: try container.decode(DecodableValue.self, forKey: $0).value
                        )
                    })
                return
            }
            if var container = try? decoder.unkeyedContainer() {
                var values: [YAMLJSONValue] = []
                while !container.isAtEnd {
                    values.append(try container.decode(DecodableValue.self).value)
                }
                value = .array(values)
                return
            }
            let container = try decoder.singleValueContainer()
            if container.decodeNil() {
                value = .null
            } else if let boolean = try? container.decode(Bool.self) {
                value = .boolean(boolean)
            } else if let integer = try? container.decode(Int64.self) {
                value = .number(String(integer))
            } else if let decimal = try? container.decode(Decimal.self) {
                value = .number(NSDecimalNumber(decimal: decimal).stringValue)
            } else {
                value = .string(try container.decode(String.self))
            }
        }
    }

    private struct DynamicKey: CodingKey {
        let stringValue: String
        let intValue: Int?

        init?(stringValue: String) {
            self.stringValue = stringValue
            intValue = nil
        }

        init?(intValue: Int) {
            stringValue = String(intValue)
            self.intValue = intValue
        }
    }
}

private enum YAMLJSONWriter {
    static func write(_ value: YAMLJSONValue) -> String {
        render(value, indentation: 0)
    }

    private static func render(_ value: YAMLJSONValue, indentation: Int) -> String {
        switch value {
        case .null:
            return "null"
        case .boolean(let value):
            return value ? "true" : "false"
        case .number(let value):
            return value
        case .string(let value):
            return quoted(value)
        case .array(let values):
            guard !values.isEmpty else { return "[]" }
            let childIndentation = indentation + 2
            let lines = values.map {
                String(repeating: " ", count: childIndentation)
                    + render($0, indentation: childIndentation)
            }
            return "[\n" + lines.joined(separator: ",\n") + "\n"
                + String(repeating: " ", count: indentation) + "]"
        case .object(let members):
            guard !members.isEmpty else { return "{}" }
            let childIndentation = indentation + 2
            let lines = members.sorted { $0.key < $1.key }.map {
                String(repeating: " ", count: childIndentation) + quoted($0.key) + " : "
                    + render($0.value, indentation: childIndentation)
            }
            return "{\n" + lines.joined(separator: ",\n") + "\n"
                + String(repeating: " ", count: indentation) + "}"
        }
    }

    private static func quoted(_ value: String) -> String {
        let encoded = try? JSONEncoder().encode(value)
        return encoded.map { String(decoding: $0, as: UTF8.self) } ?? "\"\""
    }
}
