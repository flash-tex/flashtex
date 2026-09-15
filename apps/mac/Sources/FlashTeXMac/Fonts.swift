import AppKit
import CoreText
import FlashTeXProtocol

/// Preview/export faces. LaTeX's default is Computer Modern; we use Latin Modern
/// (GUST Font License) when it can be found, falling back to Times-Roman (the
/// Core-14 face the current compiler measures with). The face name reported by
/// the layout producer wins once runtime carries font identity; until then the
/// preview picks by `FLASHTEX_PREVIEW_FACE` (`latin-modern` | `times`).
enum PreviewFonts {
    enum Face: String { case latinModern = "latin-modern", times }

    static let latinModernSearchPaths: [String] = [
        ProcessInfo.processInfo.environment["FLASHTEX_LM_DIR"],
        Bundle.main.resourceURL?.appendingPathComponent("Fonts").path,
        // Repository copy (apps/mac/Fonts) for development and tests.
        URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
            .deletingLastPathComponent().appendingPathComponent("Fonts").path,
        "/usr/local/texlive/2026basic/texmf-dist/fonts/opentype/public/lm",
        "/usr/local/texlive/2026/texmf-dist/fonts/opentype/public/lm",
        "/Library/TeX/Root/texmf-dist/fonts/opentype/public/lm",
    ].compactMap { $0 }

    // MARK: Resource generation

    /// Monotonic generation of the font resource set the preview resolves
    /// against. It moves whenever an input of name resolution or of what a
    /// PostScript name denotes changes: Latin Modern registration
    /// (`FLASHTEX_LM_DIR`, bundle, repository copy, TeX Live), a producer face
    /// switch (`flashtex-compiler` Times ↔ `flashtex-render` Latin Modern), the
    /// `FLASHTEX_PREVIEW_FACE` override, or an explicit `invalidateResources()`.
    /// Consumers stamp what they build with the generation it was built at and
    /// never serve an entry from another generation (`PreviewTextCache`), so
    /// invalidation is keyed rather than a scattered `clear()`. Main-thread
    /// state, like the cache: the draw closure, the shell's worker attach and
    /// the CoreGraphics export all run there.
    private(set) static var resourceGeneration: UInt64 = 0

    /// Records a change of the font resource set that the other inputs do not
    /// already cover (a caller registered or unregistered fonts itself).
    static func invalidateResources() { resourceGeneration &+= 1 }

    /// Directory Latin Modern was registered from, nil when not found. Only
    /// meaningful after `latinModernRegistered` has been consulted.
    private(set) static var latinModernDirectory: String?

    /// Every Latin Modern file the layout producer can request
    /// (`flashtex-render` `latin_modern_outline`, t1lmr.fd boundaries):
    /// regular 5–17, bold 5–12, italic 7–12, bold-italic 10, LM Math, and the
    /// secondary double-struck math face `NewCMMath-Regular.otf` (New Computer
    /// Modern Math, msbm's `\mathbb` design; `mathfont::BB_FONT_FILE`), plus
    /// the typewriter designs `latin_modern_outline` returns for
    /// `FamilyKind::Tt` (t1lmtt.fd boundaries: `m/n` 8, 9, 10, `<11->` 12;
    /// one 10 pt design for italic, slanted, caps and the bold `lmmonolt`),
    /// the non-upright roman designs (`latinModernRomanShapeFaceFiles`) and
    /// the sans designs (`latinModernSansFaceFiles`).
    /// The vendored `apps/mac/Fonts` (pinned by `SUPPLEMENTARY-FACES.json`
    /// plus the Commander manifest) holds all of them; when the registered
    /// directory lacks any, the gap is recorded in `latinModernMissingFaces`
    /// so a CoreText fallback for that master is never silent.
    static let latinModernFaceFiles: [String] =
        [5, 6, 7, 8, 9, 10, 12, 17].map { "lmroman\($0)-regular.otf" }
        + [5, 6, 7, 8, 9, 10, 12].map { "lmroman\($0)-bold.otf" }
        + [7, 8, 9, 10, 12].map { "lmroman\($0)-italic.otf" }
        + ["lmroman10-bolditalic.otf", "latinmodern-math.otf", newComputerModernMathFile]
        + latinModernMonoFaceFiles
        + latinModernRomanShapeFaceFiles
        + latinModernSansFaceFiles

