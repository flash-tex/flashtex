#!/usr/bin/env bash
# Licence boundary check -- DESIGN.md §3 and §9.6.
#
# FlashTeX ships an MIT Mac app and an MIT iPad companion around a GPL-2-or-later
# engine that they reach only by running its host process and speaking
# display-list-v3 over a socket. That boundary is the whole licensing argument,
# and it is exactly the kind of thing that breaks silently in one Cargo.toml
# line, so CI enforces it on every pull request:
#
#   A. No crate other than crates/flashtex-engine itself has flashtex-engine
#      anywhere in its dependency graph -- `cargo metadata`'s resolve graph,
#      which is what actually links -- in ANY of the repository's cargo
#      workspaces (the root one plus the three crates it excludes).
#
#   B. Nothing under apps/ios references a GPL crate or links Rust engine
#      artifacts: no GPL crate named in a build input, no binary/system-library
#      target, no link flags, no build step that shells out to cargo, and no
#      symlink escaping into crates/.
#
#   C. No MIT crate reaches a copyleft PDF renderer: poppler (GPL-2/3) or
#      MuPDF (AGPL-3), or their Rust bindings (any package named poppler*,
#      mupdf*), anywhere in its `cargo metadata` resolve graph; and no MIT
#      crate's build.rs or Swift/Xcode build input under apps/ links
#      libpoppler or libmupdf. They are the obvious PDF rasterisers on Linux,
#      so this is the likely accidental link (cross-platform evaluation
#      2026-09-30, §4 item 11). PDFium (pdfium, pdfium-render) is NOT denied:
#      it is BSD-3-Clause/Apache-2.0, which MIT code may link. GPL crates (the
#      engine) are exempt: GPL-2-or-later may use GPL poppler.
#
#   D. The Typst host (typst-host/, its own workspace, DESIGN.md §15.2/§15.9)
#      links no GPL package and no poppler/MuPDF (from its committed
#      Cargo.lock, without downloading its crates), path-depends only on the
#      MIT crates/display-list-v3, holds no byte copy of a GPL crate's file
#      and no reference to the engine crate, and (from cargo metadata) links
#      no GPL/LGPL/AGPL or unlicensed package from any source.
#
# The engine crate is created by another lane. Until crates/flashtex-engine
# exists, check A says so and passes; check B runs regardless, because what it
# enforces holds today and is what keeps the boundary cheap to defend later.
#
# Usage:
#   scripts/check-license-boundary.sh          human-readable; exit 1 on a violation
#   scripts/check-license-boundary.sh --list   print what it checks, then exit
#
# Portability: POSIX-ish bash 3.2 (the /bin/bash on macOS runners) plus python3.
set -euo pipefail
set -o pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# Tests only (scripts/tests/check-license-boundary.test.sh): check another tree.
if [[ -n "${FLASHTEX_BOUNDARY_ROOT:-}" ]]; then
  ROOT="$(cd "$FLASHTEX_BOUNDARY_ROOT" && pwd)"
fi
cd "$ROOT"

ENGINE_DIR="crates/flashtex-engine"
ALLOW_FILE="scripts/license-boundary-allow.txt"
FAILURES=0

fail() { printf 'VIOLATION  %s\n' "$*" >&2; FAILURES=$((FAILURES + 1)); }
ok()   { printf 'ok         %s\n' "$*"; }
info() { printf '           %s\n' "$*"; }

# Print this file's leading comment block (everything after the shebang, up to
# the first line that is not a comment). A line range would go stale the moment
# a paragraph is added -- which it did.
header() {
  awk 'NR == 1 { next } /^#/ { sub(/^# ?/, ""); print; next } { exit }' "${BASH_SOURCE[0]}"
}

if [[ "${1:-}" == "--list" ]]; then
  header
  exit 0
fi

