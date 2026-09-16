#!/usr/bin/env python3
r"""Regenerate docs/dev/kernel-inventory-audit.md.

This is the generated form of the audit first written by hand in PR #361
("docs: kernel command inventory audit"), which found that a hand-derived
version of this document needed a full re-derivation twice in one day and
moved a third of its rows in about 24 hours: this repo's commit rate makes a
hand-maintained snapshot wrong within days. See that PR (and its "Revision"
section) for the original method write-up; this script is that method,
mechanised, so `cargo test` can catch drift the way it already does for
`docs/user/compiler.md` (`render_supported_latex.sh` / `supported_latex.rs`).

Universe
    `crates/compiler/supported/canonical-latex.tsv` (regenerated separately by
    `canonical_latex.py`, needs TeX Live), filtered to its `kernel` rows: real
    LaTeX2e kernel commands/environments confirmed by a pdfLaTeX run, entirely
    independent of what this compiler claims to support.

Genuine-implementation ground truth, most to least authoritative
  1. `flashtex-compiler --supported json` (built by this script, same as
     `render_supported_latex.sh`): the compiler's own registered inventory,
     already reconciled against the literal `match` arms in `src/parser.rs`
     and `src/math.rs` by `tests/supported_latex.rs` (both directions). A
     kernel name present here (any mode, any origin) is genuinely dispatched.
  2. Three small, curated, source-derived supplements for real dispatch that
     the inventory above does not enumerate, all parsed out of source here
     (not hand-copied), so they cannot themselves drift from the dispatch
     they document:
       - `parser::PREAMBLE_LENGTHS` (preamble-only dimen assignment, e.g.
         `\paperheight`) and `parser::TABLE_LENGTHS` (assignable anywhere,
         e.g. `\tabcolsep`): both guard *length register* assignment
         (`\setlength`-style), not a content-producing command, so
         `--supported` does not count them as "supported commands" even
         though using one does not raise an unsupported-feature diagnostic.
       - Every `("name", ...)` primitive/register table entry across
         `crates/tex-expansion/src/`: kernel primitives and registers such
         as `\day`/`\month`/`\year` (Count registers) or `\message`/`\read`/
         `\write`/`\verb` are dispatched entirely by the expansion engine,
         before the parser ever sees them, so they leave no trace in
         `crates/compiler/src/` and are outside `--supported json`, which
         only enumerates the compiler crate's own registrations.
  This is deliberately NOT `vocabulary.rs`'s `KNOWN_UNIMPLEMENTED_COMMANDS` /
  `KNOWN_UNIMPLEMENTED_ENVIRONMENTS`: those lists are for a different job
  (choosing the `unknown_command` vs `unsupported_feature` diagnostic for a
  name the parser's *own* dispatch already failed to recognise) and go stale
  in the opposite direction — `\marginpar` has a real dispatch arm
  (`src/parser.rs`, `fn marginpar`) and is registered in the inventory above,
  but is still listed in `KNOWN_UNIMPLEMENTED_COMMANDS`, which can only ever
  make the diagnostic *wording* wrong (`fn unsupported`'s own
  `debug_assert!(!BUILT_INS.contains(&name))` proves the entry is dead: a
  `BUILT_INS` name can never reach the code that reads this list) — it is
  irrelevant to whether the command actually works. `--report-stale-vocab`
  below cross-checks the two lists against `BUILT_INS`/`TEXT_EXTRA_ARMS` for
  exactly this reason, and is not part of the generated document.

Classification per kernel (kind, name), not in the ground truth above
  Table 1A  zero word-boundary matches anywhere in `crates/compiler/src/`.
  Table 1B  matches exist (comments, `KNOWN_UNIMPLEMENTED_*` entries, unrelated
            identifiers, ...) but none constitute genuine implementation.
  (Both are "not implemented"; the split only tells a reader whether a naive
  grep would have looked promising. Neither reproduces from the ground-truth
  ledger above, so this script does not try to explain *why* each hit is
  spurious the way the hand-audit did — that reading-in-context step is
  exactly what made the document expensive to keep current by hand.)
Present in the ground truth above
  Table 2   zero word-boundary matches in `crates/compiler/tests/` AND zero
            matches inside any `#[cfg(test)] mod` region of `crates/compiler/src/`
            (that region is treated as running to end of file, same as the
            hand audit: every file here puts its test module last).
  (not listed) implemented and covered by a name-specific test.

Run from anywhere: `python3 crates/compiler/scripts/kernel_inventory_audit.py`.
`cargo test -p flashtex-compiler --test kernel_inventory_audit` fails when the
checked-in document disagrees with a fresh run.
"""

