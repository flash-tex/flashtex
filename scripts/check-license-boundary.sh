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
  # The root workspace globs crates/* but EXCLUDES three crates that keep their
  # own Cargo.lock (see Cargo.toml). Their graphs must be resolved separately or
  # the check has a blind spot exactly where the CLI lives.
  # crates/render-pipeline/vendor is a frozen snapshot, not a build target.
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

echo
if (( FAILURES )); then
  echo "licence boundary: $FAILURES violation(s); see DESIGN.md §3" >&2
  exit 1
fi
echo "licence boundary: clean (DESIGN.md §3)"
