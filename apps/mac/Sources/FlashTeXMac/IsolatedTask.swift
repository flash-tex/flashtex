import SwiftUI

/// `.task(id:)` whose id is read in this zero-size view's own body, not in
/// the body of the view it is attached to (P5-KEYSTROKE-MAIN).
///
/// `@Observable` tracking is per body: a parent that reads `model.caretUTF16`
/// in a `.task(id:)` re-evaluates its whole body, and re-lays out its hosting
/// view, on every keystroke, although only the task wanted the value. Attached
/// with `.background(IsolatedTask(...))`, the read re-evaluates this view
/// only; the task starts, restarts and is cancelled exactly as the parent's
/// `.task(id:)` would (same id, same lifetime as the parent on screen).
struct IsolatedTask<ID: Equatable>: View {
    /// Evaluated in this body, so only this view tracks what it reads.
    let id: () -> ID
    let action: (ID) async -> Void

    init(id: @escaping () -> ID, _ action: @escaping (ID) async -> Void) {
        self.id = id
        self.action = action
    }

    var body: some View {
        let value = id()
        Color.clear
            .frame(width: 0, height: 0)
            .accessibilityHidden(true)
            .task(id: value) { await action(value) }
    }
}

/// `.onChange(of:)` whose value is read in this zero-size view's own body
/// (see `IsolatedTask`): only this view re-evaluates when the value changes.
struct IsolatedOnChange<Value: Equatable>: View {
    let value: () -> Value
    let action: (Value) -> Void

    init(of value: @escaping () -> Value, _ action: @escaping (Value) -> Void) {
        self.value = value
        self.action = action
    }

    var body: some View {
        Color.clear
            .frame(width: 0, height: 0)
            .accessibilityHidden(true)
            .onChange(of: value()) { _, new in action(new) }
    }
}
