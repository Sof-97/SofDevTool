import AppKit
import SwiftUI

@MainActor
final class AppModel: ObservableObject {
    @Published var selectedUtilityID: String
    @Published var scope: CatalogScope = .library
    @Published var search = ""
    @Published var favorites: Set<String>
    @Published var recents: [String]
    @Published var historyWarning: String?

    let registry = UtilityRegistry.standard
    let history: HistoryRepository
    let clipboard = ClipboardAdapter()
    var shortcutController: GlobalShortcutController?
    var showLauncher: () -> Void = {}

    private var workspaces: [String: AnyView] = [:]
    private let defaults: UserDefaults

    init(defaults: UserDefaults = .standard, history: HistoryRepository? = nil) {
        self.defaults = defaults
        selectedUtilityID = defaults.string(forKey: "selectedUtilityID") ?? "json"
        favorites = Set(defaults.stringArray(forKey: "favoriteUtilityIDs") ?? [])
        recents = defaults.stringArray(forKey: "recentUtilityIDs") ?? []
        self.history = history ?? HistoryRepository()
    }

    var visibleDefinitions: [UtilityDefinition] {
        let base: [UtilityDefinition]
        switch scope {
        case .library: base = registry.definitions
        case .recent: base = recents.compactMap(registry.definition(id:))
        case .favorites: base = registry.definitions.filter { favorites.contains($0.id) }
        }
        return registry.search(search, within: base)
    }

    func open(_ id: String) {
        guard registry.definition(id: id) != nil else { return }
        selectedUtilityID = id
        defaults.set(id, forKey: "selectedUtilityID")
        recents.removeAll { $0 == id }
        recents.insert(id, at: 0)
        recents = Array(recents.prefix(12))
        defaults.set(recents, forKey: "recentUtilityIDs")
    }

    func toggleFavorite(_ id: String) {
        if favorites.contains(id) { favorites.remove(id) } else { favorites.insert(id) }
        defaults.set(Array(favorites).sorted(), forKey: "favoriteUtilityIDs")
        objectWillChange.send()
    }

    func workspace(for id: String) -> AnyView {
        if let existing = workspaces[id] { return existing }
        guard let definition = registry.definition(id: id) else {
            return AnyView(
                ContentUnavailableView("Utility unavailable", systemImage: "questionmark.square.dashed"))
        }
        let context = UtilityWorkspaceContext(
            clipboard: clipboard,
            record: { [weak self] snapshot in
                do { try self?.history.record(snapshot) } catch {
                    self?.historyWarning = "History could not be saved. Your current result is unchanged."
                }
            }
        )
        let workspace = definition.workspaceFactory(context)
        workspaces[id] = workspace
        return workspace
    }
}

enum CatalogScope: String, CaseIterable, Identifiable {
    case library = "Library"
    case recent = "Recent"
    case favorites = "Favorites"
    var id: Self { self }
}
