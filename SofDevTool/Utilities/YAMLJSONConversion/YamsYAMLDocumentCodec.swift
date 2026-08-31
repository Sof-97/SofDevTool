import Foundation
import Yams

struct YamsYAMLDocumentCodec: YAMLDocumentCodec {
    func decodeSingleDocument(
        _ source: String,
        policy: YAMLJSONResourcePolicy
    ) throws -> YAMLJSONDocument {
        do {
            guard let node = try Yams.compose(yaml: source, .basic) else {
                return YAMLJSONDocument(value: .null)
            }
            let value = try convert(node, depth: 1, policy: policy)
            return YAMLJSONDocument(value: value)
        } catch let error as UtilityError {
            throw error
        } catch let error as YamlError {
            throw translate(error)
        } catch {
            throw UtilityError("Invalid YAML: \(error.localizedDescription)")
        }
    }

    func encodeDocument(_ document: YAMLJSONDocument) throws -> String {
        do {
            return try Yams.serialize(
                node: node(from: document.value),
                indent: 2,
                allowUnicode: true,
                sortKeys: false
            )
        } catch {
            throw UtilityError("Could not emit YAML: \(error.localizedDescription)")
        }
    }

    private func convert(
        _ node: Node,
        depth: Int,
        policy: YAMLJSONResourcePolicy
    ) throws -> YAMLJSONValue {
        guard depth <= policy.maximumNestingDepth else {
            throw locatedError(
                "Document nesting exceeds the supported depth of \(policy.maximumNestingDepth); nothing was converted.",
                mark: node.mark
            )
        }
        switch node {
        case .scalar(let scalar):
            return try convert(scalar)
        case .sequence(let sequence):
            try requireSupportedCollectionTag(sequence.tag, expected: Tag.Name.seq, mark: sequence.mark)
            return .array(
                try sequence.map { try convert($0, depth: depth + 1, policy: policy) })
        case .mapping(let mapping):
            try requireSupportedCollectionTag(mapping.tag, expected: Tag.Name.map, mark: mapping.mark)
            var seen: Set<String> = []
            var members: [YAMLJSONObjectMember] = []
            for pair in mapping {
                let keyValue: YAMLJSONValue
                if case .scalar(let scalar) = pair.key, scalar.style == .plain,
                    scalar.tag.rawValue == Tag.Name.str.rawValue
                {
                    // Yams hashes mapping keys while checking duplicates, which resolves an
                    // implicit basic-resolver key to `str`. Reapply the app-owned Core rules.
                    keyValue = try resolveCorePlainScalar(scalar.string, mark: scalar.mark)
                } else {
                    keyValue = try convert(pair.key, depth: depth + 1, policy: policy)
                }
                guard case .string(let key) = keyValue else {
                    throw locatedError(
                        "YAML mapping keys must resolve to unique strings for JSON conversion.",
                        mark: pair.key.mark
                    )
                }
                guard seen.insert(key).inserted else {
                    throw locatedError("Duplicate mapping key ‘\(key)’.", mark: pair.key.mark)
                }
                members.append(
                    YAMLJSONObjectMember(
                        key: key,
                        value: try convert(pair.value, depth: depth + 1, policy: policy)
                    ))
            }
            return .object(members.sorted { $0.key < $1.key })
        case .alias:
            throw locatedError("Unresolved YAML aliases are unsupported.", mark: node.mark)
        }
    }

    private func convert(_ scalar: Node.Scalar) throws -> YAMLJSONValue {
        let tag = scalar.tag.rawValue
        let isQuoted = scalar.style == .singleQuoted || scalar.style == .doubleQuoted
        if isQuoted, tag.isEmpty { return .string(scalar.string) }

        switch tag {
        case "", "!", Tag.Name.str.rawValue:
            if isQuoted || tag == Tag.Name.str.rawValue { return .string(scalar.string) }
            return try resolveCorePlainScalar(scalar.string, mark: scalar.mark)
        case Tag.Name.null.rawValue:
            guard isNull(scalar.string) else {
                throw locatedError("Invalid explicitly tagged YAML null.", mark: scalar.mark)
            }
            return .null
        case Tag.Name.bool.rawValue:
            guard let boolean = coreBoolean(scalar.string) else {
                throw locatedError("Invalid explicitly tagged YAML boolean.", mark: scalar.mark)
            }
            return .boolean(boolean)
        case Tag.Name.int.rawValue:
            return .number(try integer(scalar.string, mark: scalar.mark))
        case Tag.Name.float.rawValue:
            return .number(try decimal(scalar.string, mark: scalar.mark))
        default:
            throw locatedError("Unsupported YAML tag ‘\(tag)’.", mark: scalar.mark)
        }
    }

