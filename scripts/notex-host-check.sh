#!/usr/bin/env bash
# The app's no-TeX-Live path, end to end (DESIGN.md 4.4, D12; lane NO-TEXLIVE).
#
# tools/bundle/texbundle.py gate (notex-gate.yml) compiles the gate documents
# with flashtex-initex. This script checks the path the Mac app takes instead:
# flashtex-host, configured exactly as EngineV3Host.environment configures it
# (FLASHTEX_BUNDLE_LOCK naming flashtex-bundle.lock, and the user's consent as
# FLASHTEX_BUNDLE_ALLOW_FETCH=<digest>@<url>, EngineV3Bundle.swift), compiling
# over its socket (dl3-client) three documents: the app's blank-article
# template (ProjectScaffold.swift), an article with amsmath, graphicx and
# hyperref, and a beamer deck. TeX Live is hidden: an empty environment, a
# HOME and TMPDIR of our own, PATH of the system directories only, and on
# macOS sandbox-exec denying every TeX Live location (texbundle.py's list).
#
# It asserts the host's HELLO says no TeX Live, a bundle resolver and a ready
# pdflatex format, and that each document compiles (status ok, pages > 0),
# cold and then warm. It checks that the path works, not byte identity: the
# gate compares PDFs with pdflatex's. A fourth document (fixtures/notex-tools)
# has a bibliography and an index: with external tools allowed, the host's
# own bibtex and makeindex must make TeX Live's .bbl, .blg, .ind and .ilg
# byte for byte, from the bundle alone.
#
#   scripts/notex-host-check.sh --host target/release/flashtex-host \
#       --client target/release/dl3-client --out DIR \
#       [--lock tools/bundle/tl2026/flashtex-bundle.lock] [--url U --digest D]
#
# --url/--digest (e.g. a packed bundle's file:// URL, as bundle-publish.yml
# calls the gate) replace the lock's; the consent is given for them.
set -euo pipefail

here="$(cd "$(dirname "$0")/.." && pwd)"
host="" client="" out="" lock="$here/tools/bundle/tl2026/flashtex-bundle.lock" url="" digest=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --host) host="$2"; shift 2 ;;
    --client) client="$2"; shift 2 ;;
    --out) out="$2"; shift 2 ;;
    --lock) lock="$2"; shift 2 ;;
    --url) url="$2"; shift 2 ;;
    --digest) digest="$2"; shift 2 ;;
    *) echo "notex-host-check: unknown argument $1" >&2; exit 2 ;;
  esac
done
[[ -x "$host" && -x "$client" && -n "$out" ]] || {
  echo "usage: $0 --host FLASHTEX_HOST --client DL3_CLIENT --out DIR [--lock F] [--url U --digest D]" >&2; exit 2; }
host="$(cd "$(dirname "$host")" && pwd)/$(basename "$host")"
client="$(cd "$(dirname "$client")" && pwd)/$(basename "$client")"
lock="$(cd "$(dirname "$lock")" && pwd)/$(basename "$lock")"

# A lock or an explicit bundle, and the consent string the app would pass.
if [[ -z "$url" ]]; then
  url="$(sed -n 's/^url *= *"\(.*\)".*/\1/p' "$lock")"
  digest="$(sed -n 's/^digest *= *"\(.*\)".*/\1/p' "$lock")"
fi
[[ -n "$url" && -n "$digest" ]] || { echo "notex-host-check: no bundle url/digest" >&2; exit 2; }

mkdir -p "$out"
out="$(cd "$out" && pwd -P)"
work="$out/work"
rm -rf "$work"
mkdir -p "$work/home" "$work/tmp" "$work/docs"

# Hide TeX Live (texbundle.py TEXLIVE_PATHS, plus the user trees and MacTeX's
# login-shell PATH entry the resolver reads, /etc/paths.d/TeX).
wrap=()
if [[ "$(uname)" == Darwin && -x /usr/bin/sandbox-exec ]]; then
  deny=""
  for p in /usr/local/texlive /Library/TeX /opt/homebrew/opt/texlive /opt/homebrew/Cellar/texlive \
           /usr/local/opt/texlive /usr/local/Cellar/texlive /opt/texlive /usr/share/texlive /usr/share/texmf \
           "$HOME/Library/texmf" "$HOME/texmf"; do
    deny+=" (subpath \"$p\")"
  done
  printf '(version 1)\n(allow default)\n(deny file-read* file-write*%s (literal "/etc/paths.d/TeX"))\n' "$deny" \
    > "$work/notex.sb"
  /usr/bin/sandbox-exec -f "$work/notex.sb" /usr/bin/true || { echo "notex-host-check: sandbox-exec does not run here" >&2; exit 1; }
  if /usr/bin/sandbox-exec -f "$work/notex.sb" /bin/test -e /Library/TeX/texbin/kpsewhich; then
    echo "notex-host-check: the sandbox does not hide /Library/TeX" >&2; exit 1
  fi
  wrap=(/usr/bin/sandbox-exec -f "$work/notex.sb")
