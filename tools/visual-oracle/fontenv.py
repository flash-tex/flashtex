"""Shared font environment and font-diagnostic gate for the oracle harnesses.

Oracle tooling only; pdflatex never runs in the product path.

Why this module exists
----------------------

Every corpus harness runs `flashtex-render` in a subprocess and compares glyph
positions against pinned pdflatex references. Two independent mistakes made
those comparisons silently meaningless, and both are fixed here once so that
every harness gets the fix.

**1. The caller's environment was discarded.** The harnesses used to build the
subprocess environment as::

    env = dict(os.environ, FLASHTEX_FONT_DIRS=args.fonts, FLASHTEX_TFM_DIRS=args.fonts)

`dict(os.environ, KEY=value)` *overrides* `KEY`, so an operator who had
correctly exported `FLASHTEX_TFM_DIRS` at a real metrics tree had it thrown
away. Worse, the value it was replaced with is the *outline* directory
(`apps/mac/Fonts`), which holds `.otf` files and no `.tfm` at all. The renderer
then found no metrics, fell back to OpenType advances, and every position was
wrong — yet the run still exited "recovered", so the corpus scored 0/59 with a
perfectly good binary. See `render_env`, which never overrides an exported
value.

Note that `--fonts <dir>` alone cannot reach the bundled metrics either: the
renderer's bundle-relative texmf roots are `<exe>/../Resources/texmf` and
`<exe>/texmf` (`render-pipeline/src/fonts.rs::bundle_texmf_roots`), *not*
`<fontdir>/texmf`. So when the TFM directories are not given explicitly,
`tfm_dirs_for` derives them by walking `<fontroot>/texmf` for directories that
actually contain `.tfm` files.

**2. A fallback run scored as a pass.** The harnesses collected diagnostics
into the JSON row but the pass/fail verdict never consulted them. Font
diagnostics are exactly the ones that invalidate a geometry comparison — the
renderer says, in so many words, "the layout is not the reference geometry" —
so `font_diagnostics` finds them and harnesses fail loudly on any hit.

There are two required-metrics roots, and `FLASHTEX_TFM_DIRS` feeds both
-----------------------------------------------------------------------

`FLASHTEX_FONT_DIRS` / `FLASHTEX_TFM_DIRS` feed the general discovery list
(`render-pipeline/src/fonts.rs::font_dirs` / `tfm_dirs_for`). The *rooted*
required-metrics reader is **not** cut off from the environment, as an earlier
revision of this file claimed. `FontSet::required_metrics` builds its candidate
roots from `self.tfm_dirs` -- which is exactly `FLASHTEX_TFM_DIRS` -- twice:

* `FontSet::texmf_roots` strips the `fonts/tfm/public/lm` suffix off each TFM
  directory and offers what is left as a `texmf-dist` root;
* every TFM directory is then also offered as a *flat* root.

Only if both come up empty does it fall back to the executable-relative trees
(`<exe>/../Resources/texmf`, `<exe>/texmf`, `<exe>/../share/flashtex/texmf`),
which is the path an installed app or CLI tarball uses.

Three consequences, each measured on this checkout rather than assumed:

* The three-directory spelling -- `.../public/lm`, `.../jknappen/ec`,
  `.../public/amsfonts/symbols` -- is sufficient on its own. No `texmf` tree
  next to the binary is needed: amsmath scores 59/59 with only the environment
  set.
* `--fonts <dir>` alone is *also* sufficient now, which it was not before this
  module existed: when the metrics directories are unset, `tfm_dirs()` below
  derives them by walking `<fonts>/texmf`. Measured: amsmath 59/59 with
  `FLASHTEX_FONT_DIRS`/`FLASHTEX_TFM_DIRS` both unset and only `--fonts` given.
* The remaining trap is an *explicitly exported but too narrow*
  `FLASHTEX_TFM_DIRS`. An exported value always wins over the derivation (that
  is the whole point of this module), so naming only `public/lm` drops the EC
  metrics under `jknappen/ec`, and a T1 document silently gets roman widths.
  This is not hypothetical: CI exported exactly that, and
  `render-pipeline`'s `enumitem_keys_oracle` failed on it with the *y*
  baselines exact and only the *x* positions drifting 0.5-2 bp -- it passes the
  moment `jknappen/ec` is added. The amsmath corpus does not catch this,
  because its fixtures are OT1 math and never load an EC face: it reads 59/59
  under `public/lm` alone.

The assertion below still keys on `font_diagnostics` being empty rather than on
configuration, because that catches whichever root failed and needs no theory
about which one was consulted.

Usage in a harness
------------------

    import fontenv

    env = fontenv.render_env(args.fonts, args.tfm_dirs)   # respects the caller
    print(fontenv.describe(env))                          # provenance in the log
    ...
    bad = fontenv.font_diagnostics(diags)
    good = ... and not bad
"""
import os
import pathlib
import sys