pkg_name() { # pkg_name <path/to/Cargo.toml>
  [[ -f "$1" ]] || return 0
  sed -n '/^\[package\]/,/^\[/p' "$1" | sed -n 's/^ *name *= *"\([^"]*\)".*/\1/p' | head -1
}

# ---------------------------------------------------------------------------
# A. Nothing but the engine may depend on the engine
# ---------------------------------------------------------------------------
ENGINE_PKG="$(pkg_name "$ENGINE_DIR/Cargo.toml")"

if [[ -z "$ENGINE_PKG" ]]; then
  ok "A  $ENGINE_DIR does not exist yet, so no crate can depend on it"
  info "this check starts enforcing the moment the engine lane lands that directory"
else
  # Any crate that keeps its own Cargo.lock is its own workspace, and its graph
  # must be resolved separately or the check has a blind spot exactly where the
  # CLI lives. Since lane P0-RETIRE-VENDOR retired
  # crates/render-pipeline/vendor/ and folded those three lockfiles into the root
  # one, there are none -- the root workspace globs all 38 crates. The loop stays
  # so the check keeps covering any that reappear.
  WORKSPACES="."
  for c in crates/*/; do
    [[ -f "${c}Cargo.lock" ]] || continue
    WORKSPACES="$WORKSPACES ${c%/}"
  done

  ALLOW=""
  if [[ -f "$ALLOW_FILE" ]]; then
    ALLOW="$(sed -e 's/#.*//' "$ALLOW_FILE" | tr '[:space:]' '\n' | grep -v '^$' | tr '\n' ' ' || true)"
  fi

  for ws in $WORKSPACES; do
    meta=""
    if ! meta="$(cd "$ROOT/$ws" && cargo metadata --format-version 1 --locked 2>/dev/null)"; then
      if ! meta="$(cd "$ROOT/$ws" && cargo metadata --format-version 1 2>/dev/null)"; then
        fail "A [$ws]  cargo metadata failed; the dependency graph could not be checked"
        continue
      fi
      info "A [$ws]  --locked failed; the lockfile is out of date with the manifests"
    fi
    # The metadata goes through a file, not a pipe: `python3 -` already reads
    # its program from stdin (the heredoc below), so stdin is not available for
    # data. Getting that wrong fails open -- an empty graph looks clean.
    meta_file="$(mktemp "${TMPDIR:-/tmp}/flashtex-boundary-XXXXXX")"
    printf '%s' "$meta" > "$meta_file"
    report="$(ENGINE="$ENGINE_PKG" ALLOW="$ALLOW" WS="$ws" python3 - "$meta_file" <<'PY'
import collections, json, os, sys

engine = os.environ["ENGINE"]
allow = set(os.environ["ALLOW"].split())
ws = os.environ["WS"]
with open(sys.argv[1], encoding="utf-8") as fh:
    meta = json.load(fh)

name_of = {p["id"]: p["name"] for p in meta["packages"]}
nodes = (meta.get("resolve") or {}).get("nodes") or []
if not nodes:
    print("ERROR\t%s\tcargo metadata returned no resolve graph" % ws)
    raise SystemExit(0)

engine_ids = {i for i, n in name_of.items() if n == engine}
if not engine_ids:
    print("CLEAN\t%s\t%s is not in this workspace's graph at all" % (ws, engine))
    raise SystemExit(0)

# Reverse the resolve graph, then breadth-first out from the engine: every
# package that can reach it, with the shortest path that gets there. All
# dependency kinds count -- a dev-dependency still links GPL code into a test
# binary, and a build-dependency still links it into a build script.
rev = collections.defaultdict(list)
for n in nodes:
    for dep in n.get("dependencies") or [d["pkg"] for d in n.get("deps", [])]:
        rev[dep].append(n["id"])

parent, seen = {}, set(engine_ids)
queue = collections.deque(engine_ids)
while queue:
    cur = queue.popleft()
    for up in rev.get(cur, ()):
        if up in seen:
            continue
        seen.add(up)
        parent[up] = cur
        queue.append(up)