    /// The non-upright roman designs `latin_modern_outline` returns for
    /// `FamilyKind::Rm` outside `m/n`, `bx/n`, `m/it` and `bx/it`
    /// (`t1lmr.fd` boundaries: `m/sl` 8, 9, 10, 12, `<15->` 17; one 10 pt
    /// design for bold slanted, small caps, unslanted and the demi series).
    /// `article.cls`/`book.cls` set every `headings`/`myheadings` running
    /// head in `\slshape`, so without these every running head in the
    /// bundle drew a substituted roman outline and warned
    /// `font_outline_substituted`.
    static let latinModernRomanShapeFaceFiles: [String] =
        [8, 9, 10, 12, 17].map { "lmromanslant\($0)-regular.otf" }
        + ["lmromanslant10-bold.otf",
           "lmromancaps10-regular.otf", "lmromancaps10-oblique.otf",
           "lmromanunsl10-regular.otf",
           "lmromandemi10-regular.otf", "lmromandemi10-oblique.otf"]

    /// The sans designs `latin_modern_outline` returns for `FamilyKind::Sf`
    /// (`t1lmss.fd` boundaries: `m/n` and `m/sl` 8, 9, 10, 12, `<15.5->` 17;
    /// one 10 pt design for bold, bold oblique and the demi-condensed
    /// series). Without these every `\textsf` run drew a substituted roman
    /// outline.
    static let latinModernSansFaceFiles: [String] =
        [8, 9, 10, 12, 17].map { "lmsans\($0)-regular.otf" }
        + [8, 9, 10, 12, 17].map { "lmsans\($0)-oblique.otf" }
        + ["lmsans10-bold.otf", "lmsans10-boldoblique.otf",
           "lmsansdemicond10-regular.otf", "lmsansdemicond10-oblique.otf"]

    /// The typewriter faces, kept separate so a caller can tell a
    /// `\texttt` gap from a body-text gap. Without these the producer draws
    /// `\texttt` from a substituted roman outline and warns
    /// `font_outline_substituted`.
    static let latinModernMonoFaceFiles: [String] =
        [8, 9, 10, 12].map { "lmmono\($0)-regular.otf" }
        + ["lmmono10-italic.otf", "lmmonoslant10-regular.otf",
           "lmmonocaps10-regular.otf", "lmmonocaps10-oblique.otf",
           "lmmonolt10-bold.otf", "lmmonolt10-boldoblique.otf"]

    /// The secondary math face the producer draws `\mathbb` from when it is
    /// bundled: New Computer Modern Math, whose double-struck letters follow
    /// AMS msbm10 (pdfLaTeX's `\mathbb`) rather than Latin Modern Math's
    /// open-face design. Resolved by raw bytes through `V2FontStore` like
    /// every other face; nothing is registered with CoreText by name.
    static let newComputerModernMathFile = "NewCMMath-Regular.otf"
    static let newComputerModernMathPostScriptName = "NewCMMath-Regular"
    static let newComputerModernMathSHA256 = "60394d357348f68cd301764fe61cc502a5858e1c4ff21b948a1d14d82586a7a2"

    /// Files of `latinModernFaceFiles` absent from `directory`.
    static func latinModernMissingFaces(in directory: String) -> [String] {
        latinModernFaceFiles.filter { !FileManager.default.fileExists(atPath: directory + "/" + $0) }
    }

    /// Faces the registered directory lacks (empty when it is complete or when
    /// nothing is registered). Meaningful after `latinModernRegistered`.
    private(set) static var latinModernMissingFaces: [String] = []

    /// Whether the Latin Modern Roman masters are registered with CoreText for
    /// this process. Registration happens on first access and moves
    /// `resourceGeneration`: an `LMRoman*` name asked for before it resolves to
    /// a CoreText fallback, afterwards to the real font. The first search
    /// directory holding any `lmroman*.otf` wins (explicit overrides first);
    /// the faces it lacks are recorded, never silently substituted.
    private(set) static var latinModernRegistered: Bool = {
        for dir in latinModernSearchPaths {
            let url = URL(fileURLWithPath: dir)
            guard let files = try? FileManager.default.contentsOfDirectory(at: url, includingPropertiesForKeys: nil) else { continue }
            let otfs = files.filter { $0.pathExtension == "otf" && $0.lastPathComponent.hasPrefix("lmroman") }
            guard !otfs.isEmpty else { continue }
            CTFontManagerRegisterFontURLs(otfs as CFArray, .process, true, nil)
            latinModernDirectory = dir
            latinModernMissingFaces = latinModernMissingFaces(in: dir)
            invalidateResources()
            return true
        }
        return false
    }()

    /// PostScript name of `latinmodern-math.otf`, the `lm.math` resource the
    /// compiler binds blackboard bold, `\setminus` and `\Longrightarrow` to.
    static let latinModernMathPostScriptName = "LatinModernMath-Regular"

