# PARKED: PDF outlines (D13 freeze, 2026-09-29)

Parked when command moved to kabir-claude and DESIGN.md D13 froze the old
engine to fixes only. Not reviewed, not fully tested, no PR opened.

Done (3 commits on origin/main at the time):
- 569b4f4dd pdf: per-level outline open state (hyperref bookmarksopenlevel)
- 6bbf6f9ce render-pipeline: PDF outline from hyperref's bookmarks; re-pin vendor/pdf
- 502b068ea render-pipeline: page-top anchors, end-of-document bookmarks

Left: the full test matrix (both profiles, --no-fail-fast), a pdflatex outline-tree
comparison (titles, levels, target pages, open state) for article, report,
\pdfbookmark, \texorpdfstring, non-ASCII and no-hyperref cases, the perf
export.pdf digest changes, and review. The vendor/pdf re-pin will conflict with
P0-RETIRE-VENDOR; on revival, drop it and target the live crate, or re-home the
feature on the new engine's PDF backend (DESIGN.md).
