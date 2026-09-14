#!/usr/bin/env bash
# App-only acceptance for the bundled rooted TFM metrics (GH36).
#
# Usage: apps/mac/scripts/texmf-acceptance.sh [--app <FlashTeX.app>] [--evidence <dir>]
#                                            [--require-discovery]
#
# Runs the producer that is ACTUALLY INSIDE the bundle (Contents/MacOS/
# flashtex-render) with the host TeX installation excluded: `env -i` with
# PATH=/usr/bin:/bin (no texbin), no FLASHTEX_*/TEXMF* variables, HOME set to
# an empty temporary directory, and — beyond the environment — a sandbox-exec
# profile that denies every read under /usr/local/texlive, /Library/TeX and
# /usr/share/texmf|texlive, so a MacTeX on the build machine cannot conceal a
# packaging gap. Fixtures: the corrected 10 pt multi-document request
# (crates/preview-controller/benchmarks/display-helper-multidoc-10pt, verified
# capture), 12 pt text and 12 pt roman-math documents, and a 10 pt document
# with bold/italic text plus an 11 pt document (supplementary metrics:
# ec-lmbx10, ec-lmri10, ec-lmbxi10, rm-lmr10 and the 11 pt scaling of the
# 10/12 pt masters).
#
# Runs, each recorded with the exact environment and every diagnostic:
#   control    no FLASHTEX_TFM_DIRS (what the producer discovers on its own:
#              the DISCOVERY route; required with --require-discovery, which
#              a producer at or after render-pipeline 421a2049 satisfies)
#   env-direct FLASHTEX_TFM_DIRS as BundledMetrics.producerEnvironment sets it
#              for the directly attached worker: <app>/Contents/Resources/
#              texmf/fonts/tfm/public/lm
#   env-user   the same with an explicit user entry appended after the bundled
#              directory (user overrides preserved)
#   env-helper the bundled flashtex-preview-controller (file-backed project)
#              spawning the bundled producer with the env-direct environment
#   removed    env-direct again on a COPY of the bundle with ec-lmr10.tfm
#              deleted, expected to fail with explicit diagnostics; the
#              pinned verifier must refuse the copy (exit 1)
#   verifier   crates/rendering-core/tools/verify_bundle_resources.py on the
#              real bundle (exit 0 required)
# PASS requires zero missing-metric diagnostics in every env-* compile result,
# evidence in the removed run that the deletion itself was noticed, and
# verifier exit 0. "Missing-metric" is no longer a list written out here: it is
# read from crates/render-pipeline/src/fontdiag.rs via
# scripts/font_diagnostics.py, because the copy that used to live here named
# only tfm_missing/required_metrics_unavailable/font_unavailable and so scored
# ec_metrics_unavailable, font_outline_substituted and math_font_unavailable as
# clean -- this gate printed ok while the bundle had failed to provide a font
# resource. Widening the list can make a previously passing bundle fail here;
# that is the gate working, not a regression in the script. The
# control run is recorded, and required only with --require-discovery. Every
# run records `uptime` next to its result. No app window is opened; nothing is
# downloaded; only processes this script spawned are waited on.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MAC_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
REPO_ROOT="$(cd "$MAC_DIR/../.." && pwd)"
APP_DIR="$MAC_DIR/build/FlashTeX.app"
EVIDENCE_DIR=""
REQUIRE_DISCOVERY=0
while [[ $# -gt 0 ]]; do
  case "$1" in
    --app) APP_DIR="$2"; shift 2 ;;
    --evidence) EVIDENCE_DIR="$2"; shift 2 ;;
    --require-discovery) REQUIRE_DISCOVERY=1; shift ;;
    -h|--help) sed -n '2,40p' "${BASH_SOURCE[0]}"; exit 0 ;;
    *) echo "texmf-acceptance.sh: unknown argument: $1" >&2; exit 1 ;;
  esac
done
die() { echo "texmf-acceptance.sh: $*" >&2; exit 1; }

