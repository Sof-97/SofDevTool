import CryptoKit
import Foundation
import SwiftUI

enum IdentifierFormat: String, CaseIterable, Codable, Identifiable {
    case uuid = "UUID", ulid = "ULID", ksuid = "KSUID"
    var id: Self { self }
}
enum UUIDVersion: Int, CaseIterable, Codable, Identifiable {
    case v1 = 1, v3 = 3, v4 = 4, v5 = 5, v6 = 6, v7 = 7
    var id: Self { self }
    var label: String { "v\(rawValue)" }
}
enum ULIDMode: String, CaseIterable, Codable, Identifiable {
    case random = "Random", monotonic = "Process-local monotonic"
    var id: Self { self }
}

struct IdentifierOptions: Codable {
    var format: IdentifierFormat = .uuid
    var count = 1
    var uuidVersion: UUIDVersion = .v4
    var uppercase = false
    var hyphenated = true
    var namespace = "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
    var name = ""
    var ulidMode: ULIDMode = .random
    var orderedKSUID = false
}
struct IdentifierSnapshot: Codable {
    let output: String
    let inspectInput: String
    let inspection: String
    let options: IdentifierOptions
}

enum IdentifierEngine {
    static func uuid(
        version: UUIDVersion, namespace: String, name: String, date: Date = Date(),
        random: RandomBytesSource = SecureRandomBytes()
    ) throws -> [UInt8] {
        switch version {
        case .v1: return try timeUUID(version: 1, date: date, random: random)
        case .v3, .v5:
            guard let namespaceBytes = uuidBytes(namespace) else {
                throw UtilityError("Namespace must be a valid UUID.")
            }
            let source = namespaceBytes + Array(name.utf8)
            var digest =
                version == .v3
                ? Array(Insecure.MD5.hash(data: Data(source))) : Array(Insecure.SHA1.hash(data: Data(source)))
            digest = Array(digest.prefix(16))
            digest[6] = (digest[6] & 0x0F) | UInt8(version.rawValue << 4)
            digest[8] = (digest[8] & 0x3F) | 0x80
            return digest
        case .v4:
            var bytes = try random.bytes(count: 16)
            bytes[6] = (bytes[6] & 0x0F) | 0x40
            bytes[8] = (bytes[8] & 0x3F) | 0x80
            return bytes
        case .v6: return try timeUUID(version: 6, date: date, random: random)
        case .v7:
            var bytes = try random.bytes(count: 16)
            let milliseconds = UInt64(max(0, date.timeIntervalSince1970 * 1000))
            for index in 0..<6 { bytes[index] = UInt8((milliseconds >> UInt64((5 - index) * 8)) & 0xFF) }
            bytes[6] = (bytes[6] & 0x0F) | 0x70
            bytes[8] = (bytes[8] & 0x3F) | 0x80
            return bytes
        }
    }

    static func formatUUID(_ bytes: [UInt8], uppercase: Bool, hyphenated: Bool) -> String {
        let hex = bytes.map { String(format: "%02x", $0) }.joined()
        let value =
            hyphenated
            ? "\(hex.prefix(8))-\(hex.dropFirst(8).prefix(4))-\(hex.dropFirst(12).prefix(4))-\(hex.dropFirst(16).prefix(4))-\(hex.dropFirst(20))"
            : hex
        return uppercase ? value.uppercased() : value
    }

    static func normalizeUUID(_ input: String, uppercase: Bool, hyphenated: Bool) throws -> String {
        let compact = input.replacingOccurrences(of: "-", with: "")
        guard compact.count == 32, compact.allSatisfy({ $0.isHexDigit }), let bytes = hexBytes(compact),
            (1...8).contains(Int((bytes[6] & 0xF0) >> 4)), bytes[8] & 0xC0 == 0x80
        else { throw UtilityError("Enter a strict RFC 9562 UUID with a supported variant and version.") }
        return formatUUID(bytes, uppercase: uppercase, hyphenated: hyphenated)
    }

