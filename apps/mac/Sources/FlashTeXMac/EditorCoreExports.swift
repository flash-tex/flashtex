// The pure editor pieces shared with the iPad app (the syntax token model,
// environment editing rules, the Return key, the delimiter matcher) live in
// the FlashTeXEditorCore target. Re-exported so every file of the app sees
// them under their old names without an import of its own.
@_exported import FlashTeXEditorCore