import argparse
import json
import os
import re
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
COMPILER = HERE.parent
REPO = COMPILER.parent.parent
TSV = COMPILER / "supported" / "canonical-latex.tsv"
DEFAULT_OUT = REPO / "docs" / "dev" / "kernel-inventory-audit.md"

REGENERATE = "run crates/compiler/scripts/kernel_inventory_audit.py"


def kernel_rows():
    """[(kind, name), ...] for canonical-latex.tsv's `kernel` rows."""
    rows = []
    for line in TSV.read_text(encoding="utf-8").splitlines():
        if not line or line.startswith("#"):
            continue
        set_, kind, name = line.split("\t")
        if set_ == "kernel":
            rows.append((kind, name))
    return rows


def build_compiler_binary():
    """Build `flashtex-compiler` the same way render_supported_latex.sh does."""
    env = dict(os.environ)
    env.setdefault("CARGO_BUILD_JOBS", "2")
    subprocess.run(
        [
            "cargo",
            "build",
            "--quiet",
            "--manifest-path",
            str(COMPILER / "Cargo.toml"),
            "--bin",
            "flashtex-compiler",
        ],
        check=True,
        env=env,
    )
    target_dir = Path(env.get("CARGO_TARGET_DIR", COMPILER / "target"))
    return target_dir / "debug" / "flashtex-compiler"


