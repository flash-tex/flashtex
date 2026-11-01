#!/usr/bin/env bash
# p5-hosted-t4.sh score A-B | merge: T4 (nightly-5k, DESIGN §8) split over
# GitHub-hosted jobs, for p5-scoreboard.yml's hosted route.
#
#   score A-B  score shards A..B of $T4_SHARDS (tools/parity/nightly.py run
#              --shard-range), into $FLASHTEX_NIGHTLY_HOME/runs/<run key>/;
#              the job uploads that directory
#   merge      with every job's shard directories put back under
#              $FLASHTEX_NIGHTLY_HOME/runs/, merge them into one T4 summary in
#              $T4_OUT (summary.json, documents.json, summary.md), as
#              nightly.py does after a run on one host. It scores nothing: a
#              shard that is missing or has unmeasured documents stays so, and
#              the summary says so (the board reads T4 partial).
#
# ONE RUN KEY FOR EVERY JOB. nightly.py keys a run by the engine binary, the
# formats and pool (by content), the harness, the manifests, the oracle (pdftex
# and texlive.tlpdb by content, and their paths), the host id and the settings
# below, and merges only the shards of one key. Every job therefore uses the one
# build (ENGINE_DIR, at the same path), the one pinned TeX Live image (at the
# same path), these settings, and the same host id: a hosted VM's
# /etc/machine-id is its own, so it is replaced by one id for the whole hosted
# pool and image (P5_HOST_ID), and only on a GitHub-hosted runner (never on a
# machine whose identity a ratchet baseline is bound to).
#
# As corpus-t4-mac.yml: nightly-5k only (the board measures arxiv, templates and
# packages itself), --raster none (the board leaves L4 out), no ratchet (a
# baseline is per host and recorded by hand; none exists for the hosted pool).
#
# Environment: ENGINE_DIR (flashtex-initex, pdftex.pool, pdflatex.fmt,
# pdftex.fmt), FLASHTEX_NIGHTLY_HOME, P5_HOST_ID, T4_SHARDS, T4_OUT (merge), JOBS
# (score; default nproc). TeX Live 2026 first on PATH.
set -euo pipefail

mode="${1:?usage: p5-hosted-t4.sh score A-B | merge}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
: "${ENGINE_DIR:?}" "${FLASHTEX_NIGHTLY_HOME:?}" "${P5_HOST_ID:?}" "${T4_SHARDS:?}"
[[ ${RUNNER_ENVIRONMENT:-} == github-hosted ]] ||
  { echo "::error::p5-hosted-t4.sh replaces /etc/machine-id: GitHub-hosted runners only" >&2; exit 2; }
[[ $P5_HOST_ID =~ ^[0-9a-f]{32}$ ]] || { echo "::error::P5_HOST_ID is not 32 hex digits" >&2; exit 2; }
if [[ "$(cat /etc/machine-id 2>/dev/null)" != "$P5_HOST_ID" ]]; then
  echo "$P5_HOST_ID" | sudo tee /etc/machine-id >/dev/null
fi

pdftex="$(command -v pdftex)" || { echo "::error::no pdftex on PATH" >&2; exit 1; }
texbin="$(dirname "$pdftex")"
texmf="$(kpsewhich -var-value TEXMFDIST)"
# Every setting that enters the run key, once, for both modes.
run=(python3 "$ROOT/tools/parity/nightly.py" run --shards "$T4_SHARDS" --tier nightly-5k
     --pt1-sample nightly-5k=0.05 --pt1-max-log-mb 64
     --engine "$ENGINE_DIR/flashtex-initex"
     --engine-env "FLASHTEX_FORMATS=$ENGINE_DIR" --engine-env "FLASHTEX_POOL=$ENGINE_DIR/pdftex.pool"
     --texbin "$texbin" --oracle-pdftex "$pdftex")
parity=(-- --texmf "$texmf" --raster none)

