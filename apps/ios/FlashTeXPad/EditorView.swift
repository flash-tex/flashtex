import FlashTeXPadKit
import SwiftUI
import UIKit

/// The SwiftUI face of `EditorController`. The model owns the document; the
/// controller owns the text view and reports every change back with the
/// caret and the lexer's mode there. A model revision the controller did
/// not produce (a file opened, a review applied) reloads the text; the
/// controller's own edits never round-trip through `text` — the model
/// records the revision it assigned them, so undo, scroll position and the
/// pending-closer list survive each keystroke.
struct EditorView: UIViewRepresentable {
    @EnvironmentObject var model: PadModel
    let document: PadDocument

    func makeUIView(context: Context) -> UITextView {
        let controller = context.coordinator.controller
        let model = self.model
        controller.onChange = { [weak model, weak controller] text, caret, math in
            guard let model, let controller else { return }
            controller.loadedRevision = model.textChanged(text, caret: caret, mathMode: math)
        }
        controller.onCaret = { [weak model] caret, math in model?.caretMoved(caret, mathMode: math) }
        controller.onCommand = { [weak model] command in model?.handle(command) ?? false }
        model.editor = controller
        controller.load(text: document.text, caret: model.caretUTF16, revision: document.revision)
        return controller.textView
    }

    func updateUIView(_ view: UITextView, context: Context) {
        let controller = context.coordinator.controller
        if model.editor !== controller { model.editor = controller }
        if document.revision != controller.loadedRevision {
            controller.load(text: document.text, caret: model.caretUTF16, revision: document.revision)
        }
    }

    static func dismantleUIView(_ view: UITextView, coordinator: Coordinator) {
        coordinator.controller.onChange = nil
        coordinator.controller.onCaret = nil
        coordinator.controller.onCommand = nil
    }

    func makeCoordinator() -> Coordinator { Coordinator() }

    @MainActor
    final class Coordinator {
        let controller = EditorController()
    }
}
