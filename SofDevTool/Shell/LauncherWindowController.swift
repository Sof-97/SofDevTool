import AppKit
import SwiftUI

@MainActor
final class LauncherWindowController: NSWindowController, NSWindowDelegate {
    private let model: AppModel

    init(model: AppModel) {
        self.model = model
        let panel = NSPanel(
            contentRect: NSRect(x: 0, y: 0, width: 580, height: 430),
            styleMask: [.nonactivatingPanel, .titled, .fullSizeContentView],
            backing: .buffered,
            defer: false
        )
        panel.titleVisibility = .hidden
        panel.titlebarAppearsTransparent = true
        panel.isFloatingPanel = true
        panel.level = .popUpMenu
        panel.collectionBehavior = [.canJoinAllSpaces, .fullScreenAuxiliary, .transient]
        // A global shortcut commonly fires while another application remains active. Let the
        // nonactivating panel stay visible in that state; windowDidResignKey still provides the
        // click-away dismissal behavior.
        panel.hidesOnDeactivate = false
        panel.isMovableByWindowBackground = true
        panel.backgroundColor = .clear
        panel.contentView = NSHostingView(rootView: LauncherView(close: {}).environmentObject(model))
        super.init(window: panel)
        panel.delegate = self
        panel.contentView = NSHostingView(
            rootView: LauncherView { [weak panel] in panel?.orderOut(nil) }.environmentObject(model))
    }

    required init?(coder: NSCoder) { fatalError("init(coder:) has not been implemented") }

    func toggle() {
        guard let window else { return }
        if window.isVisible {
            window.orderOut(nil)
            return
        }
        let screen =
            NSScreen.screens.first(where: { $0.frame.contains(NSEvent.mouseLocation) }) ?? NSScreen.main
        if let frame = screen?.visibleFrame {
            window.setFrameOrigin(
                NSPoint(x: frame.midX - window.frame.width / 2, y: frame.midY - window.frame.height / 2))
        }
        window.makeKeyAndOrderFront(nil)
    }

    func windowDidResignKey(_ notification: Notification) { window?.orderOut(nil) }
}

private struct LauncherView: View {
    @EnvironmentObject private var model: AppModel
    @State private var query = ""
    @State private var selection: String?
    @FocusState private var searchFocused: Bool
    let close: () -> Void

    private var results: [UtilityDefinition] { model.registry.search(query) }

    var body: some View {
        VStack(spacing: 0) {
            HStack {
                Image(systemName: "command")
                TextField("Find a Utility", text: $query)
                    .textFieldStyle(.plain)
                    .font(.title2)
                    .focused($searchFocused)
                    .accessibilityIdentifier("launcher.search")
                Text("esc").font(.caption.monospaced()).foregroundStyle(.secondary)
            }
            .padding(18)
            Divider()
            List(results, selection: $selection) { definition in
                Label(definition.name, systemImage: definition.symbol)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .contentShape(Rectangle())
                    .tag(definition.id)
                    .accessibilityIdentifier("launcher.utility.\(definition.id)")
                    .onTapGesture { open(definition.id) }
            }
            .listStyle(.inset)
            .onChange(of: results.map(\.id)) { _, ids in
                if !ids.contains(selection ?? "") { selection = ids.first }
            }
        }
        .background(.ultraThinMaterial)
        .preferredColorScheme(.dark)
        .onAppear {
            selection = results.first?.id
            searchFocused = true
        }
        .onSubmit { openSelection() }
        .onExitCommand { close() }
        .onKeyPress(.downArrow) {
            move(1)
            return .handled
        }
        .onKeyPress(.upArrow) {
            move(-1)
            return .handled
        }
    }

    private func move(_ delta: Int) {
        guard !results.isEmpty else { return }
        let current = results.firstIndex { $0.id == selection } ?? (delta > 0 ? -1 : 0)
        selection = results[(current + delta + results.count) % results.count].id
    }

    private func openSelection() {
        guard let selection else { return }
        open(selection)
    }

    private func open(_ id: String) {
        model.open(id)
        close()
        NSApp.activate(ignoringOtherApps: true)
        NSApp.windows.first(where: { !($0 is NSPanel) })?.makeKeyAndOrderFront(nil)
    }
}
