import PierreDiffsSwift
import PierreDiffSpikeSupport
import SwiftUI

// THROWAWAY PROTOTYPE: validates PierreDiffsSwift as a renderer; not production UI.
@main
struct PierreDiffSpikeApp: App {
    var body: some Scene {
        WindowGroup("Pierre diff validation") {
            ValidationWorkbench()
                .frame(minWidth: 960, minHeight: 700)
                .preferredColorScheme(.dark)
        }
    }
}

private struct ValidationWorkbench: View {
    @State private var oldText: String
    @State private var newText: String
    @State private var diffStyle: DiffStyle
    @State private var overflowMode = OverflowMode.scroll
    @State private var ignoresWhitespace = false
    @State private var ignoresCase = false
    @State private var renderStartedAt = ContinuousClock.now
    @State private var renderDuration: Duration?

    init() {
        let arguments = ProcessInfo.processInfo.arguments
        let initialSample = if arguments.contains("--stress") {
            Samples.large
        } else if arguments.contains("--unicode") {
            Samples.unicode
        } else {
            Samples.small
        }
        _oldText = State(initialValue: initialSample.old)
        _newText = State(initialValue: initialSample.new)
        _diffStyle = State(initialValue: arguments.contains("--unified") ? .unified : .split)
    }

    private var processed: (old: String, new: String) {
        TextDiffPreprocessor.process(
            old: oldText,
            new: newText,
            ignoresWhitespace: ignoresWhitespace,
            ignoresCase: ignoresCase
        )
    }

    private var usesEmojiSafeHighlighting: Bool {
        TextDiffRenderPolicy.requiresWholeLineHighlighting(old: processed.old, new: processed.new)
    }

    var body: some View {
        VStack(spacing: 0) {
            controls
            Divider()
            inputs
            Divider()
            renderer
            Divider()
            status
        }
        .padding(12)
        .onChange(of: oldText) { restartTimer() }
        .onChange(of: newText) { restartTimer() }
        .onChange(of: diffStyle) { restartTimer() }
        .onChange(of: ignoresWhitespace) { restartTimer() }
        .onChange(of: ignoresCase) { restartTimer() }
    }

    private var controls: some View {
        HStack(spacing: 16) {
            Picker("Layout", selection: $diffStyle) {
                Text("Split").tag(DiffStyle.split)
                Text("Unified").tag(DiffStyle.unified)
            }
            .pickerStyle(.segmented)
            .frame(width: 220)

            Toggle("Ignore whitespace", isOn: $ignoresWhitespace)
            Toggle("Ignore case", isOn: $ignoresCase)

            Spacer()

            Menu("Load sample") {
                Button("Small Swift") { load(.small) }
                Button("Unicode and whitespace") { load(.unicode) }
                Button("10,000 lines") { load(.large) }
            }
            .keyboardShortcut("l", modifiers: [.command])
        }
        .padding(.bottom, 10)
    }

    private var inputs: some View {
        HSplitView {
            editor(title: "Original", text: $oldText)
            editor(title: "Updated", text: $newText)
        }
        .frame(minHeight: 190, idealHeight: 230, maxHeight: 300)
    }

    private func editor(title: String, text: Binding<String>) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(title)
                .font(.headline)
            TextEditor(text: text)
                .font(.system(.body, design: .monospaced))
                .accessibilityLabel("\(title) text")
        }
        .padding(8)
    }

    private var renderer: some View {
        PierreDiffView(
            oldContent: processed.old,
            newContent: processed.new,
            fileName: "Example.swift",
            diffStyle: $diffStyle,
            overflowMode: $overflowMode,
            renderOptions: PierreDiffRenderOptions(
                diffIndicators: .bars,
                hunkSeparators: .lineInfo,
                lineDiffType: TextDiffRenderPolicy.lineDiffType(old: processed.old, new: processed.new),
                tokenizeMaxLength: 500_000,
                tokenizeMaxLineLength: 10_000
            ),
            onReady: {
                renderDuration = renderStartedAt.duration(to: .now)
                print("PIERRE_READY \(renderDuration?.formatted(.units(allowed: [.seconds, .milliseconds], width: .abbreviated)) ?? "unknown") old=\(oldText.count) new=\(newText.count) style=\(diffStyle.rawValue)")
            }
        )
        .accessibilityLabel("Rendered text difference")
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private var status: some View {
        HStack {
            Text("Pinned PierreDiffsSwift 1.2.4")
            if usesEmojiSafeHighlighting {
                Text("Emoji-safe whole-line highlighting")
                    .help("Pierre intraline spans break multi-scalar emoji; whole-line highlighting preserves each emoji grapheme.")
            }
            Spacer()
            Text("\(oldText.count.formatted()) → \(newText.count.formatted()) characters")
            if let renderDuration {
                Text("Ready in \(renderDuration.formatted(.units(allowed: [.seconds, .milliseconds], width: .abbreviated)))")
            } else {
                ProgressView().controlSize(.small)
                Text("Rendering")
            }
        }
        .font(.caption.monospacedDigit())
        .foregroundStyle(.secondary)
        .padding(.top, 8)
    }

    private func load(_ sample: Samples) {
        oldText = sample.old
        newText = sample.new
        restartTimer()
    }

    private func restartTimer() {
        renderStartedAt = .now
        renderDuration = nil
    }
}

private enum TextDiffPreprocessor {
    static func process(
        old: String,
        new: String,
        ignoresWhitespace: Bool,
        ignoresCase: Bool
    ) -> (old: String, new: String) {
        (
            normalize(old, ignoresWhitespace: ignoresWhitespace, ignoresCase: ignoresCase),
            normalize(new, ignoresWhitespace: ignoresWhitespace, ignoresCase: ignoresCase)
        )
    }

    private static func normalize(
        _ value: String,
        ignoresWhitespace: Bool,
        ignoresCase: Bool
    ) -> String {
        var result = value

        if ignoresCase {
            result = result.folding(options: [.caseInsensitive], locale: Locale(identifier: "en_US_POSIX"))
        }

        if ignoresWhitespace {
            result = result
                .split(separator: "\n", omittingEmptySubsequences: false)
                .map { line in line.filter { !$0.isWhitespace } }
                .joined(separator: "\n")
        }

        return result
    }
}

private enum Samples {
    case small
    case unicode
    case large

    var old: String {
        switch self {
        case .small:
            return """
            struct Greeting {
                let message = "Hello"
                let count = 2
            }
            """
        case .unicode:
            return """
            let café = "Straße"
            let emoji = "👩🏽‍💻"
            let spaced = "alpha beta"
            """
        case .large:
            return (1...10_000).map { "let value\($0) = \($0)" }.joined(separator: "\n")
        }
    }

    var new: String {
        switch self {
        case .small:
            return """
            struct Greeting {
                let message = "Hello, macOS"
                let count = 3
                let enabled = true
            }
            """
        case .unicode:
            return """
            let CAFÉ = "STRASSE"
            let emoji = "👩🏽‍🚀"
            let spaced="alphabeta"
            """
        case .large:
            return (1...10_000).map { index in
                index.isMultiple(of: 100) ? "let value\(index) = \(index + 1) // changed" : "let value\(index) = \(index)"
            }.joined(separator: "\n")
        }
    }
}
