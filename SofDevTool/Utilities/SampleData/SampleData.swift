import Foundation
import Security
import SwiftUI

enum SampleDataFormat: String, CaseIterable, Codable, Identifiable {
    case json = "JSON"
    case csv = "CSV"
    var id: Self { self }
}
enum SampleFieldType: String, CaseIterable, Codable, Identifiable {
    case fictionalName = "Fictional name"
    case fictionalEmail = "Fictional email"
    case number = "Number"
    case boolean = "Boolean"
    case date = "Date"
    case uuid = "UUID"
    case enumeration = "Enum"
    var id: Self { self }
}

struct SampleField: Codable, Equatable, Identifiable {
    var id = UUID()
    var name: String
    var type: SampleFieldType
    var numberMinimum: Double = 0
    var numberMaximum: Double = 100
    var numberIsInteger = true
    var dateMinimum = Date(timeIntervalSince1970: 0)
    var dateMaximum = Date(timeIntervalSince1970: 1_893_456_000)
    var enumChoices: [String] = ["alpha", "beta"]
}

struct SampleDataSchema: Codable, Equatable {
    var rowCount = 10
    var format = SampleDataFormat.json
    var fields: [SampleField] = [
        .init(name: "name", type: .fictionalName),
        .init(name: "email", type: .fictionalEmail),
        .init(name: "id", type: .uuid),
    ]
}

enum SampleValue: Codable, Equatable {
    case string(String), number(Double, integer: Bool), boolean(Bool)

    var displayValue: String {
        switch self {
        case .string(let value): value
        case .number(let value, let integer): integer ? String(Int(value)) : SampleDataEngine.decimal(value)
        case .boolean(let value): value ? "true" : "false"
        }
    }
}

struct SampleDataCell: Codable, Equatable {
    let fieldName: String
    let value: SampleValue
}
struct SampleDataResult: Equatable {
    let rows: [[SampleDataCell]]
    let output: String
}
struct SampleDataSnapshot: Codable, Equatable {
    let schema: SampleDataSchema
    let rows: [[SampleDataCell]]
    let output: String
}

protocol SampleDataEntropySource: Sendable { func bytes(count: Int) throws -> [UInt8] }
struct SecureSampleDataEntropy: SampleDataEntropySource {
    func bytes(count: Int) throws -> [UInt8] {
        var result = [UInt8](repeating: 0, count: count)
        guard SecRandomCopyBytes(kSecRandomDefault, count, &result) == errSecSuccess else {
            throw UtilityError("The system random-number generator failed.")
        }
        return result
    }
}

struct SampleDataRevisionGate {
    private(set) var current = 0
    mutating func begin() -> Int {
        current += 1
        return current
    }
    func accepts(_ revision: Int) -> Bool { revision == current }
}

enum SampleDataEngine {
    private static let firstNames = ["Fictional Ada", "Fictional Lin", "Fictional Sam", "Fictional Noor"]
    private static let lastNames = ["Example", "Sample", "Placeholder", "Imaginary"]

    static func generate(
        schema: SampleDataSchema, entropy: any SampleDataEntropySource = SecureSampleDataEntropy()
    ) throws -> SampleDataResult {
        try validate(schema)
        var rows: [[SampleDataCell]] = []
        for _ in 0..<schema.rowCount {
            rows.append(
                try schema.fields.map {
                    SampleDataCell(fieldName: $0.name, value: try generate(field: $0, entropy: entropy))
                })
        }
        return SampleDataResult(
            rows: rows, output: schema.format == .json ? json(rows) : csv(fields: schema.fields, rows: rows))
    }

