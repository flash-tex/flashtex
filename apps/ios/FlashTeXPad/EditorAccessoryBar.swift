import UIKit

/// The row above the on-screen keyboard: the LaTeX characters the software
/// keyboard hides behind a shift (`\`, braces, `$`, `^`, `_`), the two
/// commands typed most, `\begin` and `\item`, Tab, and undo/redo. Every
/// insertion goes through the controller's keystroke path, so `{` closes
/// itself exactly as a typed one does. Hidden while a hardware keyboard is
/// attached (`EditorController.updateAccessoryVisibility`).
final class EditorAccessoryBar: UIInputView {
    enum Item: String, CaseIterable {
        case backslash, braces, brackets, dollar, caret, underscore, frac, sqrt, begin, item, tab, undo, redo

        var label: String {
            switch self {
            case .backslash: return "\\"
            case .braces: return "{ }"
            case .brackets: return "[ ]"
            case .dollar: return "$"
            case .caret: return "^"
            case .underscore: return "_"
            case .frac: return "\\frac"
            case .sqrt: return "\\sqrt"
            case .begin: return "\\begin"
            case .item: return "\\item"
            case .tab: return "⇥"
            case .undo: return "↶"
            case .redo: return "↷"
            }
        }

        var accessibilityLabel: String {
            switch self {
            case .tab: return "Tab"
            case .undo: return "Undo"
            case .redo: return "Redo"
            default: return label
            }
        }
    }

    var onItem: ((Item) -> Void)?
    private(set) var buttons: [Item: UIButton] = [:]

    init() {
        super.init(frame: CGRect(x: 0, y: 0, width: 0, height: 48), inputViewStyle: .keyboard)
        allowsSelfSizing = true
        let scroll = UIScrollView()
        scroll.showsHorizontalScrollIndicator = false
        scroll.translatesAutoresizingMaskIntoConstraints = false
        addSubview(scroll)
        let stack = UIStackView()
        stack.axis = .horizontal
        stack.spacing = 8
        stack.translatesAutoresizingMaskIntoConstraints = false
        scroll.addSubview(stack)
        for item in Item.allCases {
            var config = UIButton.Configuration.gray()
            config.attributedTitle = AttributedString(item.label, attributes: AttributeContainer([.font: UIFont.monospacedSystemFont(ofSize: 16, weight: .medium)]))
            config.contentInsets = NSDirectionalEdgeInsets(top: 6, leading: 12, bottom: 6, trailing: 12)
            let button = UIButton(configuration: config, primaryAction: UIAction { [weak self] _ in self?.onItem?(item) })
            button.accessibilityLabel = item.accessibilityLabel
            button.accessibilityIdentifier = "accessory.\(item.rawValue)"
            buttons[item] = button
            stack.addArrangedSubview(button)
        }
        NSLayoutConstraint.activate([
            scroll.leadingAnchor.constraint(equalTo: leadingAnchor),
            scroll.trailingAnchor.constraint(equalTo: trailingAnchor),
            scroll.topAnchor.constraint(equalTo: topAnchor),
            scroll.bottomAnchor.constraint(equalTo: bottomAnchor),
            heightAnchor.constraint(equalToConstant: 48),
            stack.leadingAnchor.constraint(equalTo: scroll.contentLayoutGuide.leadingAnchor, constant: 12),
            stack.trailingAnchor.constraint(equalTo: scroll.contentLayoutGuide.trailingAnchor, constant: -12),
            stack.topAnchor.constraint(equalTo: scroll.contentLayoutGuide.topAnchor, constant: 6),
            stack.bottomAnchor.constraint(equalTo: scroll.contentLayoutGuide.bottomAnchor, constant: -6),
            stack.heightAnchor.constraint(equalTo: scroll.frameLayoutGuide.heightAnchor, constant: -12),
        ])
    }

    required init?(coder: NSCoder) { fatalError("init(coder:) is not supported") }

    /// Taps a button (tests).
    func tap(_ item: Item) { onItem?(item) }
}
