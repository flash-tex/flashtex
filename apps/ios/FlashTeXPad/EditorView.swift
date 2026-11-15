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
        Self.connect(controller, to: model)
        Self.show(document, in: controller, model: model)
        return controller.textView
    }

    func updateUIView(_ view: UITextView, context: Context) {
        let controller = context.coordinator.controller
        if model.editor !== controller { model.editor = controller }
        Self.show(document, in: controller, model: model)
    }

    /// The controller's edits, caret moves and commands go to `model`.
    static func connect(_ controller: EditorController, to model: PadModel) {
        controller.onChange = { [weak model, weak controller] text, caret, math in
            guard let model, let controller else { return }
            controller.loadedRevision = model.textChanged(text, caret: caret, mathMode: math, generation: controller.loadedGeneration)
        }
        controller.onCaret = { [weak model] caret, math in model?.caretMoved(caret, mathMode: math) }
        controller.onCommand = { [weak model] command in model?.handle(command) ?? false }
        controller.imagePasteHost = { [weak model] in model?.imagePasteHost() }
        model.editor = controller
    }

    /// Loads `document` into the controller unless it already shows it:
    /// the same buffer (`documentGeneration`, bumped by every open — each
    /// opened document starts at revision 1, so the revision alone could not
    /// tell two files apart) at the same revision.
    static func show(_ document: PadDocument, in controller: EditorController, model: PadModel) {
        if document.revision != controller.loadedRevision || model.documentGeneration != controller.loadedGeneration {
            controller.load(text: document.text, caret: model.caretUTF16, revision: document.revision)
            controller.loadedGeneration = model.documentGeneration
        }
    }

    static func dismantleUIView(_ view: UITextView, coordinator: Coordinator) {
        coordinator.controller.onChange = nil
        coordinator.controller.onCaret = nil
        coordinator.controller.onCommand = nil
        coordinator.controller.imagePasteHost = nil
    }

    func makeCoordinator() -> Coordinator { Coordinator() }

    @MainActor
    final class Coordinator {
        let controller = EditorController()
    }
}