lines = []
for pid in sorted(seen - engine_ids, key=lambda i: name_of.get(i, i)):
    nm = name_of.get(pid, pid)
    if nm in allow:
        lines.append("ALLOWED\t%s\t%s" % (ws, nm))
        continue
    path, cur = [nm], pid
    while cur in parent:
        cur = parent[cur]
        path.append(name_of.get(cur, cur))
    lines.append("OFFENDER\t%s\t%s\t%s" % (ws, nm, " -> ".join(path)))
if not lines:
    lines.append("CLEAN\t%s\tonly %s itself reaches %s" % (ws, engine, engine))
print("\n".join(lines))
PY
)"
    rm -f "$meta_file"
    while IFS="$(printf '\t')" read -r kind wsname a b; do
      [[ -n "${kind:-}" ]] || continue
      case "$kind" in
        CLEAN)    ok   "A [$wsname]  $a" ;;
        ALLOWED)  ok   "A [$wsname]  $a depends on $ENGINE_PKG and is listed in $ALLOW_FILE" ;;
        OFFENDER) fail "A [$wsname]  $a depends on $ENGINE_PKG: $b" ;;
        *)        fail "A [$wsname]  $a" ;;
      esac
    done <<< "$report"
  done
fi

# ---------------------------------------------------------------------------
# B. apps/ios: no GPL crate, no Rust engine artifacts
# ---------------------------------------------------------------------------
# GPL crate names: the engine by construction (§3), plus any crate whose own
# LICENSE file or `license` field says GPL, so a second GPL crate is covered the
# day it appears.
GPL_PKGS=""
[[ -n "$ENGINE_PKG" ]] && GPL_PKGS="$ENGINE_PKG"
for c in crates/*/; do
  [[ -f "${c}Cargo.toml" ]] || continue
  is_gpl=0
  for lic in "${c}LICENSE" "${c}LICENSE.md" "${c}LICENSE.txt" "${c}COPYING"; do
    [[ -f "$lic" ]] || continue
    if head -40 "$lic" | grep -qiE 'GNU (Lesser |Library |Affero )?General Public License'; then
      is_gpl=1
    fi
  done
  if sed -n '/^\[package\]/,/^\[/p' "${c}Cargo.toml" | grep -qiE '^ *license *= *"[^"]*GPL'; then
    is_gpl=1
  fi
  if (( is_gpl )); then
    n="$(pkg_name "${c}Cargo.toml")"
    [[ -n "$n" ]] && GPL_PKGS="$GPL_PKGS $n"
  fi
done
# `grep -v` exits 1 on no match, and with `set -o pipefail` that would abort the
# whole script before check B ever ran -- i.e. a clean tree would look like a
# failure. `|| true` is load-bearing.
GPL_PKGS="$(printf '%s\n' "$GPL_PKGS" | tr ' ' '\n' | grep -v '^$' | sort -u | tr '\n' ' ' || true)"

if [[ ! -d apps/ios ]]; then
  ok "B  apps/ios does not exist in this checkout"
else
  ios_report="$(GPL_PKGS="$GPL_PKGS" ROOT="$ROOT" python3 - <<'PY'
import os, re, sys

root = os.environ["ROOT"]
gpl = [p for p in os.environ["GPL_PKGS"].split() if p]
ios = os.path.join(root, "apps", "ios")

# Build inputs only: everything Xcode or SwiftPM actually reads. README prose
# that says "no Rust helper" is not a violation, so .md is out -- and so are
# build products (.build, DerivedData), which are not repository content.
SKIP_DIRS = {".build", "DerivedData", "build", ".git", "target"}
NAMES = ("Package.swift", "Package.resolved", "project.yml", "generate-xcodeproj.py")
EXTS = (".pbxproj", ".xcconfig", ".entitlements", ".xcscheme")

inputs, links = [], []
for dirpath, dirnames, filenames in os.walk(ios):
    dirnames[:] = [d for d in dirnames if d not in SKIP_DIRS]
    for d in list(dirnames):
        p = os.path.join(dirpath, d)
        if os.path.islink(p):
            links.append(p)
            dirnames.remove(d)
    for f in filenames:
        p = os.path.join(dirpath, f)
        if os.path.islink(p):
            links.append(p)
        if f in NAMES or f.endswith(EXTS):
            inputs.append(p)
inputs.sort()
links.sort()

out = []
def emit(kind, *rest):
    out.append("\t".join([kind] + [str(r) for r in rest]))

def rel(p):
    return os.path.relpath(p, root)

def code_lines(path):
    """Numbered lines with whole-line comments dropped.

    Package.swift explains in a comment why it does NOT use .unsafeFlags. A
    check that cannot tell a comment from a declaration would call that a
    licence violation, which is how a gate becomes something people disable.
    """
    try:
        txt = open(path, encoding="utf-8", errors="replace").read()
    except OSError:
        return []
    res = []
    for i, line in enumerate(txt.splitlines(), 1):
        s = line.strip()
        if s.startswith(("//", "#", "*", "/*")):
            continue
        res.append((i, line))
    return res

emit("INFO", "%d Xcode/SwiftPM build inputs, %d symlinks under apps/ios" % (len(inputs), len(links)))

# B1: a GPL crate named in a build input or in any Swift source.
scan1 = list(inputs)
for dirpath, dirnames, filenames in os.walk(ios):
    dirnames[:] = [d for d in dirnames if d not in SKIP_DIRS]
    scan1 += [os.path.join(dirpath, f) for f in filenames if f.endswith(".swift")]
if not gpl:
    emit("OK", "B1", "no GPL-licensed crate exists in this checkout, so none can be referenced")
else:
    pats = set()
    for p in gpl:
        pats.add(p)
        pats.add(p.replace("-", "_"))
    hits = 0
    for path in sorted(set(scan1)):
        for n, line in code_lines(path):
            for pat in pats:
                if pat in line:
                    emit("FAIL", "B1", "%s:%d references the GPL crate: %s" % (rel(path), n, line.strip()[:120]))
                    hits += 1
    if not hits:
        emit("OK", "B1", "apps/ios references none of: %s" % " ".join(gpl))

# B2: the mechanisms by which a Swift target could link a Rust artifact. There
# is no legitimate use for any of them in an MIT companion that speaks
# nearby-v1 over the network, so declaring one is the violation.
B2 = [
    (re.compile(r"\.binaryTarget"), "SwiftPM binary target"),
    (re.compile(r"\.systemLibrary"), "SwiftPM system-library target"),
    (re.compile(r"linkedLibrary|linkerSetting"), "SwiftPM linker setting"),
    (re.compile(r"unsafeFlags"), "SwiftPM unsafeFlags"),
    (re.compile(r"\.xcframework"), "xcframework reference"),
    (re.compile(r"\.dylib|\.a\b(?!\w)", re.I), "native library file"),
    (re.compile(r"OTHER_LDFLAGS\s*=\s*[^;]*-[lL]"), "OTHER_LDFLAGS naming a library"),
    (re.compile(r"LIBRARY_SEARCH_PATHS\s*=\s*[^;]*[^\s\"';]"), "LIBRARY_SEARCH_PATHS"),
    (re.compile(r"libflashtex", re.I), "libflashtex"),
]
hits = 0
for path in inputs:
    for n, line in code_lines(path):
        for rx, what in B2:
            if rx.search(line):
                emit("FAIL", "B2", "%s:%d declares %s: %s" % (rel(path), n, what, line.strip()[:120]))
                hits += 1
if not hits:
    emit("OK", "B2", "no binary target, system library, xcframework, link flag or native library under apps/ios")

# B3: a build phase or helper that shells out to the Rust toolchain would link
# engine artifacts without any of B2 appearing anywhere.
B3 = re.compile(r"(^|[^\w.-])(cargo|rustc|rustup)([\s;\"']|$)|build-helpers\.sh|/target/(debug|release)")
scan3 = list(inputs)
for dirpath, dirnames, filenames in os.walk(ios):
    dirnames[:] = [d for d in dirnames if d not in SKIP_DIRS]
    scan3 += [os.path.join(dirpath, f) for f in filenames if f.endswith((".sh", ".py"))]
hits = 0
for path in sorted(set(scan3)):
    for n, line in code_lines(path):
        if B3.search(line):
            emit("FAIL", "B3", "%s:%d invokes the Rust toolchain: %s" % (rel(path), n, line.strip()[:120]))
            hits += 1
if not hits:
    emit("OK", "B3", "no apps/ios build input invokes cargo, rustc or a Rust target directory")

# B4: apps/ios legitimately symlinks three Mac-owned Swift source trees
# (Package.swift documents them). A symlink that resolved into crates/ would
# pull Rust sources -- one day engine sources -- straight into the iPad target.
hits = 0
for link in links:
    target = os.path.realpath(link)
    if target == os.path.realpath(os.path.join(root, "crates")) or target.startswith(
        os.path.realpath(os.path.join(root, "crates")) + os.sep
    ):
        emit("FAIL", "B4", "%s resolves into crates/ (%s)" % (rel(link), target))
        hits += 1
if not hits:
    emit("OK", "B4", "none of the %d symlinks under apps/ios resolves into crates/" % len(links))

print("\n".join(out))
PY
)"
  while IFS="$(printf '\t')" read -r kind a b; do
    [[ -n "${kind:-}" ]] || continue
    case "$kind" in
      INFO) info "B  $a" ;;
      OK)   ok   "$a  $b" ;;
      FAIL) fail "$a  $b" ;;
      *)    fail "B  unexpected output: $kind $a $b" ;;
    esac
  done <<< "$ios_report"
fi

# ---------------------------------------------------------------------------
# C. MIT crates and apps: no poppler, no MuPDF
# ---------------------------------------------------------------------------
C_WORKSPACES=""
[[ -f Cargo.toml ]] && C_WORKSPACES="."
for c in crates/*/; do
  [[ -f "${c}Cargo.lock" ]] || continue
  C_WORKSPACES="$C_WORKSPACES ${c%/}"