    static func validate(_ schema: SampleDataSchema) throws {
        guard (1...1_000).contains(schema.rowCount) else {
            throw UtilityError("Row count must be from 1 through 1,000.")
        }
        guard (1...50).contains(schema.fields.count) else {
            throw UtilityError("Define from 1 through 50 fields.")
        }
        var names: Set<String> = []
        for field in schema.fields {
            guard isValidName(field.name) else {
                throw UtilityError(
                    "Field names must begin with a letter or underscore and contain only ASCII letters, digits, and underscores."
                )
            }
            guard names.insert(field.name).inserted else { throw UtilityError("Field names must be unique.") }
            switch field.type {
            case .number:
                guard field.numberMinimum.isFinite, field.numberMaximum.isFinite,
                    field.numberMinimum <= field.numberMaximum
                else {
                    throw UtilityError(
                        "Number ranges require finite minimum and maximum values in ascending order.")
                }
                if field.numberIsInteger {
                    guard field.numberMinimum.rounded() == field.numberMinimum,
                        field.numberMaximum.rounded() == field.numberMaximum,
                        field.numberMinimum >= -9_000_000_000_000_000,
                        field.numberMaximum <= 9_000_000_000_000_000,
                        field.numberMaximum - field.numberMinimum <= 9_000_000_000_000_000
                    else {
                        throw UtilityError(
                            "Integer ranges require whole-number bounds with at most 9 quadrillion values.")
                    }
                } else if !(field.numberMaximum - field.numberMinimum).isFinite {
                    throw UtilityError("Decimal range width must be finite.")
                }
            case .date:
                guard field.dateMinimum <= field.dateMaximum,
                    field.dateMinimum.timeIntervalSince1970.isFinite,
                    field.dateMaximum.timeIntervalSince1970.isFinite,
                    field.dateMaximum.timeIntervalSince(field.dateMinimum).isFinite
                else {
                    throw UtilityError("Date ranges require the earliest date first.")
                }
            case .enumeration:
                guard !field.enumChoices.isEmpty, field.enumChoices.allSatisfy({ !$0.isEmpty }) else {
                    throw UtilityError("Enum choices must be explicit and non-empty.")
                }
            default: break
            }
        }
    }

    private static func isValidName(_ name: String) -> Bool {
        guard let first = name.unicodeScalars.first,
            (first.value >= 65 && first.value <= 90) || (first.value >= 97 && first.value <= 122)
                || first == "_"
        else { return false }
        return name.unicodeScalars.dropFirst().allSatisfy { scalar in
            (scalar.value >= 65 && scalar.value <= 90) || (scalar.value >= 97 && scalar.value <= 122)
                || (scalar.value >= 48 && scalar.value <= 57) || scalar == "_"
        }
    }

    private static func generate(field: SampleField, entropy: any SampleDataEntropySource) throws
        -> SampleValue
    {
        switch field.type {
        case .fictionalName:
            return .string(
                "\(firstNames[try index(firstNames.count, entropy)]) \(lastNames[try index(lastNames.count, entropy)])"
            )
        case .fictionalEmail:
            let first = firstNames[try index(firstNames.count, entropy)].replacingOccurrences(
                of: "Fictional ", with: ""
            ).lowercased()
            let last = lastNames[try index(lastNames.count, entropy)].lowercased()
            return .string("fictional.\(first).\(last)\(try index(10_000, entropy))@example.invalid")
        case .number:
            if field.numberIsInteger {
                let minimum = Int(field.numberMinimum), maximum = Int(field.numberMaximum)
                let offset = try index(maximum - minimum + 1, entropy)
                return .number(Double(minimum + offset), integer: true)
            }
            let unit = Double(try uint64(entropy)) / Double(UInt64.max)
            return .number(
                field.numberMinimum + (field.numberMaximum - field.numberMinimum) * unit, integer: false)
        case .boolean: return .boolean(try index(2, entropy) == 1)
        case .date:
            let span = max(0, field.dateMaximum.timeIntervalSince(field.dateMinimum))
            let unit = Double(try uint64(entropy)) / Double(UInt64.max)
            return .string(iso8601.string(from: field.dateMinimum.addingTimeInterval(span * unit)))
        case .uuid:
            var bytes = try entropy.bytes(count: 16)
            bytes[6] = (bytes[6] & 0x0F) | 0x40
            bytes[8] = (bytes[8] & 0x3F) | 0x80
            let hex = bytes.map { String(format: "%02x", $0) }
            return .string(
                hex[0..<4].joined() + "-" + hex[4..<6].joined() + "-" + hex[6..<8].joined() + "-"
                    + hex[8..<10].joined() + "-" + hex[10..<16].joined())
        case .enumeration: return .string(field.enumChoices[try index(field.enumChoices.count, entropy)])
        }
    }

