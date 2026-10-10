#!/bin/bash
# Focus vs whole-document latency through the host's socket, as the app drives it.
# bench_focus.sh BOOKDIR CHAPTER(e.g. chapters/ch20) TAG
#   1. whole: fresh host (--s0-cache, --once), dl3-keys: open (cold, to convergence) + KEYS
#      letter keystrokes in CHAPTER on its middle page (--edit, --page)
#   2. focused: fresh host, out-focus seeded from 1's output (as the app does), dl3-keys
#      --includeonly CHAPTER: open (cold) + KEYS keystrokes in CHAPTER
#   3. toggles: one host (no --once), dl3-client: whole, focus, whole, focus (the app's
#      Focus / Show All: a job change each time, S0 from the disk cache after the first)
set -u
# WORK: a scratch folder (results go to $WORK/bench-TAG); INCR_BENCH_DIR/ENGINE: an engine staged by
# tools/incr-bench/mkeng.sh ENGINE (flashtex-host, dl3-keys, dl3-client and its pdflatex format).
S=${WORK:-/tmp/focus-bench}; mkdir -p "$S/tmp"
IB=${INCR_BENCH_DIR:-/tmp/incr-bench}; E=$IB/${ENGINE:-focus}
BOOK=$1; CH=$2; TAG=$3; KEYS=${KEYS:-30}
export TMPDIR=$S/tmp FLASHTEX_POOL=$E/pdftex.pool FLASHTEX_FORMATS=$IB/fmt-${ENGINE:-focus} SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1
R=$S/bench-$TAG; rm -rf "$R"; mkdir -p "$R"
K=/tmp/ffc-$$; mkdir -p $K  # short socket paths (sockaddr_un)
W=$R/work; cp -R "$BOOK" "$W"; rm -f "$W"/*.aux "$W"/*.toc "$W"/*.log "$W"/*.pdf "$W"/chapters/*.aux
mkdir -p "$R/out/chapters"
uptime > "$R/uptime.txt"

host() { # SOCK S0DIR LOG [--once]
  "$E/flashtex-host" --socket "$1" --s0-cache "$2" ${4:-} > "$3.out" 2> "$3.err" &
  HP=$!
  for i in $(seq 1 600); do grep -q listening "$3.out" 2>/dev/null && break; sleep 0.05; done
}

# the chapter's middle page in the whole document (from pdflatex-free data: its .aux after phase 1)
mid_page() { # AUXDIR
  python3 - "$1" "$CH" <<'EOF'
import re, sys
d, ch = sys.argv[1], sys.argv[2]
t = open(f"{d}/{ch}.aux").read()
first = int(re.search(r"\\newlabel\{ch:\d+\}\{\{\d+\}\{(\d+)\}", t).group(1))
last = int(re.search(r"\\setcounter\{page\}\{(\d+)\}", t).group(1)) - 1
print((first + last) // 2 - 1)  # 0-based index (book class: arabic page numbers from page 1)
EOF
}

# 1. whole document: open only, to get the .aux and the chapter's pages
host "$K/h1.sock" "$R/s0-whole" "$R/h1" --once
"$E/dl3-keys" --socket "$K/h1.sock" --root "$W" --main main.tex --output-dir "$R/out" --keys 0 > "$R/whole-open.jsonl"
wait $HP 2>/dev/null
P=$(mid_page "$R/out")
echo "chapter $CH middle page index (whole document): $P" | tee "$R/page.txt"
# whole document again: reopen from the stored S0 + keystrokes in the chapter
host "$K/h2.sock" "$R/s0-whole" "$R/h2" --once
"$E/dl3-keys" --socket "$K/h2.sock" --root "$W" --main main.tex --output-dir "$R/out" --keys $KEYS \
  --edit "$CH.tex" --page "$P" --where middle --gap-ms 300 > "$R/whole-keys.jsonl"
wait $HP 2>/dev/null

# 2. focused: out-focus from the whole document's output
rm -rf "$R/out-focus"; mkdir -p "$R/out-focus"; (cd "$R/out" && find . -type d -exec mkdir -p "$R/out-focus/{}" \; && find . -type f ! -name '*.pdf' ! -name '*.log' ! -name '*.synctex*' -exec cp {} "$R/out-focus/{}" \;)
host "$K/h3.sock" "$R/s0-focus" "$R/h3" --once
"$E/dl3-keys" --socket "$K/h3.sock" --root "$W" --main main.tex --output-dir "$R/out-focus" --includeonly "$CH" \
  --keys $KEYS --edit "$CH.tex" --at 0.6 --where middle --gap-ms 300 > "$R/focus-keys.jsonl"
wait $HP 2>/dev/null

# 3. toggles in one host (its S0 cache already holds both jobs' S0)
host "$K/h4.sock" "$R/s0-toggle" "$R/h4"
for step in whole focus whole focus; do
  if [ $step = focus ]; then
    "$E/dl3-client" --socket "$K/h4.sock" --root "$W" --main main.tex --output-dir "$R/out-focus" --includeonly "$CH" --quiet
  else
    "$E/dl3-client" --socket "$K/h4.sock" --root "$W" --main main.tex --output-dir "$R/out" --quiet
  fi | sed "s/^/$step /" >> "$R/toggle.jsonl"
done
kill $HP 2>/dev/null; wait $HP 2>/dev/null
echo "results in $R"
rm -rf "$K"
