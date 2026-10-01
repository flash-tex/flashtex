import XCTest
@testable import FlashTeXEditorCore

/// PLAN M2 registry: TOML loading, layering, `disable`, packs, settings,
/// profiles, diagnostics and load-time lint (§13, §16).
final class RegistryTests: XCTestCase {
    typealias T = TeXpand

    func testBuiltInsLoadWithoutProblems() {
        let engine = T.Engine()
        let problems = engine.diagnostics.filter { $0.severity != .note }
        XCTAssertEqual(problems, [], problems.map(\.description).joined(separator: "\n"))
        XCTAssertEqual(engine.registry.packs.map(\.name),
                       ["preamble", "sections", "lists", "floats", "theorems", "tables", "display-math", "algorithms", "listings", "beamer", "references"])
    }

    // MARK: TOML subset

    func testTOMLDocument() throws {
        let t = try T.parseTOML(#"""
        # comment
        title = "a \"quoted\" \\ string"   # trailing comment
        literal = 'C:\no\escapes'
        n = -1_000
        f = 0.5
        yes = true
        list = [1, 2,
          3,   # comment inside
        ]
        inline = { a = 1, b.c = "x" }
        multi = '''
        line 1
          line 2'''
        basic = """
        a\
           b"""

        [table.sub]
        k = "v"

        [[abbr]]
        name = "one"
          [[abbr.variant]]
          when = { package = "physics" }

        [[abbr]]
        name = "two"
        """#)
        XCTAssertEqual(t["title"], .string("a \"quoted\" \\ string"))
        XCTAssertEqual(t["literal"], .string("C:\\no\\escapes"))
        XCTAssertEqual(t["n"], .integer(-1000))
        XCTAssertEqual(t["f"], .float(0.5))
        XCTAssertEqual(t["yes"], .bool(true))
        XCTAssertEqual(t["list"], .array([.integer(1), .integer(2), .integer(3)]))
        XCTAssertEqual(t["inline"]?.table?["b"]?.table?["c"], .string("x"))
        XCTAssertEqual(t["multi"], .string("line 1\n  line 2"))
        XCTAssertEqual(t["basic"], .string("ab"))
        XCTAssertEqual(t["table"]?.table?["sub"]?.table?["k"], .string("v"))
        let abbrs = t["abbr"]?.array?.compactMap(\.table) ?? []
        XCTAssertEqual(abbrs.map { $0["name"]?.string }, ["one", "two"])
        XCTAssertEqual(abbrs[0]["variant"]?.array?.count, 1)
        XCTAssertEqual(abbrs[0].line, 21)
    }

    func testTOMLErrorsCarryLines() {
        let rows: [(String, Int)] = [
            ("a = 1\nb = \n", 2), ("a = \"x\n", 1), ("a = 1\na = 2", 2), ("d = 2026-09-30", 1),
            ("a = [1, 2", 1), ("[t]\nx = 'a' b", 2), ("s = \"\\q\"", 1),
        ]
        for (text, line) in rows {
            XCTAssertThrowsError(try T.parseTOML(text), text) { e in
                XCTAssertEqual((e as? T.TOMLError)?.line, line, "\(text.debugDescription): \(e)")
            }
        }
    }

    // MARK: layering

    func testProjectLayerOverridesABuiltIn() {
        let project = T.Layer(name: "project", source: #"""
        [[abbr]]
        name = "sec"
        body = '\section{<<arg.1>>} % mine'
        """#)
        let e = T.Engine(layers: [project])
        XCTAssertEqual(e.expandToString("sec{A}"), "\\section{A} % mine")
        XCTAssertEqual(e.registry.resolve("sec", flags: ["text"])?.layer, "project")
    }

    func testDisableRemovesLowerLayers() {
        let user = T.Layer(name: "user", source: "disable = [\"sec\", \"fig\"]\n")
        let e = T.Engine(layers: [user])
        XCTAssertEqual(e.expandToString("sec"), "error: unknown abbreviation `sec`")
        XCTAssertEqual(e.expandToString("fig"), "error: unknown abbreviation `fig`")
        XCTAssertNotEqual(e.expandToString("sub"), "error: unknown abbreviation `sub`")
    }

    func testDisableThenRedefineInTheSameLayer() {
        let user = T.Layer(name: "user", source: #"""
        disable = ["enum"]
        [[abbr]]
        name = "enum"
        body = 'mine'
        """#)
        XCTAssertEqual(T.Engine(layers: [user]).expandToString("enum"), "mine")
    }

    func testSettingsDisableDefinitionsAndPacks() {
        var s = T.Settings()
        s.disabled = ["thm"]
        s.disabledPacks = ["beamer", "tables"]
        let e = T.Engine(settings: s)
        XCTAssertEqual(e.expandToString("thm"), "error: unknown abbreviation `thm`")
        XCTAssertEqual(e.expandToString("tab"), "error: unknown abbreviation `tab`")
        XCTAssertEqual(e.expandToString("frame"), "error: unknown abbreviation `frame`")
        XCTAssertNotNil(e.registry.resolve("lem", flags: ["text"]))
    }

    func testOptInPacksNeedPacksSetting() {
        let pack = T.Layer(name: "pack:ml", source: #"""
        [pack]
        name = "ml"
        opt_in = true
        [[abbr]]
        name = "loss"
        body = '\mathcal{L}'
        """#)
        XCTAssertNil(T.Engine(layers: [pack]).registry.resolve("loss", flags: ["text"]))
        var s = T.Settings()
        s.enabledPacks = ["ml"]
        XCTAssertEqual(T.Engine(settings: s, layers: [pack]).expandToString("loss"), "\\mathcal{L}")
    }

    func testConflictingPacks() {
        let a = T.Layer(name: "a", source: "[pack]\nname = \"a\"\n[[abbr]]\nname = \"x\"\nbody = 'a'\n")
        let b = T.Layer(name: "b", source: "[pack]\nname = \"b\"\nconflicts = [\"a\"]\n[[abbr]]\nname = \"x\"\nbody = 'b'\n")
        let e = T.Engine(layers: [a, b])
        XCTAssertEqual(e.expandToString("x"), "a")
        XCTAssertTrue(e.diagnostics.contains { $0.layer == "b" && $0.message.contains("conflicts") })
    }

    // MARK: diagnostics: a bad definition is skipped, nothing else breaks

    func testMalformedDefinitionIsDiagnosedAndSkipped() {
        let project = T.Layer(name: "project", source: #"""
        [[abbr]]
        name = "good"
        body = 'ok'

        [[abbr]]
        name = "bad"
        body = '<<p.nope.value>>'

        [[abbr]]
        name = "worse"
        body = '<<mystery>>'

        [[abbr]]
        name = "has2digits"
        body = 'x'

        [[abbr]]
        name = "nobody"
        """#)
        let e = T.Engine(layers: [project])
        XCTAssertEqual(e.expandToString("good"), "ok")
        XCTAssertEqual(e.expandToString("sec{A}"), "\\section{A}")
        let errors = e.diagnostics.filter { $0.severity == .error && $0.layer == "project" }
        XCTAssertEqual(errors.map(\.line), [5, 9, 13, 17], errors.map(\.description).joined(separator: "\n"))
        XCTAssertTrue(errors[0].message.contains("undeclared param"))
        XCTAssertTrue(errors[1].message.contains("unknown hole"))
        XCTAssertTrue(errors[2].message.contains("letters only"))
        XCTAssertTrue(errors[3].message.contains("needs a `body` or a `generator`"))
        XCTAssertEqual(e.expandToString("bad"), "error: unknown abbreviation `bad`")
    }

    func testUnparseableLayerIsOneDiagnostic() {
        let e = T.Engine(layers: [T.Layer(name: "user", source: "[[abbr]\nname = 'x'")])
        XCTAssertEqual(e.diagnostics.filter { $0.layer == "user" }.map(\.line), [1])
        XCTAssertEqual(e.expandToString("sec"), "\\section{$1}", "the built-ins still load")
    }

    func testLintRules() {
        let project = T.Layer(name: "project", source: #"""
        [[abbr]]
        name = "a"
        scope = ["math"]
        instant = true
        body = '\alpha'

        [[abbr]]
        name = "a"
        scope = ["math", "text"]
        body = 'not an atom'

        [[abbr]]
        name = "b"
        scope = ["math"]
        instant = true
        params = [{ name = "x" }]
        body = '<<p.x.value>>'

        [[abbr]]
        name = "c"
        body = '<<profile.no_such_key>>'
        unknown_key = 1

        [[abbr]]
        name = "d"
        params = [{ name = "r", type = "range" }]
        body = '<<p.r.num>>'

        [[abbr]]
        name = "e"
        params = [{ name = "r", type = "range", default = "oops" }]
        body = '<<p.r.lo>>'
        """#)
        let d = T.Engine(layers: [project]).diagnostics.filter { $0.layer == "project" }
        func has(_ sev: T.Diagnostic.Severity, _ name: String, _ text: String) -> Bool {
            d.contains { $0.severity == sev && $0.definition == name && $0.message.contains(text) }
        }
        XCTAssertTrue(has(.error, "a", "instant `a` has the same name"), d.map(\.description).joined(separator: "\n"))
        XCTAssertTrue(has(.error, "b", "an instant atom takes no params"))
        XCTAssertTrue(has(.warning, "c", "unknown profile key `no_such_key`"))
        XCTAssertTrue(has(.warning, "c", "unknown key `unknown_key`"))
        XCTAssertTrue(has(.error, "d", "has no field `num`"))
        XCTAssertTrue(has(.error, "e", "the default does not parse"))
    }

    func testTierBAndCTablesAreReadButDeferred() {
        let user = T.Layer(name: "user", source: "[[ligature]]\ntrigger = \"->\"\nbody = '\\to '\n[[postfix]]\nname = \"hat\"\nbody = '\\hat{<<atom>>}'\n")
        let notes = T.Engine(layers: [user]).diagnostics.filter { $0.severity == .note }
        XCTAssertEqual(notes.count, 2)
    }

    // MARK: settings

    func testMasterSwitchDefaultsOff() {
        let s = T.Settings()
        XCTAssertFalse(s.enabled)
        for tier in T.Settings.Tier.allCases { XCTAssertFalse(s.isActive(tier), "\(tier)") }
        var on = s
        on.enabled = true
        XCTAssertTrue(on.isActive(.abbreviations))
        XCTAssertTrue(on.isActive(.instantAtoms))
        XCTAssertFalse(on.isActive(.ligatures), "ligatures stay off until chosen")
        XCTAssertTrue(on.isActive(.postfix))
        on.abbreviations = false
        XCTAssertFalse(on.isActive(.instantAtoms), "instant atoms are part of tier A")
        XCTAssertEqual(on.fractionOperator, "//", "a single `/` never expands by default")
        XCTAssertEqual(on.fractionTrigger, .tab)
        XCTAssertEqual(on.autoPreamble, .insert)
        XCTAssertEqual(on.leader, ";")
    }

    func testFileSettingsLayerButCannotEnableTheFeature() {
        let project = T.Layer(name: "project", source: #"""
        [settings]
        enabled = true
        ligatures = true
        leader = ","
        fraction_operator = "/"
        fraction_trigger = "auto"
        auto_preamble = "prompt"
        disabled_packs = ["beamer"]
        mystery = 1
        """#)
        let r = T.Engine(layers: [project]).registry
        XCTAssertFalse(r.settings.enabled, "a project file cannot switch the feature on")
        XCTAssertTrue(r.settings.ligatures)
        XCTAssertEqual(r.settings.leader, ",")
        XCTAssertEqual(r.settings.fractionOperator, "/")
        XCTAssertEqual(r.settings.fractionTrigger, .auto)
        XCTAssertEqual(r.settings.autoPreamble, .prompt)
        XCTAssertNil(r.resolve("frame", flags: ["text"]))
        XCTAssertTrue(r.diagnostics.contains { $0.severity == .warning && $0.message.contains("mystery") })

        var on = T.Settings()
        on.enabled = true
        let off = T.Layer(name: "project", source: "[settings]\nenabled = false\n")
        XCTAssertFalse(T.Engine(settings: on, layers: [off]).settings.enabled, "…but can switch it off")
    }

    func testBadSettingValuesAreDiagnosed() {
        let project = T.Layer(name: "project", source: "[settings]\nleader = \"a\"\nfraction_operator = \"%\"\nauto_preamble = \"sometimes\"\n")
        let r = T.Engine(layers: [project]).registry
        XCTAssertEqual(r.settings.leader, ";")
        XCTAssertEqual(r.settings.fractionOperator, "//")
        XCTAssertEqual(r.diagnostics.filter { $0.layer == "project" && $0.severity == .error }.count, 3)
        XCTAssertNil(T.Settings.leaderProblem(""), "an empty leader means bare Tab abbreviations")
        XCTAssertNotNil(T.Settings.leaderProblem("\\"))
        XCTAssertNotNil(T.Settings.leaderProblem(";;"))
    }

    func testSettingsCodableToleratesMissingKeys() throws {
        let decoded = try JSONDecoder().decode(T.Settings.self, from: Data(#"{"enabled": true, "leader": ","}"#.utf8))
        XCTAssertTrue(decoded.enabled)
        XCTAssertEqual(decoded.leader, ",")
        XCTAssertEqual(decoded.fractionOperator, "//")
        var s = T.Settings()
        s.enabled = true; s.disabledPacks = ["beamer"]; s.fractionOperator = "/"; s.profile = "upright"
        XCTAssertEqual(try JSONDecoder().decode(T.Settings.self, from: JSONEncoder().encode(s)), s)
    }

    // MARK: profiles

    func testProfileOverridesAndNamedProfiles() {
        let project = T.Layer(name: "project", source: #"""
        [settings]
        profile = "venue"
        [profiles.venue]
        ref_cmd = '\autoref'
        label_sep = "-"
        """#)
        let e = T.Engine(layers: [project])
        let x = try? e.expand("ref:fig-arch").get()
        XCTAssertEqual(x?.rendered(), "\\autoref{fig-arch}")
        XCTAssertEqual(x?.requires, [T.PackageRequirement("hyperref")], "the variant for \\autoref asks for hyperref")
        XCTAssertEqual(e.expandToString("sec{Intro}#"), "\\section{Intro}\\label{sec-intro}")

        let prefix = T.Layer(name: "user", source: "[profile]\n\"label_prefix.fig\" = \"figure\"\n")
        XCTAssertEqual(T.Engine(layers: [prefix]).expandToString("fig#x>img{a}"),
                       "\\begin{figure}\n  \\centering\n  \\includegraphics[${1:width=0.8\\linewidth}]{a}\n  \\label{figure:x}\n\\end{figure}")
    }

    func testUnknownProfileFallsBackToDefaults() {
        var s = T.Settings()
        s.profile = "nope"
        let e = T.Engine(settings: s)
        XCTAssertTrue(e.diagnostics.contains { $0.message.contains("unknown profile `nope`") })
        XCTAssertEqual(e.expandToString("ref:x"), "\\cref{x}")
    }

    func testOracleAndPrefixGuard() {
        let r = T.Engine().registry
        let text = T.ScopeStack.text().flags
        XCTAssertEqual(r.names(withPrefix: "su", flags: text), ["sub", "subfig"])
        XCTAssertEqual(r.names(withPrefix: "it", flags: text), ["items"], "`item` needs a list")
        XCTAssertEqual(r.names(withPrefix: "it", flags: T.ScopeStack.text("itemize").flags), ["item", "items"])
        let o = r.oracle(flags: text)
        XCTAssertTrue(o.isLeaf("ref"))
        XCTAssertEqual(o.acceptsChildren("img"), false)
        XCTAssertEqual(o.acceptsChildren("fig"), true)
        XCTAssertNil(o.acceptsChildren("nope"))
        XCTAssertFalse(o.allowsOverlay)
        XCTAssertTrue(r.oracle(flags: text, documentClass: "beamer").allowsOverlay)
        XCTAssertTrue(r.oracle(flags: T.ScopeStack.text("frame").flags).allowsOverlay)
    }
}
