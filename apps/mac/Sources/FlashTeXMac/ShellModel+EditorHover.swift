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

    /// What `\cite{` completion reads `.bib` records from without the helper
    /// (BibScanner.swift): the saved project's root (declarations such as
    /// `\bibliography{refs}` resolve under it), the paths the helper reports
    /// as declared bibliographies, and every open document — the active one
    /// first — so an open `.bib` is parsed from its buffer and the LaTeX
    /// sources are scanned for their declarations. Read once per list
    /// request (the scan itself runs off-main).
    func bibliographySources() -> BibScanner.Sources {
        var docs = [BibScanner.Sources.Document(path: activePath, text: activeText)]
        docs += documents.filter { $0.path != activePath }.map { BibScanner.Sources.Document(path: $0.path, text: $0.text) }
        return BibScanner.Sources(projectRoot: project.projectRoot, declaredPaths: documentKinds.bibliographyPaths, documents: docs)
    }

    /// How the active buffer is coloured: BibTeX when the helper reports it
    /// as a declared bibliography (`DocumentKinds.kind(of:)`), LaTeX when it
    /// reports LaTeX. With no kind reported (no helper attached, or a path
    /// the snapshot does not list) a `.bib` extension decides — colouring is
    /// a reading aid, not a claim about the project's declared sources.
    var editorLanguage: SyntaxHighlighter.Language {
        switch documentKinds.kind(of: activePath) {
        case .bibliography: return .bibtex
        case .latex: return .latex
        case nil: return BibScanner.isBibliographyPath(activePath) ? .bibtex : .latex
        }
    }
}