done
if [[ -z "$C_WORKSPACES" ]]; then
  ok "C  no cargo workspace in this checkout"
fi
for ws in $C_WORKSPACES; do
  meta=""
  if ! meta="$(cd "$ROOT/$ws" && cargo metadata --format-version 1 --locked 2>/dev/null)"; then
    if ! meta="$(cd "$ROOT/$ws" && cargo metadata --format-version 1 --offline 2>/dev/null)"; then
      fail "C [$ws]  cargo metadata failed; the dependency graph could not be checked"
      continue
    fi
  fi
  meta_file="$(mktemp "${TMPDIR:-/tmp}/flashtex-boundary-XXXXXX")"
  printf '%s' "$meta" > "$meta_file"
  report="$(GPL_PKGS="$GPL_PKGS" WS="$ws" python3 - "$meta_file" <<'PY'
import collections, json, os, re, sys

DENY = re.compile(r"^(poppler|mupdf)([-_]|$)", re.I)
gpl = set(os.environ["GPL_PKGS"].split())
ws = os.environ["WS"]
with open(sys.argv[1], encoding="utf-8") as fh:
    meta = json.load(fh)
pkgs = {p["id"]: p for p in meta["packages"]}
nodes = (meta.get("resolve") or {}).get("nodes") or []
if not nodes:
    print("ERROR\t%s\tcargo metadata returned no resolve graph" % ws)
    raise SystemExit(0)