    static func ulid(
        date: Date = Date(), random: RandomBytesSource = SecureRandomBytes(), previous: [UInt8]? = nil
    ) throws -> (String, [UInt8]) {
        var bytes = try random.bytes(count: 16)
        let milliseconds = UInt64(max(0, date.timeIntervalSince1970 * 1000))
        for index in 0..<6 { bytes[index] = UInt8((milliseconds >> UInt64((5 - index) * 8)) & 0xFF) }
        if var previous, Array(previous.prefix(6)) == Array(bytes.prefix(6)) {
            for index in stride(from: 15, through: 6, by: -1) {
                previous[index] &+= 1
                if previous[index] != 0 { break }
            }
            bytes = previous
        }
        return (encodeBase32(bytes), bytes)
    }

    static func inspectULID(_ input: String) throws -> String {
        let alphabet = Set("0123456789ABCDEFGHJKMNPQRSTVWXYZ")
        let canonical = input.uppercased()
        guard canonical.count == 26, canonical.first.map({ "01234567".contains($0) }) == true,
            canonical.allSatisfy(alphabet.contains)
        else { throw UtilityError("Enter a canonical 26-character ULID.") }
        var milliseconds: UInt64 = 0
        for character in canonical.prefix(10) {
            milliseconds = milliseconds * 32 + UInt64(base32Alphabet.firstIndex(of: character)!)
        }
        return
            "Valid ULID · \(milliseconds) ms · \(ISO8601DateFormatter().string(from: Date(timeIntervalSince1970: Double(milliseconds) / 1000)))"
    }

    static func ksuid(
        date: Date = Date(), random: RandomBytesSource = SecureRandomBytes(), sequence: UInt16? = nil
    ) throws -> String {
        let seconds = UInt32(max(0, Int64(date.timeIntervalSince1970) - 1_400_000_000))
        var bytes =
            [
                UInt8(seconds >> 24), UInt8((seconds >> 16) & 0xFF), UInt8((seconds >> 8) & 0xFF),
                UInt8(seconds & 0xFF),
            ] + (try random.bytes(count: 16))
        if let sequence {
            bytes[18] = UInt8(sequence >> 8)
            bytes[19] = UInt8(sequence & 0xFF)
        }
        return encodeBase62(bytes)
    }

    static func inspectKSUID(_ input: String) throws -> String {
        guard input.count == 27, input.allSatisfy({ base62Alphabet.contains($0) }),
            let bytes = decodeBase62(input, byteCount: 20)
        else { throw UtilityError("Enter a canonical, case-sensitive 27-character KSUID.") }
        let seconds =
            UInt32(bytes[0]) << 24 | UInt32(bytes[1]) << 16 | UInt32(bytes[2]) << 8 | UInt32(bytes[3])
        let date = Date(timeIntervalSince1970: Double(UInt64(seconds) + 1_400_000_000))
        return "Valid KSUID · \(ISO8601DateFormatter().string(from: date))"
    }

    private static func timeUUID(version: Int, date: Date, random: RandomBytesSource) throws -> [UInt8] {
        let timestamp = UInt64(max(0, date.timeIntervalSince1970 * 10_000_000)) + 122_192_928_000_000_000
        let randomBytes = try random.bytes(count: 8)
        var bytes = [UInt8](repeating: 0, count: 16)
        if version == 1 {
            let low = UInt32(timestamp & 0xFFFF_FFFF), mid = UInt16((timestamp >> 32) & 0xFFFF),
                high = UInt16((timestamp >> 48) & 0x0FFF) | 0x1000
            bytes[0] = UInt8(low >> 24)
            bytes[1] = UInt8((low >> 16) & 0xFF)
            bytes[2] = UInt8((low >> 8) & 0xFF)
            bytes[3] = UInt8(low & 0xFF)
            bytes[4] = UInt8(mid >> 8)
            bytes[5] = UInt8(mid & 0xFF)
            bytes[6] = UInt8(high >> 8)
            bytes[7] = UInt8(high & 0xFF)
        } else {
            bytes[0] = UInt8((timestamp >> 52) & 0xFF)
            bytes[1] = UInt8((timestamp >> 44) & 0xFF)
            bytes[2] = UInt8((timestamp >> 36) & 0xFF)
            bytes[3] = UInt8((timestamp >> 28) & 0xFF)
            bytes[4] = UInt8((timestamp >> 20) & 0xFF)
            bytes[5] = UInt8((timestamp >> 12) & 0xFF)
            bytes[6] = UInt8((timestamp >> 8) & 0x0F) | 0x60
            bytes[7] = UInt8(timestamp & 0xFF)
        }
        bytes[8] = (randomBytes[0] & 0x3F) | 0x80
        for index in 9..<16 { bytes[index] = randomBytes[index - 8] }
        return bytes
    }