#: Diagnostic codes that mean the geometry produced is **not** the reference
#: geometry, so any position comparison against a pdflatex reference is void.
#: A harness that sees one of these must fail rather than score the run.
#:
#: Derived, not hand-written. `crates/render-pipeline/src/fontdiag.rs` is the
#: source of truth for every code `typeset.rs` emits and for what each one
#: means; `crates/render-pipeline/tests/fontdiag.rs` fails if a new code lands
#: there unclassified. This module used to keep its own copy, as did
#: `apps/mac/scripts/{texmf-acceptance.sh,faces-acceptance.py}`, and the three
#: had drifted apart.
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[2] / "scripts"))
from font_diagnostics import codes as _font_codes  # noqa: E402

FONT_DIAGNOSTIC_CODES = frozenset(_font_codes("geometry_void"))

#: Reported but not fatal: the outline differs while the metrics — and so the
#: positions these harnesses compare — are still the reference ones.
ADVISORY_FONT_DIAGNOSTIC_CODES = frozenset(_font_codes("advisory"))


def split(value):
    """Split a colon-separated directory list, dropping empty entries."""
    if not value:
        return []
    return [p for p in value.split(os.pathsep) if p]


def tfm_dirs_for(font_dirs):
    """Derive metrics directories from outline roots.

    For each root, every directory at or under `<root>/texmf` that directly
    contains at least one `.tfm` file, plus the root itself (the flat layout
    the renderer also accepts). Deterministic order, duplicates dropped.

    This exists because `--fonts <dir>` does not reach `<dir>/texmf` on its
    own; see the module docstring.
    """
    out = []
    for root in font_dirs:
        for base in (os.path.join(root, "texmf"), root):
            if not os.path.isdir(base):
                continue
            for dirpath, dirnames, filenames in os.walk(base):
                dirnames.sort()
                if any(f.endswith(".tfm") for f in filenames) and dirpath not in out:
                    out.append(dirpath)
    return out


def render_env(fonts=None, tfm=None, base=None):
    """Build the subprocess environment for `flashtex-render`.

    `fonts` and `tfm` may each be a single directory or a colon-separated
    list. **An exported `FLASHTEX_FONT_DIRS` / `FLASHTEX_TFM_DIRS` always
    wins** — the arguments are defaults for an unset variable, never an
    override — because a correctly configured environment must not be
    silently replaced by a harness default (that is the bug this module was
    written for).

    When `FLASHTEX_TFM_DIRS` is neither exported nor passed, metrics
    directories are derived from the outline roots with `tfm_dirs_for`.
    """
    env = dict(os.environ if base is None else base)

    if not env.get("FLASHTEX_FONT_DIRS"):
        font_dirs = split(fonts)
        if font_dirs:
            env["FLASHTEX_FONT_DIRS"] = os.pathsep.join(font_dirs)
    font_dirs = split(env.get("FLASHTEX_FONT_DIRS"))

    if not env.get("FLASHTEX_TFM_DIRS"):
        tfm_dirs = split(tfm) or tfm_dirs_for(font_dirs)
        if tfm_dirs:
            env["FLASHTEX_TFM_DIRS"] = os.pathsep.join(tfm_dirs)
    return env


def describe(env):
    """One-line provenance for the gate log: which directories were used."""
    return ("fonts: FLASHTEX_FONT_DIRS=%s\nfonts: FLASHTEX_TFM_DIRS=%s"
            % (env.get("FLASHTEX_FONT_DIRS", "<unset>"),
               env.get("FLASHTEX_TFM_DIRS", "<unset>")))


def font_diagnostics(diagnostics, codes=FONT_DIAGNOSTIC_CODES):
    """Return the diagnostics that invalidate a geometry comparison.

    `diagnostics` is the raw list from a `compile_result` payload. A non-empty
    result means the run used substituted metrics and must not be scored.
    """
    return [d for d in (diagnostics or []) if d.get("code") in codes]


def format_font_diagnostics(bad):
    """Render offending diagnostics compactly for a failure message."""
    return "; ".join(
        "%s: %s" % (d.get("code"), (d.get("message") or "")[:160]) for d in bad)


def report_font_failure(name, bad, env=None):
    """Print a loud, self-explaining failure for one fixture.

    Font diagnostics are not a fixture result — they mean the harness was
    misconfigured — so they are called out separately from a position miss.
    """
    print("FONT-ENV FAILURE %s: the run used substituted metrics, so its "
          "positions are not comparable to the pdflatex reference." % name)
    print("  %s" % format_font_diagnostics(bad))
    if env is not None:
        for line in describe(env).splitlines():
            print("  %s" % line)


def add_font_arguments(parser, repo):
    """Add the standard `--fonts` / `--tfm-dirs` pair to a subparser.

    Both accept a colon-separated list. Defaults come from the environment so
    that `--help` shows what a configured shell will actually use.
    """
    parser.add_argument(
        "--fonts", default=os.environ.get(
            "FLASHTEX_FONT_DIRS", os.path.join(repo, "apps", "mac", "Fonts")),
        help="outline directories, colon separated; an exported "
             "FLASHTEX_FONT_DIRS takes precedence")
    parser.add_argument(
        "--tfm-dirs", default=os.environ.get("FLASHTEX_TFM_DIRS"),
        help="metrics directories, colon separated; an exported "
             "FLASHTEX_TFM_DIRS takes precedence. Default: every directory "
             "under <fonts>/texmf that contains .tfm files")
