import SwiftUI

struct SymbolPalette: View {
    struct Item: Identifiable, Equatable {
        let command: String
        let glyph: String

        var id: String { command }
    }

    static let items: [Item] = [
        .init(command: "\\forall", glyph: "∀"),
        .init(command: "\\exists", glyph: "∃"),
        .init(command: "\\in", glyph: "∈"),
        .init(command: "\\notin", glyph: "∉"),
        .init(command: "\\subset", glyph: "⊂"),
        .init(command: "\\subseteq", glyph: "⊆"),
        .init(command: "\\cup", glyph: "∪"),
        .init(command: "\\cap", glyph: "∩"),
        .init(command: "\\setminus", glyph: "∖"),
        .init(command: "\\emptyset", glyph: "∅"),
        .init(command: "\\rightarrow", glyph: "→"),
        .init(command: "\\Rightarrow", glyph: "⇒"),
        .init(command: "\\leftrightarrow", glyph: "↔"),
        .init(command: "\\Leftrightarrow", glyph: "⇔"),
        .init(command: "\\leq", glyph: "≤"),
        .init(command: "\\geq", glyph: "≥"),
        .init(command: "\\neq", glyph: "≠"),
        .init(command: "\\approx", glyph: "≈"),
        .init(command: "\\equiv", glyph: "≡"),
        .init(command: "\\pm", glyph: "±"),
        .init(command: "\\infty", glyph: "∞"),
        .init(command: "\\int", glyph: "∫"),
        .init(command: "\\sum", glyph: "∑"),
        .init(command: "\\prod", glyph: "∏"),
        .init(command: "\\sqrt", glyph: "√"),
        .init(command: "\\partial", glyph: "∂"),
        .init(command: "\\nabla", glyph: "∇"),
        .init(command: "\\cdot", glyph: "⋅"),
        .init(command: "\\times", glyph: "×")
    ]

    let onInsert: (String) -> Void

    var body: some View {
        ScrollView {
            LazyVGrid(columns: Array(repeating: GridItem(.flexible(), spacing: 8), count: 6), spacing: 8) {
                ForEach(Self.items) { item in
                    Button { onInsert(item.command) } label: {
                        Text(item.glyph)
                            .font(.system(size: 22))
                            .frame(maxWidth: .infinity, minHeight: 36)
                    }
                    .help(item.command)
                    .accessibilityLabel(item.command)
                }
            }
            .padding(8)
        }
        .frame(minWidth: 280, minHeight: 180)
    }
}