    private static let base32Alphabet = Array("0123456789ABCDEFGHJKMNPQRSTVWXYZ")
    private static let base62Alphabet = Array(
        "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz")

    private static func encodeBase32(_ bytes: [UInt8]) -> String {
        var output = ""
        for group in 0..<26 {
            var value = 0
            for bit in 0..<5 {
                let streamBit = group * 5 + bit - 2
                value <<= 1
                if streamBit >= 0 { value |= Int((bytes[streamBit / 8] >> UInt8(7 - streamBit % 8)) & 1) }
            }
            output.append(base32Alphabet[value])
        }
        return output
    }

    private static func encodeBase62(_ input: [UInt8]) -> String {
        var number = input, encoded: [Character] = []
        while number.contains(where: { $0 != 0 }) {
            var quotient: [UInt8] = [], remainder = 0
            for byte in number {
                let value = remainder * 256 + Int(byte)
                if !quotient.isEmpty || value / 62 != 0 { quotient.append(UInt8(value / 62)) }
                remainder = value % 62
            }
            encoded.append(base62Alphabet[remainder])
            number = quotient
        }
        return String(repeating: "0", count: max(0, 27 - encoded.count)) + String(encoded.reversed())
    }

    private static func decodeBase62(_ input: String, byteCount: Int) -> [UInt8]? {
        var bytes = [UInt8](repeating: 0, count: byteCount)
        for character in input {
            guard let digit = base62Alphabet.firstIndex(of: character) else { return nil }
            var carry = digit
            for index in stride(from: byteCount - 1, through: 0, by: -1) {
                let value = Int(bytes[index]) * 62 + carry
                bytes[index] = UInt8(value & 0xFF)
                carry = value >> 8
            }
            if carry != 0 { return nil }
        }
        return bytes
    }

    private static func uuidBytes(_ input: String) -> [UInt8]? {
        hexBytes(input.replacingOccurrences(of: "-", with: ""))
    }
    private static func hexBytes(_ input: String) -> [UInt8]? {
        guard input.count == 32 else { return nil }
        var bytes: [UInt8] = []
        var index = input.startIndex
        for _ in 0..<16 {
            let next = input.index(index, offsetBy: 2)
            guard let byte = UInt8(input[index..<next], radix: 16) else { return nil }
            bytes.append(byte)
            index = next
        }
        return bytes
    }
}