    private static func index(_ upperBound: Int, _ entropy: any SampleDataEntropySource) throws -> Int {
        precondition(upperBound > 0)
        return Int(try uint64(entropy) % UInt64(upperBound))
    }

    private static func uint64(_ entropy: any SampleDataEntropySource) throws -> UInt64 {
        try entropy.bytes(count: 8).reduce(0) { ($0 << 8) | UInt64($1) }
    }

    private static var iso8601: ISO8601DateFormatter {
        let formatter = ISO8601DateFormatter()
        formatter.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
        formatter.timeZone = TimeZone(secondsFromGMT: 0)
        return formatter
    }

    private static func json(_ rows: [[SampleDataCell]]) -> String {
        "[\n"
            + rows.map { row in
                "  {\n"
                    + row.map { "    \(jsonString($0.fieldName)): \(jsonValue($0.value))" }.joined(
                        separator: ",\n") + "\n  }"
            }.joined(separator: ",\n") + "\n]"
    }

    private static func jsonValue(_ value: SampleValue) -> String {
        switch value {
        case .string(let value): jsonString(value)
        case .number(let value, let integer): integer ? String(Int(value)) : decimal(value)
        case .boolean(let value): value ? "true" : "false"
        }
    }

    private static func jsonString(_ value: String) -> String {
        let data = try! JSONEncoder().encode(value)
        return String(decoding: data, as: UTF8.self)
    }

    private static func csv(fields: [SampleField], rows: [[SampleDataCell]]) -> String {
        ([fields.map { quote($0.name) }.joined(separator: ",")]
            + rows.map { $0.map { quote($0.value.displayValue) }.joined(separator: ",") }).joined(
                separator: "\n") + "\n"
    }

    private static func quote(_ value: String) -> String {
        "\"" + value.replacingOccurrences(of: "\"", with: "\"\"") + "\""
    }

    static func decimal(_ value: Double) -> String {
        var result = String(format: "%.8f", locale: Locale(identifier: "en_US_POSIX"), value)
        while result.contains("."), result.last == "0" { result.removeLast() }
        if result.last == "." { result.removeLast() }
        return result
    }
}