    /// Whether `latinmodern-math.otf` is registered with CoreText for this
    /// process: the first search directory holding it wins. The roman
    /// registration above deliberately takes only `lmroman*` files.
    private(set) static var latinModernMathRegistered: Bool = {
        for dir in latinModernSearchPaths {
            let url = URL(fileURLWithPath: dir).appendingPathComponent("latinmodern-math.otf")
            guard FileManager.default.fileExists(atPath: url.path) else { continue }
            CTFontManagerRegisterFontURLs([url] as CFArray, .process, true, nil)
            invalidateResources()
            return true
        }
        return false
    }()

    static func isLatinModernMathFamily(_ family: String) -> Bool {
        family.lowercased().trimmingCharacters(in: .whitespaces) == "latin modern math"
    }

    /// Set by the shell from the attached producer: `flashtex-render` (the new
    /// pipeline, Latin Modern metrics) → `.latinModern`; `flashtex-compiler`
    /// (Core-14 Times metrics today) → `.times`. `FLASHTEX_PREVIEW_FACE` overrides.
    /// A change moves `resourceGeneration`.
    static var producerFace: Face = .times {
        didSet { if oldValue != producerFace { invalidateResources() } }
    }

    /// `FLASHTEX_PREVIEW_FACE` read once: `ProcessInfo.environment` copies the
    /// whole environment on every access and this is consulted per drawn item.
    /// `overrideEnvironmentFace` replaces it explicitly (tests, a future
    /// preference) and moves `resourceGeneration`.
    private(set) static var environmentFace: Face? = Face(rawValue: ProcessInfo.processInfo.environment["FLASHTEX_PREVIEW_FACE"] ?? "")

    static func overrideEnvironmentFace(_ face: Face?) {
        guard face != environmentFace else { return }
        environmentFace = face
        invalidateResources()
    }

    static var requested: Face { environmentFace ?? producerFace }

    /// Active face: Latin Modern when requested and registered, else Times.
    static var active: Face { requested == .latinModern && latinModernRegistered ? .latinModern : .times }

    /// Producer heuristic until runtime carries font identity.
    static func face(forProducer executableName: String?) -> Face {
        guard let n = executableName?.lowercased() else { return .times }
        return n.contains("render") ? .latinModern : .times
    }

    /// Legacy selection (no font hint): the active face, regular weight/style.
    static func postScriptName(size: Double, bold: Bool = false, italic: Bool = false) -> String {
        postScriptName(face: active, size: size, bold: bold, italic: italic)
    }

    /// Latin Modern optical size by nominal point size, the master the layout
    /// producer picks (`t1lmr.fd` design-size boundaries, mirrored from
    /// render-pipeline `FontSet::latin_modern_file`): regular
    /// 5/6/7/8/9/10/12/17, bold 5–12, italic 7–12, bold-italic only 10 — every
    /// one a file in `latinModernFaceFiles`, so the name never denotes a
    /// CoreText fallback once the vendored directory is registered.
    static func postScriptName(face: Face, size: Double, bold: Bool, italic: Bool) -> String {
        switch face {
        case .times:
            switch (bold, italic) {
            case (false, false): return "Times-Roman"
            case (true, false): return "Times-Bold"
            case (false, true): return "Times-Italic"
            case (true, true): return "Times-BoldItalic"
            }
        case .latinModern:
            return "LMRoman\(latinModernMaster(size: size, bold: bold, italic: italic))-"
                + (bold && italic ? "BoldItalic" : bold ? "Bold" : italic ? "Italic" : "Regular")
        }
    }

    /// The design size of the Latin Modern Roman master for a nominal size and
    /// style (see `postScriptName(face:size:bold:italic:)`).
    static func latinModernMaster(size s: Double, bold: Bool, italic: Bool) -> Int {
        switch (bold, italic) {
        case (true, true): return 10
        case (false, true): return s < 7.5 ? 7 : s < 8.5 ? 8 : s < 9.5 ? 9 : s < 11 ? 10 : 12
        case (true, false): return s < 5.5 ? 5 : s < 6.5 ? 6 : s < 7.5 ? 7 : s < 8.5 ? 8 : s < 9.5 ? 9 : s < 11 ? 10 : 12
        case (false, false): return s < 5.5 ? 5 : s < 6.5 ? 6 : s < 7.5 ? 7 : s < 8.5 ? 8 : s < 9.5 ? 9 : s < 11 ? 10 : s < 15 ? 12 : 17
        }
    }

    static func ctFont(size: Double) -> CTFont {
        CTFontCreateWithName(postScriptName(size: size) as CFString, size, nil)
    }

    // MARK: font-hints-v1

    /// A requested family the preview could not honor and the face used instead.
    struct Substitution: Hashable {
        var family: String
        var weight: RuntimeV1.PageItem.FontHint.Weight
        var style: RuntimeV1.PageItem.FontHint.Style
        var usedFace: String
        var requested: String {
            family + (weight == .bold ? " bold" : "") + (style == .italic ? " italic" : "")
        }
        var description: String { "font substituted: \(requested) → \(usedFace)" }
    }