else
  for x in pdflatex pdftex kpsewhich; do
    if PATH=/usr/bin:/bin command -v "$x" >/dev/null; then
      echo "notex-host-check: $x is on the system PATH and cannot be hidden here" >&2; exit 1
    fi
  done
fi

doc() { mkdir -p "$work/docs/$1"; cat > "$work/docs/$1/main.tex"; }
# ProjectTemplate.blankArticle (apps/mac/Sources/FlashTeXMac/ProjectScaffold.swift).
doc blank <<'EOF'
\documentclass{article}
\usepackage[margin=1in]{geometry}
\usepackage{amsmath,amssymb}

\title{Untitled}
\author{}
\date{\today}

\begin{document}
\maketitle

\section{Introduction}

\end{document}
EOF
doc article <<'EOF'
\documentclass{article}
\usepackage{amsmath}
\usepackage{graphicx}
\usepackage{hyperref}
\begin{document}
\section{Intro}\label{s:i}
See Section~\ref{s:i} and \url{https://example.org}.
\begin{align}
a^2+b^2 &= c^2 \\
\int_0^1 x\,dx &= \tfrac12
\end{align}
\includegraphics[width=2cm]{example-image}
\end{document}
EOF
doc beamer <<'EOF'
\documentclass{beamer}
\usetheme{Madrid}
\title{Deck}
\author{A}
\begin{document}
\frame{\titlepage}
\begin{frame}{One}
\begin{itemize}\item a \item b\end{itemize}
$\sum_{i=1}^n i = \frac{n(n+1)}{2}$
\end{frame}
\end{document}
EOF

sock="$work/host.sock"
env_bundle=(FLASHTEX_BUNDLE_LOCK="$lock" FLASHTEX_BUNDLE_ALLOW_FETCH="$digest@$url")
lock_url="$(sed -n 's/^url *= *"\(.*\)".*/\1/p' "$lock")"
lock_digest="$(sed -n 's/^digest *= *"\(.*\)".*/\1/p' "$lock")"
if [[ "$url" != "$lock_url" || "$digest" != "$lock_digest" ]]; then
  # A bundle other than the lock's: the environment names it, as a developer would.
  env_bundle=(FLASHTEX_BUNDLE_URL="$url" FLASHTEX_BUNDLE_DIGEST="$digest")
fi
/usr/bin/env -i HOME="$work/home" TMPDIR="$work/tmp/" PATH=/usr/bin:/bin:/usr/sbin:/sbin LANG=C \
  "${env_bundle[@]}" FLASHTEX_BUNDLE_CACHE_DIR="$out/bundle-cache" FLASHTEX_FORMAT_CACHE_DIR="$out/format-cache" \
  "${wrap[@]}" "$host" --socket "$sock" > "$out/host.log" 2>&1 &
pid=$!
cleanup() { kill "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true; }
trap cleanup EXIT
for _ in $(seq 1 300); do [[ -S "$sock" ]] && break; kill -0 "$pid" 2>/dev/null || break; sleep 0.1; done
[[ -S "$sock" ]] || { echo "notex-host-check: the host did not start"; cat "$out/host.log"; exit 1; }

failed=0
report="$out/report.md"
{ echo "### No TeX Live: the app's path (flashtex-host, bundle ${digest:0:12})"; echo
  echo "| document | cold first page ms | cold done ms | warm done ms | pages | verdict |"; echo "|---|---|---|---|---|---|"; } > "$report"
for d in blank article beamer; do
  set +e
  "$client" --socket "$sock" --root "$work/docs/$d" --main main.tex --repeat 2 > "$out/$d.jsonl" 2> "$out/$d.err"
  code=$?
  set -e
  ok="$(python3 - "$out/$d.jsonl" "$code" <<'PY'
import json, sys
rows = [json.loads(l) for l in open(sys.argv[1]) if l.startswith("{")]
good = sys.argv[2] == "0" and len(rows) == 2 and all(r["status"] == "ok" and r["pages"] > 0 for r in rows)
c = rows[0] if rows else {}
w = rows[1] if len(rows) > 1 else {}
print(f'{c.get("first_page_ms")} | {c.get("done_ms")} | {w.get("done_ms")} | {c.get("pages")} | {"ok" if good else "FAILED"}')
PY
)"
  echo "| $d | $ok |" >> "$report"
  [[ "$ok" == *"| ok" ]] || { failed=1; echo "notex-host-check: $d failed:"; cat "$out/$d.jsonl" "$out/$d.err"; }
done

# Bibliography and index with no TeX Live (lane RUST-TOOLS): with external
# tools allowed, as the app allows them for a trusted project, the host runs
# its own bibtex and makeindex (src/bibtex.rs, src/makeindex.rs) with .bst and
# .ist files from the bundle, then compiles again. Their outputs must be
# byte-identical to TeX Live 2026's bibtex and makeindex on the same document
# (fixtures/notex-tools/expected, made by TeX Live's programs as latexmk runs
# them: `bibtex main`, `makeindex -o main.ind main.idx`). The project carries
# TeX Live's plain.bst (unmodified): the published bundle packs the packages
# compiled documents read, and TeX Live's `bibtex` package, which holds the
# standard styles, is not one of them yet (fixtures/notex-tools/README).
tools_doc="$work/docs/tools"
mkdir -p "$tools_doc"
cp "$here/fixtures/notex-tools/main.tex" "$here/fixtures/notex-tools/refs.bib" \
   "$here/fixtures/notex-tools/plain.bst" "$tools_doc/"
set +e
"$client" --socket "$sock" --root "$tools_doc" --main main.tex --output-dir "$tools_doc/out" \
  --external-tools auto > "$out/tools.jsonl" 2> "$out/tools.err"
code=$?
set -e
tools_ok="$(python3 - "$out/tools.jsonl" "$code" "$tools_doc/out" "$here/fixtures/notex-tools/expected" <<'PY'
import json, os, sys
rows = [json.loads(l) for l in open(sys.argv[1]) if l.startswith("{")]
bad = []
if sys.argv[2] != "0" or len(rows) != 1 or rows[0].get("status") != "ok":
    bad.append("the compile failed")
r = rows[0] if rows else {}
done = {t.get("tool"): t for t in r.get("tools", []) if t.get("event") == "done"}
skips = [t for t in r.get("tools", []) if t.get("event") == "skip"]
for tool in ("bibtex", "makeindex"):
    t = done.get(tool)
    if t is None:
        bad.append(f"{tool} did not run (skips: {skips})")
    elif t.get("status") not in ("ok", "warnings") or not t.get("changed"):
        bad.append(f"{tool}: {t}")
if not r.get("followups"):
    bad.append("no follow-up compile")
for name in sorted(os.listdir(sys.argv[4])):
    got = os.path.join(sys.argv[3], name)
    want = open(os.path.join(sys.argv[4], name), "rb").read()
    if not os.path.isfile(got) or open(got, "rb").read() != want:
        bad.append(f"{name} differs from TeX Live's")
print("ok" if not bad else "FAILED: " + "; ".join(bad))
PY
)"
echo >> "$report"
echo "Bibliography and index (bibtex and makeindex in the host, no TeX Live): $tools_ok" >> "$report"
[[ "$tools_ok" == ok ]] || { failed=1; echo "notex-host-check: tools: $tools_ok"; cat "$out/tools.jsonl" "$out/tools.err"; }
cleanup
trap - EXIT

