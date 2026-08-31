import Foundation

struct UtilityOperationSnapshot: Codable, Sendable {
    let utilityID: String
    let schemaVersion: Int
    let payload: Data
}

struct UtilityHistoryEntry: Codable, Identifiable, Sendable {
    let id: UUID
    let capturedAt: Date
    let utilityID: String
    let schemaVersion: Int
    let payload: Data
}

private struct HistoryFile: Codable {
    let version: Int
    var entries: [UtilityHistoryEntry]
}

@MainActor
final class HistoryRepository: ObservableObject {
    @Published private(set) var entriesByUtility: [String: [UtilityHistoryEntry]] = [:]
    @Published private(set) var malformedUtilityIDs: Set<String> = []
    @Published var isRecordingEnabled: Bool {
        didSet { defaults.set(isRecordingEnabled, forKey: "history.global.enabled") }
    }

    private let directory: URL
    private let defaults: UserDefaults
    private let fileManager: FileManager
    private var pausedUtilityIDs: Set<String> = []

    init(directory: URL? = nil, defaults: UserDefaults = .standard, fileManager: FileManager = .default) {
        self.defaults = defaults
        self.fileManager = fileManager
        if defaults.object(forKey: "history.global.enabled") == nil {
            defaults.set(true, forKey: "history.global.enabled")
        }
        isRecordingEnabled = defaults.bool(forKey: "history.global.enabled")
        self.directory =
            directory
            ?? fileManager.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0].appending(
                path: "SofDevTool/History", directoryHint: .isDirectory)
        loadKnownUtilities()
    }

    func isEnabled(for utilityID: String, default defaultValue: Bool = true) -> Bool {
        let key = "history.utility.\(utilityID).enabled"
        return defaults.object(forKey: key) == nil ? defaultValue : defaults.bool(forKey: key)
    }

    func setEnabled(_ enabled: Bool, for utilityID: String) {
        defaults.set(enabled, forKey: "history.utility.\(utilityID).enabled")
    }

    func entries(for utilityID: String) -> [UtilityHistoryEntry] { entriesByUtility[utilityID] ?? [] }

    func record(
        _ snapshot: UtilityOperationSnapshot,
        defaultEnabled: Bool = true,
        at date: Date = Date(),
        id: UUID = UUID()
    ) throws {
        guard isRecordingEnabled,
            isEnabled(for: snapshot.utilityID, default: defaultEnabled),
            !pausedUtilityIDs.contains(snapshot.utilityID)
        else { return }
        let entry = UtilityHistoryEntry(
            id: id, capturedAt: date, utilityID: snapshot.utilityID, schemaVersion: snapshot.schemaVersion,
            payload: snapshot.payload)
        var entries = entries(for: snapshot.utilityID)
        entries.append(entry)
        entries.sort {
            $0.capturedAt == $1.capturedAt
                ? $0.id.uuidString > $1.id.uuidString : $0.capturedAt > $1.capturedAt
        }
        entries = Array(entries.prefix(25))
        do {
            try fileManager.createDirectory(at: directory, withIntermediateDirectories: true)
            let data = try JSONEncoder.history.encode(HistoryFile(version: 1, entries: entries))
            try data.write(to: fileURL(snapshot.utilityID), options: .atomic)
            entriesByUtility[snapshot.utilityID] = entries
            pausedUtilityIDs.remove(snapshot.utilityID)
        } catch {
            pausedUtilityIDs.insert(snapshot.utilityID)
            throw error
        }
    }

    func retry(_ utilityID: String) { pausedUtilityIDs.remove(utilityID) }

    func clear(_ utilityID: String) throws {
        if fileManager.fileExists(atPath: fileURL(utilityID).path) {
            try fileManager.removeItem(at: fileURL(utilityID))
        }
        entriesByUtility[utilityID] = []
        malformedUtilityIDs.remove(utilityID)
        pausedUtilityIDs.remove(utilityID)
    }

    func clearAll() throws {
        if fileManager.fileExists(atPath: directory.path) { try fileManager.removeItem(at: directory) }
        entriesByUtility = [:]
        malformedUtilityIDs = []
        pausedUtilityIDs = []
    }

    private func loadKnownUtilities() {
        guard let urls = try? fileManager.contentsOfDirectory(at: directory, includingPropertiesForKeys: nil)
        else { return }
        for url in urls where url.pathExtension == "json" {
            let utilityID = url.deletingPathExtension().lastPathComponent
            do {
                let file = try JSONDecoder.history.decode(HistoryFile.self, from: Data(contentsOf: url))
                entriesByUtility[utilityID] = file.entries.sorted { $0.capturedAt > $1.capturedAt }
            } catch { malformedUtilityIDs.insert(utilityID) }
        }
    }

    private func fileURL(_ utilityID: String) -> URL { directory.appending(path: utilityID + ".json") }
}

private extension JSONEncoder {
    static var history: JSONEncoder {
        let encoder = JSONEncoder()
        encoder.dateEncodingStrategy = .iso8601
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
        return encoder
    }
}

private extension JSONDecoder {
    static var history: JSONDecoder {
        let decoder = JSONDecoder()
        decoder.dateDecodingStrategy = .iso8601
        return decoder
    }
}