    struct Resolved: Equatable {
        var postScriptName: String
        /// Non-nil when the requested family was not available and another face
        /// was used; the requested metrics were then NOT preserved.
        var substitution: Substitution?
    }

    /// Families that map to the registered Latin Modern Roman masters.
    static func isLatinModernFamily(_ family: String) -> Bool {
        let f = family.lowercased().trimmingCharacters(in: .whitespaces)
        return f.hasPrefix("latin modern") || f.hasPrefix("lmroman") || f == "lm roman" || f == "computer modern" || f.hasPrefix("computer modern")
    }

    /// Families that map to the Core-14 Times faces.
    static func isTimesFamily(_ family: String) -> Bool { core14Family(family) == .times }

    /// The three Core-14 text families macOS ships (`Times-Roman`, `Helvetica`,
    /// `Courier` with their Bold/Italic-or-Oblique faces). `flashtex-compiler`
    /// names its hint families after the Core-14 *face* it measured with
    /// (`Times-Bold`, `Times-Italic`, …); the face suffix is accepted as an alias
    /// of the family and the hint's own weight/style pick the face.
    enum Core14Family: String { case times, helvetica, courier }

    static func core14Family(_ family: String) -> Core14Family? {
        var f = family.lowercased().trimmingCharacters(in: .whitespaces)
        for suffix in ["-bolditalic", "-boldoblique", "-bold", "-italic", "-oblique", "-roman"] where f.hasSuffix(suffix) {
            f = String(f.dropLast(suffix.count)); break
        }
        switch f {
        case "times", "times new roman", "times roman", "timesnewroman": return .times
        case "helvetica", "helvetica neue", "arial": return .helvetica
        case "courier", "courier new": return .courier
        default: return nil
        }
    }

    /// Core-14 PostScript face for a family at the requested weight/style.
    static func core14PostScriptName(_ family: Core14Family, bold: Bool, italic: Bool) -> String {
        switch family {
        case .times: return postScriptName(face: .times, size: 10, bold: bold, italic: italic)
        case .helvetica, .courier:
            let base = family == .helvetica ? "Helvetica" : "Courier"
            switch (bold, italic) {
            case (false, false): return base
            case (true, false): return base + "-Bold"
            case (false, true): return base + "-Oblique"
            case (true, true): return base + "-BoldOblique"
            }
        }
    }

    /// Resolves an explicit `font-hints-v1` hint to a PostScript face. Latin
    /// Modern families use the registered LM masters (weight/style honored);
    /// Core-14 families (Times, Helvetica, Courier) use their Core-14 faces;
    /// anything else — or Latin Modern when it is not registered on this
    /// machine — falls back to Times with the requested weight/style and is
    /// reported as a substitution. A nil hint is legacy selection
    /// (`postScriptName(size:)`), never a substitution.
    static func resolve(hint: RuntimeV1.PageItem.FontHint?, size: Double) -> Resolved {
        guard let hint else { return Resolved(postScriptName: postScriptName(size: size), substitution: nil) }
        let bold = hint.weight == .bold, italic = hint.style == .italic
        // Checked before the roman families: "Latin Modern Math" also has the
        // "latin modern" prefix, but the roman masters lack its glyphs.
        if isLatinModernMathFamily(hint.family) {
            if latinModernMathRegistered {
                return Resolved(postScriptName: latinModernMathPostScriptName, substitution: nil)
            }
            let times = postScriptName(face: .times, size: size, bold: bold, italic: italic)
            return Resolved(postScriptName: times,
                            substitution: Substitution(family: hint.family, weight: hint.weight, style: hint.style, usedFace: times))
        }
        if isLatinModernFamily(hint.family), latinModernRegistered {
            return Resolved(postScriptName: postScriptName(face: .latinModern, size: size, bold: bold, italic: italic), substitution: nil)
        }
        if let family = core14Family(hint.family) {
            return Resolved(postScriptName: core14PostScriptName(family, bold: bold, italic: italic), substitution: nil)
        }
        let times = postScriptName(face: .times, size: size, bold: bold, italic: italic)
        return Resolved(postScriptName: times,
                        substitution: Substitution(family: hint.family, weight: hint.weight, style: hint.style, usedFace: times))
    }

    /// Every distinct substitution a result would need, in first-seen order.
    static func substitutions(in result: RuntimeV1.CompileResult) -> [Substitution] {
        var seen = Set<Substitution>(), out: [Substitution] = []
        for page in result.pages {
            for case .text(let t) in page.items {
                if let sub = resolve(hint: t.font, size: t.fontSizePt).substitution, seen.insert(sub).inserted { out.append(sub) }
            }
        }
        return out
    }
}
