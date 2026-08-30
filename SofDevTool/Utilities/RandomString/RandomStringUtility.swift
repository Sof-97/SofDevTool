import Foundation
import Security
import SwiftUI

protocol RandomBytesSource { func bytes(count: Int) throws -> [UInt8] }
struct SecureRandomBytes: RandomBytesSource {
    func bytes(count: Int) throws -> [UInt8] {
        var bytes = [UInt8](repeating: 0, count: count)
        guard SecRandomCopyBytes(kSecRandomDefault, count, &bytes) == errSecSuccess else {
            throw UtilityError("The system random-number generator failed.")
        }
        return bytes
    }
}

struct RandomStringOptions: Codable, Equatable {
    var length = 20
    var count = 1
    var uppercase = true
    var lowercase = true
    var digits = true
    var symbols = true
    var custom = ""
    var excludeAmbiguous = true
    var alphabet: [Character] {
        var value = ""
        if uppercase { value += "ABCDEFGHIJKLMNOPQRSTUVWXYZ" }
        if lowercase { value += "abcdefghijklmnopqrstuvwxyz" }
        if digits { value += "0123456789" }
        if symbols { value += "-._~!@#$%^&*" }
        value += custom
        if excludeAmbiguous { value.removeAll { "0O1lI|`'\"".contains($0) } }
        return Array(Set(value)).sorted()
    }
}
struct RandomStringSnapshot: Codable {
    let output: String
    let options: RandomStringOptions
}

enum RandomStringEngine {
    static func generate(options: RandomStringOptions, random: RandomBytesSource = SecureRandomBytes()) throws
        -> [String]
    {
        guard (1...4096).contains(options.length), (1...100).contains(options.count) else {
            throw UtilityError("Length must be 1–4096 and count must be 1–100.")
        }
        let alphabet = options.alphabet
        guard alphabet.count >= 2 else { throw UtilityError("Choose at least two distinct characters.") }
        let cutoff = 256 - (256 % alphabet.count)
        var results: [String] = []
        for _ in 0..<options.count {
            var result = ""
            while result.count < options.length {
                for byte in try random.bytes(count: max(32, options.length)) where Int(byte) < cutoff {
                    result.append(alphabet[Int(byte) % alphabet.count])
                    if result.count == options.length { break }
                }
            }
            results.append(result)
        }
        return results
    }
}

struct RandomStringWorkspace: View {
    let context: UtilityWorkspaceContext
    @State private var options = RandomStringOptions()
    @State private var output = ""
    @State private var diagnostic: String?
    @State private var pendingRestore: UtilityHistoryEntry?

    init(context: UtilityWorkspaceContext) {
        self.context = context
        if let data = UserDefaults.standard.data(forKey: "random-string.options"),
            let saved = try? JSONDecoder().decode(RandomStringOptions.self, from: data)
        {
            _options = State(initialValue: saved)
        }
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack {
                Stepper("Length: \(options.length)", value: $options.length, in: 1...4096)
                Stepper("Count: \(options.count)", value: $options.count, in: 1...100)
            }
            HStack {
                Toggle("A–Z", isOn: $options.uppercase)
                Toggle("a–z", isOn: $options.lowercase)
                Toggle("0–9", isOn: $options.digits)
                Toggle("Safe symbols", isOn: $options.symbols)
                Toggle("Exclude ambiguous", isOn: $options.excludeAmbiguous)
            }
            TextField("Custom characters", text: $options.custom).textFieldStyle(.roundedBorder)
            Text(
                "Alphabet: \(options.alphabet.count) characters · entropy per result: \(entropy, format: .number.precision(.fractionLength(1))) bits"
            ).font(.caption).foregroundStyle(.secondary)
            TextEditorCard(
                title: "Generated Results", text: $output, editable: false, accessibilityID: "random.result")
            if let diagnostic { DiagnosticBanner(message: diagnostic) }
            HStack {
                Button("Generate") { generate() }.keyboardShortcut(.return, modifiers: .command)
                    .accessibilityIdentifier("random.generate")
                Button("Clear") {
                    output = ""
                    diagnostic = nil
                }
                Spacer()
                Button("Copy All") { context.clipboard.writeText(output) }.disabled(output.isEmpty)
            }
        }.padding(16)
            .onReceive(NotificationCenter.default.publisher(for: .restoreUtilitySnapshot)) {
                requestRestore($0.object as? UtilityHistoryEntry)
            }
            .onChange(of: options) { _, value in
                UserDefaults.standard.set(
                    try? JSONEncoder().encode(value), forKey: "random-string.options")
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
                Text("Restoring this History entry replaces the current generated results.")
            }
    }

    private var entropy: Double { Double(options.length) * log2(Double(max(1, options.alphabet.count))) }
    private func generate() {
        do {
            output = try RandomStringEngine.generate(options: options).joined(separator: "\n")
            diagnostic = nil
            if let payload = try? JSONEncoder().encode(RandomStringSnapshot(output: output, options: options))
            {
                context.record(.init(utilityID: "random-string", schemaVersion: 1, payload: payload))
            }
        } catch {
            output = ""
            diagnostic = error.localizedDescription
        }
    }
    private func restore(_ entry: UtilityHistoryEntry?) {
        guard let entry, entry.utilityID == "random-string",
            let snapshot = try? JSONDecoder().decode(RandomStringSnapshot.self, from: entry.payload)
        else { return }
        output = snapshot.output
        options = snapshot.options
        diagnostic = nil
    }

    private func requestRestore(_ entry: UtilityHistoryEntry?) {
        guard let entry, entry.utilityID == "random-string",
            let snapshot = try? JSONDecoder().decode(RandomStringSnapshot.self, from: entry.payload)
        else { return }
        if !output.isEmpty, output != snapshot.output { pendingRestore = entry } else { restore(entry) }
    }
}