# Runs below change cwd to an empty home. Resolve caller-relative --app
# once so binaries and resource overrides still name the requested bundle.
[[ -d "$APP_DIR" ]] || die "app directory not found: $APP_DIR"
APP_DIR="$(cd "$APP_DIR" && pwd)"
RENDER="$APP_DIR/Contents/MacOS/flashtex-render"
CONTROLLER="$APP_DIR/Contents/MacOS/flashtex-preview-controller"
RESOURCES="$APP_DIR/Contents/Resources"
BUNDLED_TFM_DIR="$RESOURCES/texmf/fonts/tfm/public/lm"
VERIFIER="$REPO_ROOT/crates/rendering-core/tools/verify_bundle_resources.py"
[[ -x "$RENDER" ]] || die "no bundled producer at $RENDER (package with make-app.sh --render <flashtex-render>)"
[[ -d "$BUNDLED_TFM_DIR" ]] || die "no rooted metrics at $BUNDLED_TFM_DIR"
[[ -f "$VERIFIER" ]] || die "missing $VERIFIER"
command -v sandbox-exec >/dev/null 2>&1 || die "sandbox-exec not available"

WORK="$(mktemp -d "${TMPDIR:-/tmp}/flashtex-texmf-acceptance.XXXXXX")"
trap 'rm -rf "$WORK"' EXIT
[[ -n "$EVIDENCE_DIR" ]] || EVIDENCE_DIR="$WORK/evidence"
mkdir -p "$EVIDENCE_DIR" "$WORK/home"
EVIDENCE_DIR="$(cd "$EVIDENCE_DIR" && pwd)"
FIXTURES="$WORK/fixtures.jsonl"
DRIVER="$WORK/helper-driver.py"
PROFILE="$WORK/no-host-tex.sb"

# Byte-identical to the verified capture's producer input (requests
# preview-1/preview-2: article default 10 pt, main.tex + chapter.tex +
# refs.bib), then 12 pt text and 12 pt roman math (display-list-v2 negotiated
# like the shell does).
cat > "$FIXTURES" <<'JSONL'
{"protocol_version":1,"id":"preview-1","type":"compile","payload":{"project_id":"p","revision":1,"entry_path":"main.tex","documents":[{"path":"chapter.tex","text":"Chapter text with \\(a+b\\).\n"},{"path":"main.tex","text":"\\documentclass{article}\n\\begin{document}\nOffice AV fi.\\input{chapter}\n\\end{document}\n"},{"path":"refs.bib","text":"@article{sample, title={Example}, author={A. Author}, year={2026}}\n"}]}}
{"protocol_version":1,"id":"preview-2","type":"compile","payload":{"project_id":"p","revision":2,"entry_path":"main.tex","documents":[{"path":"chapter.tex","text":"Chapter text with \\(a+b\\).\n"},{"path":"main.tex","text":"\\documentclass{article}\n\\begin{document}\nOffice AV fi.\\input{chapter}\n\\end{document}\n"},{"path":"refs.bib","text":"@article{sample, title={Example}, author={A. Author}, year={2026}}\n"}],"layout_capabilities":["display-list-v2"]}}
{"protocol_version":1,"id":"text-12pt","type":"compile","payload":{"project_id":"p","revision":3,"entry_path":"main.tex","documents":[{"path":"main.tex","text":"\\documentclass[12pt]{article}\n\\begin{document}\nOffice AV fi. Twelve point text with ligatures: office, affine, fluffy.\n\\end{document}\n"}],"layout_capabilities":["display-list-v2"]}}
{"protocol_version":1,"id":"math-12pt","type":"compile","payload":{"project_id":"p","revision":4,"entry_path":"main.tex","documents":[{"path":"main.tex","text":"\\documentclass[12pt]{article}\n\\begin{document}\nBody $x^2 + y_1$ text. \\[ \\sum_{i=1}^{n} a_i = \\frac{1}{2} \\]\n\\end{document}\n"}],"layout_capabilities":["display-list-v2"]}}
{"protocol_version":1,"id":"styles-10pt","type":"compile","payload":{"project_id":"p","revision":5,"entry_path":"main.tex","documents":[{"path":"main.tex","text":"\\documentclass{article}\n\\begin{document}\nRegular \\textbf{bold} \\textit{italic} \\textbf{\\textit{bold italic}} with $x_i^2$ text.\n\\end{document}\n"}],"layout_capabilities":["display-list-v2"]}}
{"protocol_version":1,"id":"text-11pt","type":"compile","payload":{"project_id":"p","revision":6,"entry_path":"main.tex","documents":[{"path":"main.tex","text":"\\documentclass[11pt]{article}\n\\begin{document}\nEleven point \\textbf{bold} \\textit{italic} text with $a+b$.\n\\end{document}\n"}],"layout_capabilities":["display-list-v2"]}}
JSONL

cat > "$PROFILE" <<'SB'
(version 1)
(allow default)
(deny file-read* (subpath "/usr/local/texlive"))
(deny file-read* (subpath "/Library/TeX"))
(deny file-read* (subpath "/usr/share/texmf"))
(deny file-read* (subpath "/usr/share/texlive"))
SB