# The HELLO the host logs: no TeX Live, the bundle, a ready format.
python3 - "$out/host.log" "$digest" >> "$report" <<'PY' || failed=1
import json, sys
hello = None
for line in open(sys.argv[1], errors="replace"):
    if line.startswith("flashtex-host: {") and '"resolver"' in line:
        hello = json.loads(line.split(": ", 1)[1])
bad = []
if hello is None:
    bad.append("no HELLO in the host's log")
else:
    if hello.get("texlive") is not None:
        bad.append(f"the host found a TeX Live: {hello['texlive']}")
    if not str(hello.get("resolver", "")).startswith("bundle " + sys.argv[2]):
        bad.append(f"resolver is {hello.get('resolver')!r}, not the bundle")
    if not any(f.get("name") == "pdflatex" and f.get("status") == "ready" for f in hello.get("formats", [])):
        bad.append(f"pdflatex format not ready: {hello.get('formats')}")
print()
print(f"HELLO: texlive `{hello and hello.get('texlive')}`, resolver `{hello and hello.get('resolver')}`, "
      f"formats `{hello and hello.get('formats')}`")
for b in bad:
    print(f"- **FAILED**: {b}")
    print(f"notex-host-check: {b}", file=sys.stderr)
sys.exit(1 if bad else 0)
PY
cat "$report"
rm -rf "$work"
exit "$failed"
