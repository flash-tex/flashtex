#!/usr/bin/env bash
# report-only.sh WHAT RC [WHY]: a report-only step's tool exited RC.
#
#   some-tool ... 2>&1 | tee "$OUT/x.txt" || scripts/ci/report-only.sh x $? "why it may fail"
#
# A report-only step does not gate (Commander ruling on #2116: report-only
# workflows stay report-only unless DESIGN names them as a gate), but it must
# not pass silently either: this writes a ::warning:: annotation and a line in
# the job summary, then exits 0.
what=${1:-a report-only tool}
rc=${2:-?}
why=${3:-}
msg="$what exited $rc: report-only, not a gate${why:+ ($why)}"
echo "::warning::$msg"
if [ -n "${GITHUB_STEP_SUMMARY:-}" ]; then
  echo "- **report-only failure:** $msg" >> "$GITHUB_STEP_SUMMARY"
fi
exit 0