def supported_inventory(binary):
    """(command names, environment names) currently registered as supported."""
    out = subprocess.run(
        [str(binary), "--supported", "json"],
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    data = json.loads(out)
    commands = {c["name"] for c in data["commands"]}
    environments = {e["name"] for e in data["environments"]}
    return commands, environments


def parse_str_array(src, const_name):
    """Names in `const CONST_NAME: &[&str] = &[...]`, one line or many."""
    start = src.index(const_name)
    start = src.index("&[", start)
    end = src.index("];", start)
    return re.findall(r'"([^"]*)"', src[start:end])


def name_pattern(name):
    """A grep -w-equivalent boundary that also works for a trailing `*`
    (`\\b` does not: `*` and the quote/brace after it are both non-word
    characters, so no word boundary ever occurs between them)."""
    return re.compile(r"(?<!\w)" + re.escape(name) + r"(?!\w)")


def rust_files(root):
    return sorted(p for p in root.rglob("*.rs"))


def all_files(root):
    """Every regular file under `root`, recursively — matching the method's
    own `grep -rnwF` invocation, which is not limited to `*.rs`: the test
    corpus's coverage evidence includes `tests/oracle/**/*.json` and
    `tests/*_corpus/**` fixtures grep would (and does) also search."""
    return sorted(p for p in root.rglob("*") if p.is_file())


def any_match(pattern, files):
    for path in files:
        text = path.read_text(encoding="utf-8", errors="replace")
        if pattern.search(text):
            return True
    return False


TEST_MOD = re.compile(r"#\[cfg\(test\)\]\s*\nmod\s+\w+")


def inline_test_regions(files):
    """Per-file text of everything from its `#[cfg(test)] mod ...` onward.

    Not just any `#[cfg(test)]`: a file may also have a `#[cfg(test)] use
    ...;` near its top (test-only imports), and taking that as the split
    point would swallow the whole file's production code as "test"."""
    regions = []
    for path in files:
        text = path.read_text(encoding="utf-8", errors="replace")
        m = TEST_MOD.search(text)
        if m:
            regions.append(text[m.start() :])
    return regions


def any_match_in_texts(pattern, texts):
    return any(pattern.search(t) for t in texts)


def expansion_engine_command_names():
    """Kernel commands dispatched entirely by `flashtex-tex-expansion`
    (TeX/LaTeX primitives and registers such as `\\day`/`\\month`/`\\year`
    Count registers, `\\message`/`\\read`/`\\write`/`\\verb` primitives): they
    are real dispatch the parser never sees, so they leave no trace in
    `crates/compiler/src/` and are absent from `--supported json`, which only
    enumerates the compiler crate's own registrations (see the module
    docstring). Every `("name", ...)` tuple across the engine crate's source
    is a primitive/register table entry keyed by the exact control-sequence
    name, the same shape `EXPANSION_COMMANDS` uses in `supported.rs`."""
    names = set()
    for path in (COMPILER.parent / "tex-expansion" / "src").rglob("*.rs"):
        text = path.read_text(encoding="utf-8", errors="replace")
        names |= set(re.findall(r'\("([A-Za-z]+)"\s*,', text))
    return names


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", type=Path, default=DEFAULT_OUT)
    parser.add_argument(
        "--bin",
        type=Path,
        default=None,
        help="an already-built flashtex-compiler binary (e.g. Cargo's own "
        "CARGO_BIN_EXE_flashtex-compiler, from the drift-check test in "
        "tests/kernel_inventory_audit.rs); skips the `cargo build` below",
    )
    parser.add_argument(
        "--report-stale-vocab",
        action="store_true",
        help="print the KNOWN_UNIMPLEMENTED_COMMANDS/ENVIRONMENTS staleness "
        "check (see the module docstring) to stderr instead of writing the doc",
    )
    args = parser.parse_args()

    binary = args.bin if args.bin else build_compiler_binary()
    impl_commands, impl_environments = supported_inventory(binary)

    parser_src = (COMPILER / "src" / "parser.rs").read_text(encoding="utf-8")
    preamble_lengths = set(parse_str_array(parser_src, "const PREAMBLE_LENGTHS"))
    table_lengths = set(parse_str_array(parser_src, "const TABLE_LENGTHS"))
    engine_commands = expansion_engine_command_names()

    if args.report_stale_vocab:
        report_stale_vocabulary(parser_src)
        return

    src_files = rust_files(COMPILER / "src")
    test_files = all_files(COMPILER / "tests")
    inline_test_texts = inline_test_regions(src_files)

    table_1a, table_1b, table_2 = [], [], []
    for kind, name in kernel_rows():
        pattern = name_pattern(name)
        if kind == "command":
            implemented = (
                name in impl_commands
                or name in preamble_lengths
                or name in table_lengths
                or name in engine_commands
            )
        else:
            implemented = name in impl_environments

        if implemented:
            tested = any_match(pattern, test_files) or any_match_in_texts(
                pattern, inline_test_texts
            )
            if not tested:
                table_2.append((kind, name))
            continue

        if any_match(pattern, src_files):
            table_1b.append((kind, name))
        else:
            table_1a.append((kind, name))

    doc = render(table_1a, table_1b, table_2)
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(doc, encoding="utf-8")
    print(
        f"regenerated {args.out}: "
        f"Table 1A {len(table_1a)}, Table 1B {len(table_1b)}, Table 2 {len(table_2)}"
    )


def render(table_1a, table_1b, table_2):
    lines = []
    lines.append("# Kernel command inventory audit")
    lines.append("")
    lines.append(f"GENERATED by `{REGENERATE}`; do not edit by hand.")
    lines.append("")
    lines.append("## Method")
    lines.append("")
    lines.append(
        "`crates/compiler/supported/canonical-latex.tsv`'s `kernel` rows "
        f"({len(table_1a) + len(table_1b) + len(table_2)} problem rows of "
        "440 total: 410 commands, 30 environments) are cross-referenced "
        "against `flashtex-compiler --supported json` (the compiler's own "
        "registered inventory, reconciled against the real `match` arms in "
        "`src/parser.rs`/`src/math.rs` by `tests/supported_latex.rs`), plus "
        "two length-register dispatch tables that inventory omits by design "
        "(`PREAMBLE_LENGTHS`, `TABLE_LENGTHS` in `src/parser.rs`) and every "
        "primitive/register table entry in `crates/tex-expansion/src/` "
        "(engine-dispatched kernel primitives such as `\\day`/`\\message`/"
        "`\\verb`, invisible to the compiler crate), all parsed from source, "
        "not hand-copied. A name absent from all four is genuinely "
        "unimplemented. Test coverage is a word-boundary match "
        "in `crates/compiler/tests/` or inside any `#[cfg(test)] mod` "
        "region of `crates/compiler/src/` (treated as running to end of "
        "file). See this script's module docstring "
        "(`crates/compiler/scripts/kernel_inventory_audit.py`) for the full "
        "method, including why `vocabulary.rs`'s `KNOWN_UNIMPLEMENTED_*` "
        "lists are deliberately not part of it."
    )
    lines.append("")

    lines.append(
        f"## Table 1A — kernel names with ZERO matches in `crates/compiler/src/` ({len(table_1a)})"
    )
    lines.append("")
    lines.append("| kind | name | reproducing grep (run from repo root) | result |")
    lines.append("| ---- | ---- | -------------------------------------- | ------ |")
    for kind, name in table_1a:
        grep = f"grep -rnwF -e '{name}' crates/compiler/src/"
        lines.append(f"| {kind} | {name} | `{grep}` | (no output) |")
    lines.append("")

    lines.append(
        f"## Table 1B — names with matches but NO genuine implementation ({len(table_1b)})"
    )
    lines.append("")
    lines.append(
        "Word-grep below returns hits (comments, `KNOWN_UNIMPLEMENTED_*` "
        "entries, unrelated identifiers, ...), but none of them is a "
        "registered command/environment, nor a `PREAMBLE_LENGTHS`/"
        "`TABLE_LENGTHS`/engine-primitive entry — see Method."
    )
    lines.append("")
    lines.append("| kind | name | word grep (run from repo root) | genuine-impl check |")
    lines.append("| ---- | ---- | ------------------------------- | ------------------- |")
    for kind, name in table_1b:
        grep = f"grep -rnwF -e '{name}' crates/compiler/src/"
        lines.append(
            f"| {kind} | {name} | `{grep}` | not in `--supported json`, "
            "`PREAMBLE_LENGTHS`, `TABLE_LENGTHS` or a `crates/tex-expansion/src/` "
            "primitive/register table |"
        )
    lines.append("")

    lines.append(
        "## Table 2 — implemented but with ZERO name matches in tests/ or "
        f"inline test modules ({len(table_2)})"
    )
    lines.append("")
    lines.append(
        "\"Implemented\" means registered in `--supported json`, a "
        "`PREAMBLE_LENGTHS`/`TABLE_LENGTHS` dispatch guard, or a "
        "`crates/tex-expansion/src/` primitive/register table entry. Both test "
        "greps below return no output: `grep -rnwF -e 'NAME' "
        "crates/compiler/tests/` and the same word match inside any "
        "`#[cfg(test)] mod` region of `crates/compiler/src/`."
    )
    lines.append("")
    lines.append("| kind | name |")
    lines.append("| ---- | ---- |")
    for kind, name in table_2:
        lines.append(f"| {kind} | {name} |")
    lines.append("")
    return "\n".join(lines) + "\n"


def report_stale_vocabulary(parser_src):
    """KNOWN_UNIMPLEMENTED_COMMANDS/ENVIRONMENTS entries that are already
    reachable through real dispatch *before* the code that reads them ever
    runs — see the module docstring. `fn unsupported`'s own
    `debug_assert!(!BUILT_INS.contains(&name))` is the proof for commands:
    if this is empty, that assertion can never fire in a debug test run.
    """
    vocab_src = (COMPILER / "src" / "vocabulary.rs").read_text(encoding="utf-8")
    built_ins = set(parse_str_array(parser_src, "const BUILT_INS"))
    supported_src = (COMPILER / "src" / "supported.rs").read_text(encoding="utf-8")
    text_extra_arms = set(parse_str_array(supported_src, "const TEXT_EXTRA_ARMS"))
    expansion_names = set(
        re.findall(
            r'\("([^"]*)"',
            supported_src[
                supported_src.index("const EXPANSION_COMMANDS") : supported_src.index(
                    "];", supported_src.index("const EXPANSION_COMMANDS")
                )
            ],
        )
    )
    known_unimplemented_commands = parse_str_array(
        vocab_src, "const KNOWN_UNIMPLEMENTED_COMMANDS"
    )
    reachable = built_ins | text_extra_arms
    stale = sorted(set(known_unimplemented_commands) & reachable)
    probably_stale = sorted(
        (set(known_unimplemented_commands) & expansion_names) - reachable
    )

    implemented_environments = set(
        parse_str_array(vocab_src, "const IMPLEMENTED_ENVIRONMENTS")
    )
    math_src = (COMPILER / "src" / "math.rs").read_text(encoding="utf-8")
    grid_start = math_src.index("const GRID_ENVIRONMENTS")
    grid_end = math_src.index("];", grid_start)
    grid_environments = set(re.findall(r'\(\s*"([^"]*)"', math_src[grid_start:grid_end]))
    known_unimplemented_environments = parse_str_array(
        vocab_src, "const KNOWN_UNIMPLEMENTED_ENVIRONMENTS"
    )
    stale_envs = sorted(
        set(known_unimplemented_environments)
        & (implemented_environments | grid_environments)
    )

    print(
        f"KNOWN_UNIMPLEMENTED_COMMANDS: {len(set(known_unimplemented_commands))} "
        f"unique entries, {len(stale)} are in BUILT_INS/TEXT_EXTRA_ARMS (provably "
        "dead: fn unsupported's debug_assert!(!BUILT_INS.contains(&name)) means "
        "these can never reach the code that reads this list):",
        file=sys.stderr,
    )
    print(stale, file=sys.stderr)
    print(
        f"\n{len(probably_stale)} more are EXPANSION_COMMANDS entries (macro/"
        "counter/length-definition commands handled by the expansion pass "
        "before the parser's fallback ever runs); likely the same bug, not "
        "proven with the same certainty:",
        file=sys.stderr,
    )
    print(probably_stale, file=sys.stderr)
    print(
        f"\nKNOWN_UNIMPLEMENTED_ENVIRONMENTS: {len(set(known_unimplemented_environments))} "
        f"unique entries, {len(stale_envs)} are also in vocabulary.rs's own "
        "IMPLEMENTED_ENVIRONMENTS or math::GRID_ENVIRONMENTS (self-contradictory):",
        file=sys.stderr,
    )
    print(stale_envs, file=sys.stderr)


if __name__ == "__main__":
    main()
