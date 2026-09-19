import Foundation

/// What Return does inside an environment (Settings > Editor > Environments).
///
/// Two questions, answered per environment name with a fallback for the rest:
///
/// 1. **Indent** — is the body one indent unit deeper than `\begin{…}`? The
///    conventional answer is yes for everything except `document` (nobody
///    indents a whole document body) and the verbatim-like environments,
///    whose bodies are copied byte-for-byte.
/// 2. **New line** — what every new body line starts with: `\item ` in
///    `itemize`/`enumerate`, `\item[] ` in `description`, `\bibitem{} ` in
///    `thebibliography`, nothing elsewhere.
///
/// Both feed the Return key (`EditorIntelligence.newline`), the environment
/// completion skeletons (`Completion.environmentSnippet`), Wrap in
/// Environment (`EditorNavigation.wrap`) and Re-indent
/// (`EditorIndentation`), so one setting changes them all together.
///
/// The `conventional` rules are the shipped defaults; the user's rules are
/// stored as JSON in `EditorPreferences` (`environmentRules`). A starred
/// environment falls back to its unstarred rule (`align*` → `align`), so a
/// rule for `enumerate` covers `enumerate*` too.
struct EnvironmentEditingRules: Equatable, Codable, Sendable {
    struct Rule: Equatable, Codable, Identifiable, Sendable {
        /// The environment name as written in `\begin{…}`, without the star.
        var environment: String
        /// Whether the body sits one indent unit in from `\begin`.
        var indent: Bool
        /// Text every new body line starts with (after the indentation);
        /// empty for none. The caret lands inside the first empty `[]` or
        /// `{}` it contains, else after it.
        var newLine: String

        var id: String { environment }

        init(environment: String, indent: Bool = true, newLine: String = "") {
            self.environment = environment
            self.indent = indent
            self.newLine = newLine
        }
    }

    /// Whether the body of an environment with no rule of its own is
    /// indented one unit.
    var indentByDefault: Bool
    var rules: [Rule]

    init(indentByDefault: Bool = true, rules: [Rule]) {
        self.indentByDefault = indentByDefault
        self.rules = rules
    }

    /// The opinionated defaults: indent everything except `document`, and
    /// start list entries with their `\item`.
    static let conventional = EnvironmentEditingRules(indentByDefault: true, rules: [
        Rule(environment: "document", indent: false),
        Rule(environment: "itemize", newLine: "\\item "),
        Rule(environment: "enumerate", newLine: "\\item "),
        Rule(environment: "description", newLine: "\\item[] "),
        Rule(environment: "thebibliography", newLine: "\\bibitem{} "),
    ])

    /// Environments whose body is never indented and never gets a line
    /// template, whatever the rules say: their content is literal.
    static var preservedBodyEnvironments: Set<String> { EditorIndentation.preservedBodyEnvironments }

    /// The rule for `environment`: an exact match, else the unstarred
    /// name's (`align*` → `align`).
    func rule(for environment: String) -> Rule? {
        let name = environment.trimmingCharacters(in: .whitespaces)
        if let exact = rules.first(where: { $0.environment == name }) { return exact }
        guard name.hasSuffix("*") else { return nil }
        let base = String(name.dropLast())
        return rules.first(where: { $0.environment == base })
    }

    /// Whether Return after `\begin{environment}` (and every body line the
    /// editor writes) goes one indent unit deeper.
    func indentsBody(of environment: String) -> Bool {
        if Self.preservedBodyEnvironments.contains(environment) { return false }
        return rule(for: environment)?.indent ?? indentByDefault
    }

    /// What a new body line of `environment` starts with (`""` for nothing).
    func newLineText(in environment: String) -> String {
        if Self.preservedBodyEnvironments.contains(environment) { return "" }
        return rule(for: environment)?.newLine ?? ""
    }

    /// The command a line template opens with (`\item` for `\item[] `), or
    /// nil for a template that is not a command. Return on a line that
    /// starts with this command and has content after it repeats the
    /// template — the "next entry" rule of lists.
    static func command(of template: String) -> String? {
        guard template.hasPrefix("\\") else { return nil }
        let letters = template.dropFirst().prefix { $0.isLetter }
        return letters.isEmpty ? nil : "\\" + letters
    }

    /// UTF-16 offset of the caret inside `template`: after the `[` or `{`
    /// of its first empty group, else the end.
    static func caretOffset(in template: String) -> Int {
        let ns = template as NSString
        var i = 0
        while i + 1 < ns.length {
            let c = ns.character(at: i), next = ns.character(at: i + 1)
            if (c == 0x5B && next == 0x5D) || (c == 0x7B && next == 0x7D) { return i + 1 } // `[]`, `{}`
            i += 1
        }
        return ns.length
    }

    /// Environment names that are not blank, each trimmed and listed once
    /// (the first rule for a name wins), which is how the settings table
    /// hands its rows back.
    func normalized() -> EnvironmentEditingRules {
        var seen = Set<String>()
        let kept = rules.compactMap { rule -> Rule? in
            let name = rule.environment.trimmingCharacters(in: .whitespaces)
            guard !name.isEmpty, seen.insert(name).inserted else { return nil }
            return Rule(environment: name, indent: rule.indent, newLine: rule.newLine)
        }
        return EnvironmentEditingRules(indentByDefault: indentByDefault, rules: kept)
    }

    // MARK: persistence (JSON in UserDefaults)

    func encoded() -> Data? { try? JSONEncoder().encode(self) }

    static func decoded(_ data: Data) -> EnvironmentEditingRules? {
        (try? JSONDecoder().decode(EnvironmentEditingRules.self, from: data))?.normalized()
    }
}
