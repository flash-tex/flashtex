//! `flashtex --version` and `flashtex licenses`: the version, and the
//! licences with the GPL source offer (retirement plan §6, RQ6 and RQ19).
//!
//! This program is MIT and holds no engine code. The engine hosts it runs as
//! separate programs (`flashtex-host`, `flashtex-host-unicode`) are
//! GPL-2.0-or-later and link GPL libraries, so whoever receives them is owed
//! their source. The owner's RQ19 ruling (DESIGN.md §13) makes the public
//! repository that offer, linked from the app's About panel and every
//! release's notes; this command says the same thing, in the same words, to
//! whoever has only the command line.

/// The repository holding the source of everything that ships.
pub const SOURCE_URL: &str = "https://github.com/flash-tex/flashtex";

/// The source line the Mac app's About panel shows
/// (`apps/mac/Sources/FlashTeXMac/AboutPanel.swift`, `sourceLine`) and
/// `.github/workflows/release.yml`'s notes repeat; a test keeps them equal.
pub const SOURCE_LINE: &str =
    "Source code for the FlashTeX engine (GPL-2.0-or-later): https://github.com/flash-tex/flashtex";

/// The version (`build.rs`: the release's, else the crate's).
pub fn version() -> &'static str {
    env!("FLASHTEX_VERSION")
}

/// The Git revision this was built from (`build.rs`), or `unknown`.
pub fn revision() -> &'static str {
    env!("FLASHTEX_GIT_SHA")
}

/// `flashtex --version`: `flashtex <version> (<revision>)`.
pub fn version_line() -> String {
    format!("flashtex {} ({})", version(), revision())
}

/// Where this build's own source is: the release tag for a release build,
/// else the revision when it is a clean one, else the repository.
pub fn source_tree(release: bool, version: &str, revision: &str) -> String {
    if release {
        return format!("{SOURCE_URL}/tree/v{version}");
    }
    let clean = !revision.is_empty()
        && revision != "unknown"
        && revision.bytes().all(|b| b.is_ascii_hexdigit());
    if clean {
        format!("{SOURCE_URL}/tree/{revision}")
    } else {
        SOURCE_URL.to_string()
    }
}

/// `flashtex licenses`.
pub fn text() -> String {
    let release = !env!("FLASHTEX_RELEASE").is_empty();
    render(
        &version_line(),
        &source_tree(release, version(), revision()),
    )
}

fn render(version_line: &str, tree: &str) -> String {
    format!(
        "{version_line}

flashtex (this program) is free software under the MIT License. It holds no
engine code: to typeset, it runs an engine host as a separate program,
flashtex-host (Classic mode, a translation of pdfTeX) or flashtex-host-unicode
(Unicode mode, a translation of XeTeX).

The engine hosts are free software under the GNU General Public License,
version 2 or later. They link TeX Live 2026's kpathsea, zlib, libpng and xpdf
(xpdf is GPL version 2 or 3 only, so the hosts are distributed under GPL
version 2 or 3), and Unicode mode's host also HarfBuzz, FreeType, TECkit and
ICU's data. You may have their complete corresponding source code:

{SOURCE_LINE}
This build: {tree}

The hosts are crates/flashtex-engine and crates/flashtex-xetex there; the
libraries they link are vendored under third_party/, each with a README that
names its version and upstream pin. Every release also attaches the source of
its tag (flashtex-<version>-source.tar.gz). The licence texts ship with this
program: share/flashtex/LICENSE (this program's) and share/flashtex/engine/
(the hosts' licences and the notices of what they link).
"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_source_line_is_the_about_panels() {
        // The app's About panel and the release notes carry the offer in
        // these words (#1806); the command line must not drift from them.
        let about = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../apps/mac/Sources/FlashTeXMac/AboutPanel.swift");
        if let Ok(swift) = std::fs::read_to_string(&about) {
            assert!(
                swift.contains(&format!("static let sourceLine = \"{SOURCE_LINE}\"")),
                "AboutPanel.sourceLine differs from licenses::SOURCE_LINE"
            );
        }
        let release = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../.github/workflows/release.yml");
        if let Ok(yml) = std::fs::read_to_string(&release) {
            assert!(
                yml.contains(SOURCE_LINE),
                "release.yml's notes repeat SOURCE_LINE"
            );
        }
    }

    #[test]
    fn the_offer_names_the_licences_and_the_source() {
        let t = render(
            "flashtex 9.9.9 (abc)",
            "https://github.com/flash-tex/flashtex/tree/v9.9.9",
        );
        assert!(t.starts_with("flashtex 9.9.9 (abc)\n"));
        assert!(t.contains("MIT License"));
        assert!(t.contains("GNU General Public License,\nversion 2 or later"));
        assert!(t.contains(SOURCE_LINE));
        assert!(t.contains("This build: https://github.com/flash-tex/flashtex/tree/v9.9.9"));
        for lib in [
            "kpathsea", "zlib", "libpng", "xpdf", "HarfBuzz", "FreeType", "TECkit",
        ] {
            assert!(t.contains(lib), "{lib}");
        }
    }

    #[test]
    fn the_tree_is_the_tag_the_revision_or_the_repository() {
        assert_eq!(
            source_tree(true, "0.3.0", "0123456789ab"),
            "https://github.com/flash-tex/flashtex/tree/v0.3.0"
        );
        assert_eq!(
            source_tree(false, "0.1.0", "0123456789ab"),
            "https://github.com/flash-tex/flashtex/tree/0123456789ab"
        );
        assert_eq!(
            source_tree(false, "0.1.0", "0123456789ab-dirty"),
            SOURCE_URL
        );
        assert_eq!(source_tree(false, "0.1.0", "unknown"), SOURCE_URL);
    }

    #[test]
    fn version_line_names_the_program() {
        assert!(version_line().starts_with("flashtex "));
        assert!(version_line().ends_with(')'));
    }
}