fwd = {n["id"]: n.get("dependencies") or [d["pkg"] for d in n.get("deps", [])] for n in nodes}

def is_gpl(p):
    return p["name"] in gpl or "GPL" in (p.get("license") or "").upper()

lines = []
members = [m for m in meta.get("workspace_members", []) if m in pkgs and not is_gpl(pkgs[m])]
for m in sorted(members, key=lambda i: pkgs[i]["name"]):
    # Breadth-first from the MIT member; all dependency kinds count, as in check A.
    parent, seen, q = {}, {m}, collections.deque([m])
    while q:
        cur = q.popleft()
        for d in fwd.get(cur, ()):
            if d not in seen:
                seen.add(d)
                parent[d] = cur
                q.append(d)
    for hit in sorted(i for i in seen if i != m and DENY.match(pkgs.get(i, {}).get("name", ""))):
        path, cur = [pkgs[hit]["name"]], hit
        while cur in parent:
            cur = parent[cur]
            path.append(pkgs[cur]["name"])
        lines.append("OFFENDER\t%s\t%s\t%s" % (ws, pkgs[m]["name"], " <- ".join(path)))
    # build.rs that links the C library directly.
    mdir = os.path.dirname(pkgs[m]["manifest_path"])
    br = os.path.join(mdir, "build.rs")
    if os.path.isfile(br):
        for i, line in enumerate(open(br, encoding="utf-8", errors="replace"), 1):
            s = line.strip()
            if not s.startswith("//") and re.search(r"rustc-link-lib=(dylib=|static=)?(poppler|mupdf)", s, re.I):
                lines.append("OFFENDER\t%s\t%s\t%s:%d links %s" % (ws, pkgs[m]["name"], os.path.relpath(br), i, s[:100]))
