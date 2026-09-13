import SwiftUI

extension Notification.Name {
    static let focusUtilitySearch = Notification.Name("focusUtilitySearch")
}

struct WorkbenchView: View {
    @EnvironmentObject private var model: AppModel
    @Environment(\.colorSchemeContrast) private var contrast
    @State private var historyExpanded = true

    private var palette: SemanticThemePalette { model.theme.palette }

    var body: some View {
        GeometryReader { geometry in
            VStack(spacing: 0) {
                PrecisionToolbar(historyExpanded: $historyExpanded)
                themedSeparator
                HSplitView {
                    UtilitySidebar()
                        .frame(minWidth: 235, idealWidth: 270, maxWidth: 320)
                        .frame(maxHeight: .infinity, alignment: .top)

                    if let definition = model.registry.definition(id: model.selectedUtilityID) {
                        UtilityWorkspacePane(definition: definition)
                            .frame(minWidth: 440, maxWidth: .infinity, maxHeight: .infinity)

                        if historyExpanded && geometry.size.width >= 1_020 {
                            HistoryInspector(utilityID: definition.id, expanded: $historyExpanded)
                                .frame(minWidth: 220, idealWidth: 245, maxWidth: 310)
                                .frame(maxHeight: .infinity, alignment: .top)
                        }
                    }
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
            }
            .background(palette.windowBackground.color)
            .foregroundStyle(palette.primaryText.color)
            .tint(palette.appAccent.color)
            .overlay(alignment: .topLeading) {
                ThemeAccessibilityValue(surface: "Workbench", theme: model.theme)
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

    private var themedSeparator: some View {
        palette.separator.color.opacity(contrast == .increased ? 0.72 : 0.34).frame(height: 1)
    }
}

struct ThemeAccessibilityValue: View {
    let surface: String
    let theme: AppTheme
    var accessibilityID: String? = nil

    var body: some View {
        if ProcessInfo.processInfo.arguments.contains("-ui-testing") {
            Text("\(surface) Theme")
                .font(.caption2)
                .foregroundStyle(.clear)
                .frame(width: 1, height: 1)
                .opacity(0.001)
                .accessibilityIdentifier(accessibilityID ?? "\(surface.lowercased()).theme")
                .accessibilityValue(theme.rawValue)
        }
    }
}

private struct PrecisionToolbar: View {
    @EnvironmentObject private var model: AppModel
    @Environment(\.colorSchemeContrast) private var contrast
    @Binding var historyExpanded: Bool
    @FocusState private var searchFocused: Bool

    private var palette: SemanticThemePalette { model.theme.palette }

    var body: some View {
        HStack(spacing: 12) {
            Picker("Scope", selection: $model.scope) {
                ForEach(CatalogScope.allCases) { Text($0.rawValue).tag($0) }
            }
            .labelsHidden()
            .pickerStyle(.segmented)
            .frame(minWidth: 210, idealWidth: 260, maxWidth: 260)
            .accessibilityIdentifier("catalog.scope")

            Spacer(minLength: 0)
            HStack(spacing: 7) {
                Image(systemName: "magnifyingglass")
                    .foregroundStyle(palette.subtleText.color)
                TextField("Search Utilities", text: $model.search)
                    .textFieldStyle(.plain)
                    .focused($searchFocused)
                    .accessibilityIdentifier("catalog.search")
                Text("⌘F")
                    .font(.caption2.monospaced())
                    .foregroundStyle(palette.subtleText.color)
            }
            .padding(.horizontal, 9)
            .frame(minWidth: 240, idealWidth: 360, maxWidth: 480, minHeight: 32)
            .background(palette.editorSurface.color, in: RoundedRectangle(cornerRadius: 8))
            .overlay(
                RoundedRectangle(cornerRadius: 8)
                    .stroke(
                        palette.separator.color.opacity(contrast == .increased ? 0.82 : 0.38),
                        lineWidth: contrast == .increased ? 1.5 : 1)
            )
            Spacer(minLength: 0)

            HStack(spacing: 3) {
                Button {
                    historyExpanded.toggle()
                } label: {
                    Label("History", systemImage: "clock.arrow.circlepath")
                }
                .help("Toggle History")
                .accessibilityIdentifier("toolbar.history")

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
            .labelStyle(.iconOnly)
            .buttonStyle(.borderless)
            .font(.body)
            .fixedSize(horizontal: true, vertical: false)
        }
        .padding(.horizontal, 10)
        .padding(.vertical, 7)
        .frame(minHeight: 48)
        .background(palette.secondaryPane.color)
        .onReceive(NotificationCenter.default.publisher(for: .focusUtilitySearch)) { _ in
            searchFocused = true
        }
    }
}

private struct UtilityWorkspacePane: View {
    @EnvironmentObject private var model: AppModel
    let definition: UtilityDefinition

    private var palette: SemanticThemePalette { model.theme.palette }

    var body: some View {
        VStack(spacing: 0) {
            HStack(spacing: 12) {
                Image(systemName: definition.symbol)
                    .font(.system(size: 16, weight: .semibold))
                    .foregroundStyle(palette.secondaryAccent.color)
                    .frame(width: 34, height: 34)
                    .background(palette.raisedSurface.color, in: RoundedRectangle(cornerRadius: 9))
                    .overlay(
                        RoundedRectangle(cornerRadius: 9)
                            .stroke(palette.separator.color.opacity(0.42)))
                VStack(alignment: .leading, spacing: 2) {
                    Text(definition.category.rawValue.uppercased())
                        .font(.caption2.weight(.semibold))
                        .tracking(0.7)
                        .foregroundStyle(palette.subtleText.color)
                    Text(definition.name).font(.title3.weight(.semibold))
                    Text(definition.summary)
                        .font(.caption)
                        .foregroundStyle(palette.secondaryText.color)
                        .lineLimit(1)
                }
                Spacer()
                Button {
                    model.toggleFavorite(definition.id)
                } label: {
                    Label(
                        model.favorites.contains(definition.id) ? "Remove Favorite" : "Add Favorite",
                        systemImage: model.favorites.contains(definition.id) ? "star.fill" : "star")
                }
                .labelStyle(.iconOnly)
                .buttonStyle(.borderless)
                .foregroundStyle(
                    model.favorites.contains(definition.id)
                        ? palette.warning.color : palette.secondaryText.color
                )
                .help("Toggle Favorite")
            }
            .padding(.horizontal, 18)
            .frame(minHeight: 72)
            .background(palette.windowBackground.color)

            palette.separator.color.opacity(0.34).frame(height: 1)

            model.workspace(for: definition.id)
                .padding(16)
                .background(palette.windowBackground.color)
        }
    }
}

private struct UtilitySidebar: View {
    @EnvironmentObject private var model: AppModel
    @Environment(\.colorSchemeContrast) private var contrast

    private var palette: SemanticThemePalette { model.theme.palette }

    var body: some View {
        Group {
            if model.visibleDefinitions.isEmpty {
                ContentUnavailableView.search(text: model.search)
                    .overlay(alignment: .bottom) {
                        Button("Clear Search") { model.search = "" }.padding()
                    }
            } else {
                List(
                    selection: Binding(
                        get: { model.selectedUtilityID },
                        set: {
                            guard let id = $0, id != model.selectedUtilityID else { return }
                            Task { @MainActor in model.open(id) }
                        })
                ) {
                    ForEach(UtilityCategory.allCases, id: \.self) { category in
                        let definitions = model.visibleDefinitions.filter { $0.category == category }
                        if !definitions.isEmpty {
                            Section {
                                ForEach(definitions) { definition in
                                    UtilityCatalogRow(
                                        definition: definition,
                                        selected: model.selectedUtilityID == definition.id,
                                        favorite: model.favorites.contains(definition.id)
                                    )
                                    .tag(definition.id)
                                    .listRowBackground(Color.clear)
                                }
                            } header: {
                                Text(category.rawValue.uppercased())
                                    .font(.caption2.weight(.bold))
                                    .tracking(0.65)
                                    .foregroundStyle(palette.subtleText.color)
                            }
                        }
                    }
                }
                .listStyle(.sidebar)
                .scrollContentBackground(.hidden)
                .accessibilityIdentifier("catalog.list")
            }
        }
        .background(palette.secondaryPane.color)
        .overlay(alignment: .trailing) {
            palette.separator.color.opacity(contrast == .increased ? 0.72 : 0.34).frame(width: 1)
        }
    }
}

private struct UtilityCatalogRow: View {
    @EnvironmentObject private var model: AppModel
    @Environment(\.colorSchemeContrast) private var contrast
    let definition: UtilityDefinition
    let selected: Bool
    let favorite: Bool

    private var palette: SemanticThemePalette { model.theme.palette }

    var body: some View {
        HStack(spacing: 9) {
            Image(systemName: definition.symbol)
                .font(.callout.weight(.semibold))
                .foregroundStyle(selected ? palette.primaryText.color : palette.secondaryAccent.color)
                .frame(width: 28, height: 28)
                .background(palette.raisedSurface.color, in: RoundedRectangle(cornerRadius: 8))
            VStack(alignment: .leading, spacing: 2) {
                Text(definition.name)
                    .font(.callout.weight(.semibold))
                    .lineLimit(1)
                    .accessibilityIdentifier("catalog.utility.\(definition.id)")
                Text(definition.summary)
                    .font(.caption2)
                    .foregroundStyle(palette.secondaryText.color)
                    .lineLimit(1)
            }
            Spacer(minLength: 4)
            if favorite {
                Image(systemName: "star.fill")
                    .font(.caption2)
                    .foregroundStyle(palette.warning.color)
                    .accessibilityLabel("Favorite")
            }
        }
        .padding(.vertical, 3)
        .padding(.horizontal, 4)
        .background {
            if selected {
                RoundedRectangle(cornerRadius: 7)
                    .fill(palette.selection.color.opacity(contrast == .increased ? 0.3 : 0.18))
                    .overlay {
                        if contrast == .increased {
                            RoundedRectangle(cornerRadius: 7)
                                .stroke(palette.appAccent.color, lineWidth: 2)
                        }
                    }
            }
        }
        .contentShape(Rectangle())
    }
}
