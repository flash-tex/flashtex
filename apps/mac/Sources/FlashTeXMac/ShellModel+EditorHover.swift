import Foundation

/// What the editor's hover resolves cross-references against
/// (EditorHoverResolution.swift): the project's other open documents, and a
/// file probe rooted at the project directory.
extension ShellModel {
    /// Built once per hover (never per keystroke). The documents are the open
    /// ones other than the active buffer — the hover already has that text —
    /// in project order, so a `\cite` key resolves from the first `.bib` that
    /// defines it. The file probe refuses anything that resolves outside the
    /// project root or through a symlink (`ProjectDocuments.rootedFile`), so a
    /// crafted `\includegraphics{../../etc/passwd}` cannot be used to test for
    /// files outside the project.
    func editorHoverContext() -> EditorIntelligence.HoverContext {
        let others = documents.filter { $0.path != activePath }
            .map { EditorIntelligence.HoverContext.Document(path: $0.path, text: $0.text) }
        guard let root = project.projectRoot else {
            return EditorIntelligence.HoverContext(otherDocuments: others, canProbeFiles: false)
        }
        return EditorIntelligence.HoverContext(otherDocuments: others, canProbeFiles: true) { path in
            guard case .file(let url) = ProjectDocuments.rootedFile(path, under: root) else { return false }
            var isDirectory: ObjCBool = false
            return FileManager.default.fileExists(atPath: url.path, isDirectory: &isDirectory) && !isDirectory.boolValue
        }
    }
}