# Drives the bundled preview controller over its JSON Lines protocol for the
# 10 pt multi-document project: file-backed startup, one compile, close.
# Prints the preview update's compile result as one compile_result line.
cat > "$DRIVER" <<'PY'
import json, os, subprocess, sys, time
controller, producer, project, ledger = sys.argv[1:5]
fixtures = [json.loads(l) for l in open(sys.argv[5]) if l.strip()]
docs = fixtures[1]["payload"]["documents"]
os.makedirs(project, exist_ok=True); os.makedirs(ledger, exist_ok=True)
for d in docs:
    open(os.path.join(project, d["path"]), "w").write(d["text"])
cfg = os.path.join(project, "..", "controller-config.json")
json.dump({"session_id": "texmf", "project_id": "p", "entry_path": "main.tex",
           "project_root": project, "private_ledger_root": ledger,
           "compiler_path": producer}, open(cfg, "w"))
p = subprocess.Popen([controller, cfg], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
def send(t, payload, i):
    p.stdin.write(json.dumps({"protocol_version": 1, "session_id": "texmf", "id": i, "type": t, "payload": payload}) + "\n"); p.stdin.flush()
frames, result, deadline = [], None, time.time() + 60
line = p.stdout.readline(); frames.append(line)
assert json.loads(line)["type"] == "ready", line
send("configure_layout", {"layout_capabilities": ["display-list-v2"], "renderer_support_confirmed": True}, "cfg")
send("compile", {}, "c1")
while time.time() < deadline:
    line = p.stdout.readline()
    if not line: break
    frames.append(line); f = json.loads(line)
    if f.get("type") == "update" and f["payload"].get("kind") == "preview":
        result = f["payload"]["result"]; break
send("close", {}, "x")
try: p.wait(timeout=10)
except subprocess.TimeoutExpired: p.kill(); p.wait()
open(sys.argv[6], "w").writelines(frames)
if result is None: print(json.dumps({"error": "no preview update", "frames": len(frames)})); sys.exit(1)
print(json.dumps(result))
PY

# Which diagnostic codes mean "the bundle did not provide a font resource".
# Derived, never hand-written: crates/render-pipeline/src/fontdiag.rs is the
# source of truth and crates/render-pipeline/tests/fontdiag.rs fails if a code
# lands in typeset.rs that nobody classified.
#
# This list used to be three codes hard-coded here, omitting
# ec_metrics_unavailable, font_outline_substituted and math_font_unavailable.
# A run emitting only those printed "0 missing-metric diagnostics = ok" while
# a real substitution had happened -- on the acceptance gate for the shipped
# app's font discovery. `die` on failure: a gate that cannot read its own
# criteria must stop, not count zero.
MISSING_CODES="$(python3 "$REPO_ROOT/scripts/font_diagnostics.py" --view substitution --format regex)" \
  || die "cannot read the font-diagnostic classification (scripts/font_diagnostics.py)"
[[ -n "$MISSING_CODES" ]] || die "font-diagnostic classification came back empty"
echo "==> missing-metric codes counted: $MISSING_CODES"
# Counts missing-metric diagnostics across every compile_result line in $1.
count_missing() { { grep -oE "\"code\":\"($MISSING_CODES)\"" "$1" || true; } | wc -l | tr -d ' '; }
results_in() { grep -c '"type":"compile_result"' "$1" || true; }

PASS=0; FAIL=0; LINES=()
ok()  { PASS=$((PASS + 1)); echo "  ok   $1"; LINES+=("- ok: $1"); }
bad() { FAIL=$((FAIL + 1)); echo "  FAIL $1" >&2; LINES+=("- FAIL: $1"); }
info() { echo "  info $1"; LINES+=("- info: $1"); }

# run_producer <name> <producer> [VAR=value ...]: hostile base environment
# plus the given variables, host TeX denied by the sandbox profile.
run_producer() {
  local name="$1" producer="$2"; shift 2
  local out="$EVIDENCE_DIR/$name.output.jsonl" err="$EVIDENCE_DIR/$name.stderr.txt" envf="$EVIDENCE_DIR/$name.env.txt"
  printf 'PATH=/usr/bin:/bin\nHOME=%s\n' "$WORK/home" > "$envf"
  printf '%s\n' "$@" >> "$envf"
  ( cd "$WORK/home" && env -i PATH=/usr/bin:/bin HOME="$WORK/home" "$@" \
      sandbox-exec -f "$PROFILE" "$producer" < "$FIXTURES" > "$out" 2> "$err" ) || echo "exit $?" >> "$err"
  LINES+=("- uptime after $name: $(uptime | sed 's/^ *//')")
}

echo "==> App: $APP_DIR"
echo "    producer sha256 $(shasum -a 256 "$RENDER" | awk '{print $1}')"
uptime | sed 's/^/    /'
LINES+=("- app: $APP_DIR" "- producer sha256: $(shasum -a 256 "$RENDER" | awk '{print $1}')" "- uptime: $(uptime)")
LINES+=("- producer component (components.json): $(python3 -c "import json,sys; r=json.load(open(sys.argv[1]))['render']; print('source_path', r['source_path'], 'git_sha', r['git_sha'], 'sha256', r['sha256'])" "$RESOURCES/components.json" 2>/dev/null || echo unavailable)")
for f in "$BUNDLED_TFM_DIR"/*.tfm "$RESOURCES/texmf/doc/fonts/lm/GUST-FONT-LICENSE.TXT"; do
  LINES+=("- resource sha256 $(shasum -a 256 "$f" | awk '{print $1}') ${f#$APP_DIR/}")
done

echo "==> control: bundled producer, host TeX denied, no FLASHTEX_TFM_DIRS"
run_producer control "$RENDER"
n="$(count_missing "$EVIDENCE_DIR/control.output.jsonl")"; r="$(results_in "$EVIDENCE_DIR/control.output.jsonl")"
if [[ "$REQUIRE_DISCOVERY" -eq 1 ]]; then
  if [[ "$r" -eq 6 && "$n" -eq 0 ]]; then ok "control/discovery route: 6/6 compile results, 0 missing-metric diagnostics (FLASHTEX_TFM_DIRS unset; producer found Contents/Resources/texmf itself)"; else bad "control/discovery route: $r/6 results, $n missing-metric diagnostics"; fi
else
  info "control/discovery route: $r results, $n missing-metric diagnostics (recorded, not required without --require-discovery)"
fi

echo "==> env-direct: FLASHTEX_TFM_DIRS=$BUNDLED_TFM_DIR"
run_producer env-direct "$RENDER" "FLASHTEX_TFM_DIRS=$BUNDLED_TFM_DIR"
n="$(count_missing "$EVIDENCE_DIR/env-direct.output.jsonl")"; r="$(results_in "$EVIDENCE_DIR/env-direct.output.jsonl")"
if [[ "$r" -eq 6 && "$n" -eq 0 ]]; then ok "env-direct: 6/6 compile results, 0 missing-metric diagnostics"; else bad "env-direct: $r/6 results, $n missing-metric diagnostics"; fi

echo "==> env-user: bundled directory first, explicit user entry preserved after it"
run_producer env-user "$RENDER" "FLASHTEX_TFM_DIRS=$BUNDLED_TFM_DIR:$WORK/home/user-tfm"
n="$(count_missing "$EVIDENCE_DIR/env-user.output.jsonl")"; r="$(results_in "$EVIDENCE_DIR/env-user.output.jsonl")"
if [[ "$r" -eq 6 && "$n" -eq 0 ]]; then ok "env-user: 6/6 compile results, 0 missing-metric diagnostics"; else bad "env-user: $r/6 results, $n missing-metric diagnostics"; fi

if [[ -x "$CONTROLLER" ]]; then
  echo "==> env-helper: bundled flashtex-preview-controller spawning the bundled producer"
  hout="$EVIDENCE_DIR/env-helper.output.jsonl"
  printf 'PATH=/usr/bin:/bin\nHOME=%s\nFLASHTEX_TFM_DIRS=%s\n' "$WORK/home" "$BUNDLED_TFM_DIR" > "$EVIDENCE_DIR/env-helper.env.txt"
  if ( cd "$WORK/home" && env -i PATH=/usr/bin:/bin HOME="$WORK/home" "FLASHTEX_TFM_DIRS=$BUNDLED_TFM_DIR" \
        sandbox-exec -f "$PROFILE" /usr/bin/python3 "$DRIVER" "$CONTROLLER" "$RENDER" "$WORK/helper/project" "$WORK/helper/ledger" "$FIXTURES" "$EVIDENCE_DIR/env-helper.frames.jsonl" > "$hout" 2> "$EVIDENCE_DIR/env-helper.stderr.txt" ); then
    n="$(count_missing "$hout")"
    if [[ "$n" -eq 0 ]] && grep -q '"pages"' "$hout"; then ok "env-helper: preview update received, 0 missing-metric diagnostics"; else bad "env-helper: $n missing-metric diagnostics"; fi
  else
    bad "env-helper: no preview update ($(tail -1 "$EVIDENCE_DIR/env-helper.stderr.txt" | cut -c1-160))"
  fi
else
  info "env-helper skipped: no bundled flashtex-preview-controller"
fi

echo "==> removed: ec-lmr10.tfm deleted from a copy of the bundle"
COPY="$WORK/removed/FlashTeX.app"
mkdir -p "$WORK/removed" && ditto "$APP_DIR" "$COPY"
rm "$COPY/Contents/Resources/texmf/fonts/tfm/public/lm/ec-lmr10.tfm"
run_producer removed "$COPY/Contents/MacOS/flashtex-render" "FLASHTEX_TFM_DIRS=$COPY/Contents/Resources/texmf/fonts/tfm/public/lm"
n="$(count_missing "$EVIDENCE_DIR/removed.output.jsonl")"
# Differential, not absolute. `n > 0` alone does not show the deletion was
# noticed -- an intact bundle can already emit missing-metric diagnostics for
# unrelated faces, and `grep ec-lmr10.tfm` matches the intact run too, because
# the ec_metrics_unavailable message says "... ec-lmr10.tfm used". Measured on
# an intact tree: the deletion's own evidence is a `tfm_missing` naming
# ec-lmr10.tfm that the intact run does not produce. Require exactly that.
deleted_tfm() { { grep -oE '"code":"tfm_missing","message":"[^"]*ec-lmr10\.tfm[^"]*"' "$1" || true; } | wc -l | tr -d ' '; }
after="$(deleted_tfm "$EVIDENCE_DIR/removed.output.jsonl")"
before="$(deleted_tfm "$EVIDENCE_DIR/env-direct.output.jsonl")"
if [[ "$after" -gt 0 && "$before" -eq 0 ]]; then
  ok "removed: deleting ec-lmr10.tfm produces $after tfm_missing diagnostic(s) naming it that the intact bundle does not ($n missing-metric diagnostics in total)"
elif [[ "$before" -gt 0 ]]; then
  bad "removed: the INTACT bundle already reports ec-lmr10.tfm missing ($before), so this check proves nothing about the deletion"
else
  bad "removed: deleting ec-lmr10.tfm produced no tfm_missing naming it (total missing-metric diagnostics: $n)"
fi
rc=0; python3 "$VERIFIER" "$COPY/Contents/Resources" > "$EVIDENCE_DIR/removed.verifier.json" 2>&1 || rc=$?
if [[ "$rc" -eq 1 ]] && grep -q '"missing"' "$EVIDENCE_DIR/removed.verifier.json"; then ok "removed: verifier refuses the copy (exit 1, ec-lmr10.tfm missing)"; else bad "removed: verifier exit $rc"; fi

echo "==> verifier on the real bundle"
rc=0; python3 "$VERIFIER" "$RESOURCES" > "$EVIDENCE_DIR/verifier.json" 2>&1 || rc=$?
if [[ "$rc" -eq 0 ]]; then ok "verify_bundle_resources.py $RESOURCES: exit 0 (9 pinned resources verified)"; else bad "verify_bundle_resources.py exit $rc"; fi
if python3 -c "import json,sys; c=json.load(open(sys.argv[1])); r=c['resources']; assert r['status']=='verified' and len(r['resources'])==9" "$RESOURCES/components.json" 2>/dev/null; then
  ok "components.json carries the 9 verified resource hashes + $(python3 -c "import json,sys; print(len(json.load(open(sys.argv[1]))['resources'].get('supplementary',{})))" "$RESOURCES/components.json") supplementary"
else
  bad "components.json lacks a verified resources entry"
fi

echo "==> texmf-acceptance: $PASS ok, $FAIL failed"
{
  echo "# texmf-acceptance.sh — $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo
  echo "Host TeX excluded by env -i (PATH=/usr/bin:/bin, empty HOME, no FLASHTEX_*/TEXMF*) and a sandbox-exec profile denying reads under /usr/local/texlive, /Library/TeX, /usr/share/texmf, /usr/share/texlive."
  echo
  printf '%s\n' "${LINES[@]}"
  echo
  echo "Result: $PASS ok, $FAIL failed"
} > "$EVIDENCE_DIR/README.md"
echo "==> Evidence in $EVIDENCE_DIR"
[[ "$FAIL" -eq 0 ]]