case "$mode" in
  score)
    range="${2:?usage: p5-hosted-t4.sh score A-B}"
    jobs="${JOBS:-$(nproc)}"
    # 70 % of this VM's memory (it runs nothing else); nightly.py refuses a -j over it
    budget="$(awk '/^MemTotal:/ { printf "%.1f", $2 * 0.7 / 1048576 }' /proc/meminfo)"
    echo "T4 shards $range of $T4_SHARDS: $jobs workers, memory budget $budget GiB, host id ${P5_HOST_ID:0:12}"
    # FETCH FIRST, LOUDLY. The shards' arXiv e-prints are downloaded before any
    # scoring (corpus.py fetch: retried with backoff, each verified against its
    # pinned sha256 before it is cached). A document still missing after the
    # retries is an infrastructure failure: the shards are scored anyway (the
    # board reads that document missing) and the job then FAILS, so a cut
    # download is never mistaken for a quiet partial T4.
    shard_args=()
    for k in $(seq "${range%-*}" "${range#*-}"); do shard_args+=(--shard "$k/$T4_SHARDS"); done
    fetch_rc=0
    echo "::group::fetch the e-prints of shards $range"
    python3 "$ROOT/tools/parity/corpus.py" fetch --manifest "$ROOT/tools/parity/corpus/nightly-5k.json" \
      "${shard_args[@]}" 2>"$RUNNER_TEMP/t4-fetch-errors.txt" || fetch_rc=$?
    echo "::endgroup::"
    cat "$RUNNER_TEMP/t4-fetch-errors.txt" >&2
    if [[ $fetch_rc -ne 0 ]]; then
      while IFS= read -r line; do echo "::error::T4 fetch (infrastructure):$line"; done \
        < <(grep -E '^  nightly-5k/' "$RUNNER_TEMP/t4-fetch-errors.txt" || true)
    fi
    rc=0
    "${run[@]}" --shard-range "$range" -j "$jobs" --memory-budget-gib "$budget" \
      --out "$RUNNER_TEMP/t4-part" "${parity[@]}" || rc=$?
    # the part's own summary counts every other shard missing (exit 4): only a
    # harness or usage failure fails the job; the merge says what is missing
    rm -rf "$FLASHTEX_NIGHTLY_HOME/work"
    [[ $rc -eq 0 || $rc -eq 4 ]] || exit $rc
    for k in $(seq "${range%-*}" "${range#*-}"); do
      d=$(printf 'shard-%03d' "$k")
      compgen -G "$FLASHTEX_NIGHTLY_HOME/runs/*/$d/done.json" >/dev/null ||
        echo "::warning::T4 shard $k did not finish (its documents read missing on the board)"
    done
    if [[ $fetch_rc -ne 0 ]]; then
      echo "::error::T4 shards $range: e-prints could not be fetched and verified after retries (corpus.py fetch exit $fetch_rc); infrastructure, not parity: re-run this job" >&2
      exit 3
    fi
    ;;
  merge)
    : "${T4_OUT:?}"
    keys=("$FLASHTEX_NIGHTLY_HOME"/runs/*/)
    [[ ${#keys[@]} -eq 1 && -d ${keys[0]} ]] ||
      { echo "::error::expected the shards of one run key, found: ${keys[*]##*/runs/}" >&2; exit 1; }
    want="$(basename "${keys[0]}")"
    echo "merging $(compgen -G "${keys[0]}shard-*/done.json" | wc -l) of $T4_SHARDS shards of run $want"
    # A deadline already passed (negative): nightly.py skips every finished shard and
    # stops at the first one it would have to score, so it scores nothing and only merges.
    rc=0
    "${run[@]}" -j 1 --deadline-minutes=-1 --out "$T4_OUT" "${parity[@]}" || rc=$?
    [[ -f $T4_OUT/summary.json ]] || { echo "::error::nightly.py wrote no T4 summary (exit $rc)" >&2; exit 1; }
    got="$(python3 -c 'import json, sys; print(json.load(open(sys.argv[1]))["run_key"])' "$T4_OUT/summary.json")"
    [[ $got == "$want" ]] ||
      { echo "::error::the merge's run key $got is not the shards' $want (another engine, oracle or setting)" >&2; exit 1; }
    [[ $rc -eq 0 || $rc -eq 4 ]] || exit $rc
    ;;
  *) echo "usage: p5-hosted-t4.sh score A-B | merge" >&2; exit 2 ;;
esac