struct IdentifierWorkspace: View {
    let context: UtilityWorkspaceContext
    @State private var options = IdentifierOptions()
    @State private var output = ""
    @State private var inspectInput = ""
    @State private var inspection = ""
    @State private var diagnostic: String?
    @State private var previousULID: [UInt8]?
    @State private var pendingRestore: UtilityHistoryEntry?

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Picker("Identifier format", selection: $options.format) {
                ForEach(IdentifierFormat.allCases) { Text($0.rawValue).tag($0) }
            }.pickerStyle(.segmented)
            Group {
                switch options.format {
                case .uuid: uuidControls
                case .ulid:
                    Picker("Mode", selection: $options.ulidMode) {
                        ForEach(ULIDMode.allCases) { Text($0.rawValue).tag($0) }
                    }.frame(width: 260)
                case .ksuid: Toggle("Ordered-sequence batch", isOn: $options.orderedKSUID)
                }
            }
            HStack {
                Stepper(
                    "Count: \(options.count)", value: $options.count,
                    in: 1...(options.format == .ksuid && options.orderedKSUID ? 65_536 : 1_000))
                Spacer()
                Button("Generate") { generate() }.keyboardShortcut(.return, modifiers: .command)
                    .accessibilityIdentifier("identifier.generate")
            }
            Text("Identifiers are collision-resistant, not guaranteed unique or secret.").font(.caption)
                .foregroundStyle(.secondary)
            TextEditorCard(
                title: "Generated Identifiers", text: $output, editable: false,
                accessibilityID: "identifier.result"
            ).frame(minHeight: 140)
            HStack {
                TextField("Paste an identifier to validate, normalize, or inspect", text: $inspectInput)
                    .textFieldStyle(.roundedBorder)
                Button("Validate") { validate() }
                Button("Paste") { inspectInput = context.clipboard.readText() ?? inspectInput }
            }
            if !inspection.isEmpty { Text(inspection).font(.callout).textSelection(.enabled) }
            if let diagnostic { DiagnosticBanner(message: diagnostic) }
            HStack {
                Button("Clear") {
                    output = ""
                    inspectInput = ""
                    inspection = ""
                    diagnostic = nil
                }
                Spacer()
                Button("Copy All") { context.clipboard.writeText(output) }.disabled(output.isEmpty)
            }
        }.padding(16)
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
                Text("Restoring this History entry replaces the current Identifier Generator session.")
            }
    }

    private var uuidControls: some View {
        VStack(alignment: .leading) {
            HStack {
                Picker("Version", selection: $options.uuidVersion) {
                    ForEach(UUIDVersion.allCases) { Text($0.label).tag($0) }
                }.frame(width: 110)
                Toggle("Uppercase", isOn: $options.uppercase)
                Toggle("Hyphens", isOn: $options.hyphenated)
            }
            if options.uuidVersion == .v3 || options.uuidVersion == .v5 {
                TextField("Namespace UUID", text: $options.namespace).textFieldStyle(.roundedBorder)
                TextField("Name", text: $options.name).textFieldStyle(.roundedBorder)
            }
            if options.uuidVersion == .v7 {
                Text("v7 is recommended for sortable UUIDs.").font(.caption).foregroundStyle(.secondary)
            }
        }
    }

    private func generate() {
        do {
            var values: [String] = []
            switch options.format {
            case .uuid:
                for _ in 0..<options.count {
                    values.append(
                        IdentifierEngine.formatUUID(
                            try IdentifierEngine.uuid(
                                version: options.uuidVersion, namespace: options.namespace, name: options.name
                            ), uppercase: options.uppercase, hyphenated: options.hyphenated))
                }
            case .ulid:
                for _ in 0..<options.count {
                    let value = try IdentifierEngine.ulid(
                        previous: options.ulidMode == .monotonic ? previousULID : nil)
                    values.append(value.0)
                    previousULID = value.1
                }
            case .ksuid:
                for index in 0..<options.count {
                    values.append(
                        try IdentifierEngine.ksuid(sequence: options.orderedKSUID ? UInt16(index) : nil))
                }
            }
            output = values.joined(separator: "\n")
            diagnostic = nil
            record()
        } catch {
            output = ""
            diagnostic = error.localizedDescription
        }
    }

    private func validate() {
        do {
            switch options.format {
            case .uuid:
                inspection =
                    "Valid UUID · normalized: \(try IdentifierEngine.normalizeUUID(inspectInput, uppercase: options.uppercase, hyphenated: options.hyphenated))"
            case .ulid: inspection = try IdentifierEngine.inspectULID(inspectInput)
            case .ksuid: inspection = try IdentifierEngine.inspectKSUID(inspectInput)
            }
            diagnostic = nil
            record()
        } catch {
            inspection = ""
            diagnostic = error.localizedDescription
        }
    }
    private func record() {
        guard
            let payload = try? JSONEncoder().encode(
                IdentifierSnapshot(
                    output: output, inspectInput: inspectInput, inspection: inspection, options: options))
        else { return }
        context.record(.init(utilityID: "identifiers", schemaVersion: 1, payload: payload))
    }
    private func restore(_ entry: UtilityHistoryEntry?) {
        guard let entry, entry.utilityID == "identifiers",
            let snapshot = try? JSONDecoder().decode(IdentifierSnapshot.self, from: entry.payload)
        else { return }
        output = snapshot.output
        inspectInput = snapshot.inspectInput
        inspection = snapshot.inspection
        options = snapshot.options
        diagnostic = nil
    }

    private func requestRestore(_ entry: UtilityHistoryEntry?) {
        guard let entry, entry.utilityID == "identifiers",
            let snapshot = try? JSONDecoder().decode(IdentifierSnapshot.self, from: entry.payload)
        else { return }
        let currentNonEmpty = !output.isEmpty || !inspectInput.isEmpty
        let differs = output != snapshot.output || inspectInput != snapshot.inspectInput
        if currentNonEmpty, differs { pendingRestore = entry } else { restore(entry) }
    }
}
