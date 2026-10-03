import Foundation

/// What TeXpand needs from the project (TeXpandEditor.swift): which file is
/// being edited, which is the project's main file, where the project lives
/// (for `texpand.toml`), and the text of open documents (for the root
/// file's package index). A value, read per use on the main thread.
struct TeXpandProject {
    /// The edited document's path, as the model names it.
    var activePath: String
    /// The project's main (entry) document.
    var entryPath: String
    /// The project folder, when the project is saved on disk.
    var root: URL?
    /// An open document's text by path; nil when it is not open.
    var text: (String) -> String?
}

extension ShellModel {
    var texpandProject: TeXpandProject {
        TeXpandProject(activePath: activePath, entryPath: project.entryPath, root: project.projectRoot,
                       text: { [weak self] path in self?.documents.first { $0.path == path }?.text })
    }
}