if not lines:
    lines.append("CLEAN\t%s\t%d MIT crates; none reaches poppler or MuPDF" % (ws, len(members)))
print("\n".join(lines))
PY
)"
  rm -f "$meta_file"
  while IFS="$(printf '\t')" read -r kind wsname a b; do
    [[ -n "${kind:-}" ]] || continue
    case "$kind" in
      CLEAN)    ok   "C [$wsname]  $a" ;;
      OFFENDER) fail "C [$wsname]  MIT crate $a reaches a copyleft PDF renderer: $b" ;;
      *)        fail "C [$wsname]  $a" ;;
    esac
  done <<< "$report"
done

# C2: the MIT apps' build inputs must not link libpoppler or libmupdf either.
if [[ -d apps ]]; then
  app_hits="$(find apps \( -name .build -o -name DerivedData -o -name build -o -name target \) -prune -o \
      -type f \( -name Package.swift -o -name '*.pbxproj' -o -name '*.xcconfig' -o -name project.yml \) -print 2>/dev/null |
    while IFS= read -r f; do
      grep -niE '(lib)?(poppler|mupdf)' "$f" | grep -vE '^[0-9]+:[[:space:]]*(//|#)' | sed "s|^|$f:|" || true
    done)"
  if [[ -n "$app_hits" ]]; then
    while IFS= read -r h; do fail "C2  app build input names a copyleft PDF renderer: ${h:0:160}"; done <<< "$app_hits"
  else
    ok "C2  no Swift/Xcode build input under apps/ names poppler or MuPDF"
  fi
fi

# ---------------------------------------------------------------------------
# D. The Typst host (typst-host/): no GPL code, by link or by copy
# ---------------------------------------------------------------------------
# DESIGN.md §15.2/§15.9: typst-host/ is its own cargo workspace (MIT, linking
# the Apache-2.0 typst crates). It is checked from its committed Cargo.lock and
# Cargo.toml -- no `cargo metadata`, so this job never downloads Typst's ~300
# crates -- which is sound because its CI builds with --locked:
#   D1  no GPL package (the engine or any GPL crate here) and no poppler/MuPDF
#       in typst-host/Cargo.lock; every path dependency is the MIT
#       crates/display-list-v3 and nothing else;
#   D2  provenance: no file under typst-host/ is a byte copy of a file in a GPL
#       crate, and no Rust source there names the engine crate;
#   D3  every package in its `cargo metadata` graph (crates.io included) has
#       a licence, and none is GPL, LGPL or AGPL (allow file for exceptions).
if [[ ! -f typst-host/Cargo.toml ]]; then
  ok "D  typst-host/ does not exist in this checkout"