    private func resolveCorePlainScalar(_ source: String, mark: Mark?) throws -> YAMLJSONValue {
        if isNull(source) { return .null }
        if let boolean = coreBoolean(source) { return .boolean(boolean) }
        if matches(source, #"^[-+]?(?:0|[1-9][0-9_]*|0o[0-7_]+|0x[0-9a-fA-F_]+)$"#) {
            return .number(try integer(source, mark: mark))
        }
        if matches(
            source,
            #"^[-+]?(?:(?:[0-9][0-9_]*)?\.[0-9_]+|[0-9][0-9_]*(?:\.[0-9_]*)?[eE][-+]?[0-9]+)$"#
        ) {
            return .number(try decimal(source, mark: mark))
        }
        if matches(source, #"^[-+]?\.(?:inf|Inf|INF|nan|NaN|NAN)$"#) {
            throw locatedError("Non-finite YAML numbers cannot be represented in JSON.", mark: mark)
        }
        return .string(source)
    }

    private func integer(_ source: String, mark: Mark?) throws -> String {
        let cleaned = source.replacingOccurrences(of: "_", with: "")
        let negative = cleaned.hasPrefix("-")
        let unsigned = cleaned.drop(while: { $0 == "+" || $0 == "-" })
        let radix: Int
        let digits: Substring
        if unsigned.hasPrefix("0o") {
            radix = 8
            digits = unsigned.dropFirst(2)
        } else if unsigned.hasPrefix("0x") {
            radix = 16
            digits = unsigned.dropFirst(2)
        } else {
            radix = 10
            digits = unsigned
        }
        guard let magnitude = UInt64(digits, radix: radix) else {
            throw locatedError(
                "Integer is outside the exact JSON conversion range.",
                mark: mark
            )
        }
        if negative {
            if magnitude == UInt64(Int64.max) + 1 { return String(Int64.min) }
            guard magnitude <= UInt64(Int64.max) else {
                throw locatedError("Integer is outside the exact JSON conversion range.", mark: mark)
            }
            return String(-Int64(magnitude))
        }
        return String(magnitude)
    }

    private func decimal(_ source: String, mark: Mark?) throws -> String {
        let cleaned = source.replacingOccurrences(of: "_", with: "")
        guard !cleaned.lowercased().contains("inf"), !cleaned.lowercased().contains("nan"),
            let value = Decimal(string: cleaned, locale: Locale(identifier: "en_US_POSIX"))
        else {
            throw locatedError("Number cannot be represented faithfully in JSON.", mark: mark)
        }
        let canonical = NSDecimalNumber(decimal: value).stringValue
        guard canonical != "NaN" else {
            throw locatedError("Number cannot be represented faithfully in JSON.", mark: mark)
        }
        return canonical
    }

    private func node(from value: YAMLJSONValue) -> Node {
        switch value {
        case .null:
            return Node("null", Tag(.null))
        case .boolean(let value):
            return Node(value ? "true" : "false", Tag(.bool))
        case .number(let value):
            let tag: Tag =
                value.contains(".") || value.lowercased().contains("e")
                ? Tag(.float) : Tag(.int)
            return Node(value, tag)
        case .string(let value):
            return Node(value, Tag(.str))
        case .array(let values):
            return Node(values.map(node))
        case .object(let members):
            return Node(
                members.sorted { $0.key < $1.key }.map {
                    (Node($0.key, Tag(.str)), node(from: $0.value))
                })
        }
    }

    private func requireSupportedCollectionTag(_ tag: Tag, expected: Tag.Name, mark: Mark?) throws {
        guard tag.rawValue.isEmpty || tag.rawValue == "!" || tag.rawValue == expected.rawValue else {
            throw locatedError("Unsupported YAML tag ‘\(tag.rawValue)’.", mark: mark)
        }
    }

    private func isNull(_ source: String) -> Bool {
        source.isEmpty || ["~", "null"].contains(source.lowercased())
    }

    private func coreBoolean(_ source: String) -> Bool? {
        switch source.lowercased() {
        case "true": true
        case "false": false
        default: nil
        }
    }

    private func matches(_ source: String, _ pattern: String) -> Bool {
        source.range(of: pattern, options: .regularExpression) != nil
    }

    private func locatedError(_ message: String, mark: Mark?) -> UtilityError {
        guard let mark else { return UtilityError(message) }
        return UtilityError("\(message) Line \(mark.line), column \(mark.column).")
    }

    private func translate(_ error: YamlError) -> UtilityError {
        switch error {
        case .scanner(_, let problem, let mark, _),
            .parser(_, let problem, let mark, _),
            .composer(_, let problem, let mark, _):
            return locatedError("Invalid YAML: \(problem).", mark: mark)
        case .duplicatedKeysInMapping(let duplicates, let context):
            return locatedError(
                "Duplicate YAML mapping key: \(duplicates.sorted().joined(separator: ", ")).",
                mark: context.mark
            )
        default:
            return UtilityError("Invalid YAML: \(error.localizedDescription)")
        }
    }
}