struct SampleDataWorkspace: View {
    let context: UtilityWorkspaceContext
    @State private var schema = SampleDataSchema()
    @State private var rows: [[SampleDataCell]] = []
    @State private var output = ""
    @State private var diagnostic: String?
    @State private var pendingRestore: UtilityHistoryEntry?
    @State private var suppressSchemaInvalidation = false

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text(
                "Every generated identity is fictional. This local generator does not simulate locales, relationships, or real people."
            ).font(.caption).foregroundStyle(.secondary)
            HStack {
                Picker("Output", selection: $schema.format) {
                    ForEach(SampleDataFormat.allCases) { Text($0.rawValue).tag($0) }
                }.frame(width: 150)
                Stepper("Rows: \(schema.rowCount)", value: $schema.rowCount, in: 1...1_000)
                Button("Add Field") {
                    if schema.fields.count < 50 {
                        schema.fields.append(
                            .init(name: "field\(schema.fields.count + 1)", type: .enumeration))
                    }
                }
                Spacer()
                Button("Generate") { generate() }.keyboardShortcut(.return, modifiers: .command)
                    .accessibilityIdentifier("sample-data.generate")
            }
            ScrollView {
                VStack(spacing: 8) {
                    ForEach(Array(schema.fields.indices), id: \.self) { index in fieldRow(index) }
                }
            }.frame(maxHeight: 190)
            TextEditorCard(
                title: "Generated \(schema.format.rawValue)", text: $output, editable: false,
                accessibilityID: "sample-data.result")
            if let diagnostic { DiagnosticBanner(message: diagnostic) }
            HStack {
                Button("Clear") {
                    rows = []
                    output = ""
                    diagnostic = nil
                }
                Spacer()
                ConfirmedCopyButton(title: "Copy Result", isEnabled: !output.isEmpty) {
                    context.clipboard.writeText(output)
                }
            }
        }
        .padding(16)
        .onChange(of: schema) { _, _ in
            if suppressSchemaInvalidation {
                suppressSchemaInvalidation = false
            } else {
                output = ""
                rows = []
                diagnostic = nil
            }
        }
        .onReceive(NotificationCenter.default.publisher(for: .restoreUtilitySnapshot)) {
            requestRestore($0.object as? UtilityHistoryEntry)
        }
        .alert(
            "Replace current workspace?",
            isPresented: Binding(get: { pendingRestore != nil }, set: { if !$0 { pendingRestore = nil } })
        ) {
            Button("Restore", role: .destructive) {
                let entry = pendingRestore
                pendingRestore = nil
                restore(entry)
            }
            Button("Cancel", role: .cancel) { pendingRestore = nil }
        } message: {
            Text("Restoring this History entry replaces the current sample-data session.")
        }
    }

    private func fieldRow(_ index: Int) -> some View {
        HStack {
            TextField("Field name", text: $schema.fields[index].name).frame(width: 130)
            Picker("Type", selection: $schema.fields[index].type) {
                ForEach(SampleFieldType.allCases) { Text($0.rawValue).tag($0) }
            }.frame(width: 170)
            switch schema.fields[index].type {
            case .number:
                TextField("Min", value: $schema.fields[index].numberMinimum, format: .number).frame(width: 70)
                TextField("Max", value: $schema.fields[index].numberMaximum, format: .number).frame(width: 70)
                Toggle("Integer", isOn: $schema.fields[index].numberIsInteger)
            case .date:
                DatePicker("From", selection: $schema.fields[index].dateMinimum, displayedComponents: .date)
                DatePicker("To", selection: $schema.fields[index].dateMaximum, displayedComponents: .date)
            case .enumeration:
                TextField(
                    "Choices (comma-separated)",
                    text: Binding(
                        get: { schema.fields[index].enumChoices.joined(separator: ",") },
                        set: {
                            schema.fields[index].enumChoices = $0.split(
                                separator: ",", omittingEmptySubsequences: false
                            ).map(String.init)
                        }))
            default: EmptyView()
            }
            Button("Up") { if index > 0 { schema.fields.swapAt(index, index - 1) } }.disabled(index == 0)
            Button("Down") { if index + 1 < schema.fields.count { schema.fields.swapAt(index, index + 1) } }
                .disabled(index + 1 == schema.fields.count)
            Button("Remove", role: .destructive) { schema.fields.remove(at: index) }
        }
    }

    private func generate() {
        do {
            let result = try SampleDataEngine.generate(schema: schema)
            rows = result.rows
            output = result.output
            diagnostic = nil
            let snapshot = SampleDataSnapshot(schema: schema, rows: rows, output: output)
            if let payload = try? JSONEncoder().encode(snapshot) {
                context.record(.init(utilityID: "sample-data", schemaVersion: 1, payload: payload))
            }
        } catch {
            rows = []
            output = ""
            diagnostic = error.localizedDescription
        }
    }

    private func restore(_ entry: UtilityHistoryEntry?) {
        guard let entry, entry.utilityID == "sample-data",
            let snapshot = try? JSONDecoder().decode(SampleDataSnapshot.self, from: entry.payload)
        else { return }
        suppressSchemaInvalidation = true
        schema = snapshot.schema
        rows = snapshot.rows
        output = snapshot.output
        diagnostic = nil
    }

    private func requestRestore(_ entry: UtilityHistoryEntry?) {
        guard let entry, entry.utilityID == "sample-data",
            let snapshot = try? JSONDecoder().decode(SampleDataSnapshot.self, from: entry.payload)
        else { return }
        if !output.isEmpty, output != snapshot.output { pendingRestore = entry } else { restore(entry) }
    }
}