else
  gpl_dirs=""
  for c in crates/*/; do
    n="$(pkg_name "${c}Cargo.toml")"
    [[ -n "$n" ]] || continue
    for g in $GPL_PKGS; do
      [[ "$g" == "$n" ]] && gpl_dirs="$gpl_dirs ${c%/}"
    done
  done
  # Not inside $(...): bash 3.2 mis-parses quotes in a heredoc there.
  d_out="$(mktemp "${TMPDIR:-/tmp}/flashtex-boundary-XXXXXX")"
  GPL_PKGS="$GPL_PKGS" GPL_DIRS="$gpl_dirs" python3 - > "$d_out" <<'PY'
import hashlib, os, re

gpl = set(os.environ["GPL_PKGS"].split())
lines = []
lock_path = "typst-host/Cargo.lock"
if not os.path.isfile(lock_path):
    lines.append("D1\ttypst-host/Cargo.lock is missing: the workspace must commit its lockfile")
else:
    names = re.findall(r'^name = "([^"]+)"', open(lock_path, encoding="utf-8").read(), re.M)
    for n in sorted(set(names)):
        if n in gpl:
            lines.append("D1\ttypst-host/Cargo.lock contains the GPL package %s" % n)
        if re.match(r"^(poppler|mupdf)([-_]|$)", n, re.I):
            lines.append("D1\ttypst-host/Cargo.lock contains the copyleft PDF renderer %s" % n)
    if not lines:
        lines.append("OK\tD1  typst-host/Cargo.lock: %d packages, none GPL, none poppler/MuPDF" % len(set(names)))
allowed = {os.path.normpath("crates/display-list-v3")}
bad_paths = []
for m in re.finditer(r'path\s*=\s*"([^"]+)"', open("typst-host/Cargo.toml", encoding="utf-8").read()):
    p = os.path.normpath(os.path.join("typst-host", m.group(1)))
    if p.startswith("typst-host" + os.sep) or p == "typst-host":
        continue  # the package itself: its [lib] and [[bin]] paths
    if p not in allowed:
        bad_paths.append(p)
for p in bad_paths:
    lines.append("D1\ttypst-host/Cargo.toml has a path dependency on %s (only crates/display-list-v3 is allowed)" % p)
if not bad_paths:
    lines.append("OK\tD1  typst-host/Cargo.toml path dependencies: crates/display-list-v3 only")

def files(root):
    for dp, dn, fn in os.walk(root):
        dn[:] = [d for d in dn if d not in ("target", ".git")]
        for f in fn:
            p = os.path.join(dp, f)
            if os.path.isfile(p) and not os.path.islink(p):
                yield p

digest = lambda p: hashlib.sha256(open(p, "rb").read()).hexdigest()
gpl_hashes = {}
for d in os.environ["GPL_DIRS"].split():
    for p in files(d):
        if os.path.getsize(p) >= 64:  # tiny files match by chance
            gpl_hashes.setdefault(digest(p), p)
copies = 0
for p in files("typst-host"):
    if os.path.getsize(p) >= 64 and digest(p) in gpl_hashes:
        copies += 1
        lines.append("D2\t%s is a byte copy of the GPL file %s" % (p, gpl_hashes[digest(p)]))
    if p.endswith(".rs"):
        for i, line in enumerate(open(p, encoding="utf-8", errors="replace"), 1):
            s = line.split("//")[0]
            if re.search(r"\bflashtex_engine\b", s):
                copies += 1
                lines.append("D2\t%s:%d names the GPL engine crate" % (p, i))
if not copies:
    lines.append("OK\tD2  no typst-host file copies or names GPL code (%d GPL files compared)" % len(gpl_hashes))
print("\n".join(lines))
PY
  report="$(cat "$d_out")"
  rm -f "$d_out"
  while IFS="$(printf '\t')" read -r kind msg; do
    [[ -n "${kind:-}" ]] || continue
    case "$kind" in
      OK) ok "$msg" ;;
      *)  fail "$kind  $msg" ;;
    esac
  done <<< "$report"

  # D3: the licence of every package in the Typst host's graph, from cargo
  # metadata (D1 knows only this repository's crate names, so a GPL crate
  # from crates.io would pass it). Any GPL/LGPL/AGPL expression fails, and so
  # does a package with no `license` field, unless it is listed in
  # $TYPST_ALLOW_FILE as name@version. --offline first: once the crates are
  # in cargo's cache this needs no network.
  TYPST_ALLOW_FILE="scripts/license-boundary-typst-allow.txt"
  d_meta="$(mktemp "${TMPDIR:-/tmp}/flashtex-boundary-XXXXXX")"
  d3_ok=1
  if ! cargo metadata --manifest-path typst-host/Cargo.toml --format-version 1 --locked --offline > "$d_meta" 2>/dev/null; then
    # Always --locked: never rewrite typst-host/Cargo.lock. A stale lock (or
    # no network for crates not yet in cargo's cache) is a failure: the
    # Typst host's CI builds --locked too, so the lock is what ships.
    if ! cargo metadata --manifest-path typst-host/Cargo.toml --format-version 1 --locked > "$d_meta" 2> "$d_meta.err"; then
      why="$(grep -m1 -iE 'lock file|needs to be updated|--locked|error' "$d_meta.err" | cut -c1-160 || true)"
      fail "D3  cargo metadata --locked failed for typst-host (stale Cargo.lock or crates unavailable): ${why:-see cargo}"
      d3_ok=0
    fi
    rm -f "$d_meta.err"
  fi
  if (( d3_ok )); then
    d3_allow=""
    [[ -f "$TYPST_ALLOW_FILE" ]] && d3_allow="$(sed -e 's/#.*//' "$TYPST_ALLOW_FILE" | tr '[:space:]' ' ')"
    d3_out="$(mktemp "${TMPDIR:-/tmp}/flashtex-boundary-XXXXXX")"
    ALLOW="$d3_allow" python3 - "$d_meta" > "$d3_out" <<'PY'
