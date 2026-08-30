import SwiftUI

struct WorkbenchView: View {
    @EnvironmentObject private var model: AppModel
    @State private var historyExpanded = true

    var body: some View {
        NavigationSplitView {
            UtilitySidebar()
                .navigationSplitViewColumnWidth(min: 250, ideal: 290, max: 360)
        } detail: {
            VStack(spacing: 0) {
                if let definition = model.registry.definition(id: model.selectedUtilityID) {
                    HStack {
                        Label(definition.name, systemImage: definition.symbol).font(.headline)
                        Spacer()
                        Button {
                            model.toggleFavorite(definition.id)
                        } label: {
                            Image(systemName: model.favorites.contains(definition.id) ? "star.fill" : "star")
                        }
                        .buttonStyle(.plain)
                        .help("Toggle Favorite")
                    }
                    .padding(.horizontal, 18).padding(.vertical, 12)
                    Divider()
                    HSplitView {
                        model.workspace(for: definition.id)
                            .frame(minWidth: 440, maxWidth: .infinity, maxHeight: .infinity)
                        if historyExpanded {
                            HistoryInspector(utilityID: definition.id, expanded: $historyExpanded)
                                .frame(minWidth: 230, idealWidth: 280, maxWidth: 360)
                        }
                    }
                }
            }
        }
        .toolbar {
            ToolbarItemGroup(placement: .navigation) {
                Picker("Scope", selection: $model.scope) {
                    ForEach(CatalogScope.allCases) { Text($0.rawValue).tag($0) }
                }
                .pickerStyle(.segmented)
                .accessibilityIdentifier("catalog.scope")
            }
            ToolbarItem(placement: .principal) {
                TextField("Search Utilities", text: $model.search)
                    .textFieldStyle(.roundedBorder).frame(width: 300)
                    .accessibilityIdentifier("catalog.search")
            }
            ToolbarItemGroup {
                Button {
                    historyExpanded.toggle()
                } label: {
                    Label("History", systemImage: "clock.arrow.circlepath")
                }
                Button {
                    model.showLauncher()
                } label: {
                    Label("Launcher", systemImage: "command")
                }
                .keyboardShortcut("k", modifiers: .command)
                .accessibilityIdentifier("toolbar.launcher")
                SettingsLink { Label("Settings", systemImage: "gearshape") }
                    .accessibilityIdentifier("toolbar.settings")
            }
        }
        .alert(
            "History warning",
            isPresented: Binding(
                get: { model.historyWarning != nil }, set: { if !$0 { model.historyWarning = nil } })
        ) {
            Button("OK", role: .cancel) {}
        } message: {
            Text(model.historyWarning ?? "")
        }
    }
}

private struct UtilitySidebar: View {
    @EnvironmentObject private var model: AppModel

    var body: some View {
        if model.visibleDefinitions.isEmpty {
            ContentUnavailableView.search(text: model.search)
                .overlay(alignment: .bottom) { Button("Clear Search") { model.search = "" }.padding() }
        } else {
            List(
                selection: Binding(
                    get: { model.selectedUtilityID }, set: { if let id = $0 { model.open(id) } })
            ) {
                ForEach(UtilityCategory.allCases, id: \.self) { category in
                    let definitions = model.visibleDefinitions.filter { $0.category == category }
                    if !definitions.isEmpty {
                        Section(category.rawValue) {
                            ForEach(definitions) { definition in
                                Label(definition.name, systemImage: definition.symbol)
                                    .tag(definition.id)
                                    .accessibilityIdentifier("catalog.utility.\(definition.id)")
                            }
                        }
                    }
                }
            }
            .accessibilityIdentifier("catalog.list")
        }
    }
}
