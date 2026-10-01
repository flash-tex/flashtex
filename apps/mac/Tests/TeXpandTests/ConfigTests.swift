import XCTest
@testable import FlashTeXEditorCore

/// PLAN M8: the six layers, merge semantics, magic comments, pack defaults.
final class ConfigTests: XCTestCase {
    typealias T = TeXpand

    static var on: T.Settings { var s = T.Settings(); s.enabled = true; return s }

    func testLayerOrderAndMerge() {
        let synthesized = T.Layer(name: "macros", source: "[[abbr]]\nname = \"claim\"\nbody = 'from macros'\n[[abbr]]\nname = \"sec\"\nbody = 'never wins'\n")
        let pack = T.Layer(name: "texpand-packs/ml.toml", source: "[pack]\nname = \"ml\"\nopt_in = true\nscope = [\"math\"]\nrequires = [\"bm\"]\n[[abbr]]\nname = \"loss\"\nbody = '\\mathcal{L}'\n")
        let user = T.Layer(name: "texpand.toml (user)", source: "[profile]\ndiff_d = '\\mathrm{d}'\nfrac = '\\dfrac'\n[[abbr]]\nname = \"claim\"\nbody = 'from user'\n")
        let project = T.Layer(name: "texpand.toml (project)", source: "[profile]\nfrac = '\\tfrac'\n[[abbr]]\nname = \"claim\"\nbody = 'from project'\n")
        var s = Self.on
        s.enabledPacks = ["ml"]
        let config = T.Config(synthesized: [synthesized], packs: [pack], user: user, project: project)
        let e = T.Engine(registry: config.registry(settings: s))
        XCTAssertEqual(e.expandToString("claim"), "from project", "the project file beats the user file beats the macros")
        XCTAssertEqual(e.expandToString("sec{A}"), "\\section{A}", "synthesized definitions never override the built-ins")
        XCTAssertEqual(e.expandToString("dd:y/x", in: T.Context(scope: .math)), "\\tfrac{\\mathrm{d}y}{\\mathrm{d}x}", "profile keys merge key-wise")
        let loss = try? e.expand("loss", in: T.Context(scope: .math)).get()
        XCTAssertEqual(loss?.rendered(), "\\mathcal{L}")
        XCTAssertEqual(loss?.requires.map(\.name), ["bm"], "a pack's requires are its definitions'")
        XCTAssertEqual(e.expandToString("loss"), "error: `loss` is not available here (it works in: math)", "and its scope")
        XCTAssertNil(T.Engine(registry: T.Config(packs: [pack]).registry(settings: Self.on)).registry.resolve("loss", flags: ["math"]),
                     "an opt-in pack needs `packs`")
    }

    func testMagicComments() throws {
        let doc = """
        % !TEX root = main.tex
        % !texpand profile=upright leader=, disable=sec,xx ligatures=on
        %!texpand fraction_operator=/ enabled=on
        \\documentclass{article}
        """
        let layer = try XCTUnwrap(T.magicComments(in: doc))
        let e = T.Engine(registry: T.Config(magic: layer).registry(settings: Self.on))
        XCTAssertEqual(e.settings.leader, ",")
        XCTAssertEqual(e.settings.profile, "upright")
        XCTAssertTrue(e.settings.ligatures)
        XCTAssertEqual(e.settings.fractionOperator, "/")
        XCTAssertEqual(e.expandToString("sec"), "error: unknown abbreviation `sec`")
        XCTAssertNil(e.registry.ligatures.first { $0.trigger == "xx" })
        XCTAssertEqual(e.expandToString("dd:y/x", in: T.Context(scope: .math)), "\\frac{\\mathrm{d}y}{\\mathrm{d}x}")
        XCTAssertFalse(T.Engine(registry: T.Config(magic: layer).registry(settings: T.Settings())).settings.enabled,
                       "a magic comment cannot switch TeXpand on")
        let off = try XCTUnwrap(T.magicComments(in: "% !texpand enabled=off"))
        XCTAssertFalse(T.Engine(registry: T.Config(magic: off).registry(settings: Self.on)).settings.enabled, "…but can switch it off")
        XCTAssertNil(T.magicComments(in: "no comments\n% just a comment"))
        let bad = try XCTUnwrap(T.magicComments(in: "% !texpand leader=a nonsense=1"))
        let d = T.Engine(registry: T.Config(magic: bad).registry(settings: Self.on)).diagnostics.filter { $0.layer == "% !texpand" }
        XCTAssertEqual(d.map(\.severity), [.error, .warning], "diagnostics name the layer: \(d)")
    }

    func testMalformedDefinitionOnlyDiagnoses() {
        let project = T.Layer(name: "texpand.toml (project)", source: "[[abbr]]\nname = \"sec\"\nbody = '<<nope>>'\n[[abbr]]\nname = \"ok\"\nbody = 'fine'\n")
        let e = T.Engine(registry: T.Config(project: project).registry(settings: Self.on))
        XCTAssertEqual(e.expandToString("sec{A}"), "\\section{A}", "the built-in survives a broken override")
        XCTAssertEqual(e.expandToString("ok"), "fine")
        XCTAssertEqual(e.diagnostics.filter { $0.layer == "texpand.toml (project)" }.map(\.description),
                       ["error: texpand.toml (project):1 [sec]: `body`: unknown hole `<<nope>>`"])
    }
}