import json, os, re, sys

allow = set(os.environ["ALLOW"].split())
meta = json.load(open(sys.argv[1], encoding="utf-8"))
copyleft = re.compile(r"(^|[^A-Za-z])(A|L)?GPL", re.I)
lines, n = [], 0
for p in sorted(meta["packages"], key=lambda p: (p["name"], p["version"])):
    n += 1
    who = "%s@%s" % (p["name"], p["version"])
    lic = p.get("license")
    if who in allow:
        continue
    if not lic:
        lines.append("D3\t%s has no licence field (license_file: %s); list it in the allow file only after reading the file" % (who, p.get("license_file")))
    elif copyleft.search(lic):
        lines.append("D3\t%s is licensed %r: no GPL, LGPL or AGPL code may link into the Typst host" % (who, lic))
if not lines:
    lines.append("OK\tD3  typst-host graph: %d packages, none GPL/LGPL/AGPL, every one licensed" % n)
print("\n".join(lines))
PY
    while IFS="$(printf '\t')" read -r kind msg; do
      [[ -n "${kind:-}" ]] || continue
      case "$kind" in
        OK) ok "$msg" ;;
        *)  fail "$kind  $msg" ;;
      esac
    done < "$d3_out"
    rm -f "$d3_out"
  fi
  rm -f "$d_meta"
fi

echo
if (( FAILURES )); then
  echo "licence boundary: $FAILURES violation(s); see DESIGN.md §3" >&2
  exit 1
fi
echo "licence boundary: clean (DESIGN.md §3)"
