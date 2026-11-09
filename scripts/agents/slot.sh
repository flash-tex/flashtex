#!/usr/bin/env bash
# slot.sh: shared build/test slots with owner files, stale-slot breaking and a
# priority queue. Portable to macOS (bash 3.2, BSD userland) and NixOS (GNU).
#
# Usage:
#   slot.sh acquire <pool> <lane> [--max-minutes N] [--priority P] [--pid PID]
#                                 [--wait-minutes W]
#   slot.sh run     <pool> <lane> [same options] -- <command> [args...]
#   slot.sh release <pool> <lane> [<slot-name>]
#   slot.sh status  [<pool>]
#
# Pools:
#   mac-build  3 slots on this Mac: mac-build.lock, mac-build.lock2, mac-build.lock3
#   pc         4 slots on the NixOS PC (run over `ssh nixos`): pc-slot1 .. pc-slot4
# The slot names are the ones lanes already mkdir by hand, so hand-made locks and
# slot.sh share the same pool.
#
# A slot is a directory under $SLOT_ROOT (default ~/.cache/flashtex-agents/commander)
# holding an `owner` file of key=value lines: lane, pid, host, start, start_utc,
# max_minutes, priority.
#
# Breaking a slot (logged to $SLOT_ROOT/slot-breaks.log). A slot is broken only when
#   (a) its recorded pid is on this host and is not running; or
#   (b) it is past max_minutes + 10 and its pid cannot be seen running: the pid lives
#       on another host (e.g. a PC slot taken from the Mac), or the slot has no
#       owner file (a hand-made lock; its age comes from the directory mtime and
#       max_minutes defaults to $SLOT_DEFAULT_MAX).
# A slot whose pid is running on this host is never broken, however old.
#
# Queue: waiters drop a ticket in $SLOT_ROOT/<pool>.queue/. Tickets sort by priority
# (higher first), then by arrival time (earlier first). A waiter takes a free slot
# only when fewer live tickets sort ahead of it than there are free slots. Waiters
# refresh their ticket every poll; a ticket whose pid is dead (same host) or that
# has not been refreshed for 5 minutes (other host) is removed.
#
# Pid to record: `acquire` records the pid of its caller ($PPID) unless --pid is
# given. An agent whose every shell command runs in a fresh shell must use `run`
# (the slot is held by slot.sh itself for the command's lifetime and released on
# exit) or pass --pid of a process that lives as long as the work.
# For the pc pool `run` executes the command on the PC, so the pid is checkable.
#
# Environment: SLOT_ROOT, SLOT_PC_HOST (default nixos; `local` = no ssh),
# SLOT_LOCAL=1 (force local), SLOT_REMOTE_ROOT (SLOT_ROOT on the PC, default its own), SLOT_POLL (seconds, default 15),
# SLOT_DEFAULT_MAX (minutes, default 60).
set -u

SELF=$0
CMD=${1:-}
[ $# -gt 0 ] && shift

SLOT_ROOT=${SLOT_ROOT:-$HOME/.cache/flashtex-agents/commander}
SLOT_POLL=${SLOT_POLL:-15}
SLOT_DEFAULT_MAX=${SLOT_DEFAULT_MAX:-60}
SLOT_PC_HOST=${SLOT_PC_HOST:-nixos}
HOST=$(uname -n)
GRACE_MIN=10
TICKET_STALE_SEC=300

die() { echo "slot.sh: $*" >&2; exit 2; }
now() { date +%s; }
utc() { date -u +%Y-%m-%dT%H:%M:%SZ; }

pool_slots() {
  case $1 in
    mac-build) echo "mac-build.lock mac-build.lock2 mac-build.lock3" ;;
    pc) echo "pc-slot1 pc-slot2 pc-slot3 pc-slot4" ;;
    *) die "unknown pool '$1' (mac-build or pc)" ;;
  esac
}

mtime() { # portable mtime in epoch seconds
  stat -c %Y "$1" 2>/dev/null || stat -f %m "$1" 2>/dev/null || echo 0
}

kv() { # kv <file> <key>
  [ -f "$1" ] || return 0
  sed -n "s/^$2=//p" "$1" | head -n 1
}

pid_alive() { # pid_alive <pid>
  case $1 in ''|*[!0-9]*) return 1 ;; esac
  kill -0 "$1" 2>/dev/null && return 0
  ps -p "$1" >/dev/null 2>&1
}

# Is the pool handled on another host?
pool_remote() {
  [ "$1" = pc ] || return 1
  [ "${SLOT_LOCAL:-}" = 1 ] && return 1
  [ "$SLOT_PC_HOST" = local ] && return 1
  return 0
}

run_remote() { # run this script on the PC with the given args
  b64=$(base64 < "$SELF" | tr -d '\n')
  args=""
  for a in "$@"; do args="$args $(printf '%q' "$a")"; done
  rroot=""
  [ -n "${SLOT_REMOTE_ROOT:-}" ] && rroot="SLOT_ROOT=$(printf '%q' "$SLOT_REMOTE_ROOT")"
  ssh "$SLOT_PC_HOST" "$rroot SLOT_LOCAL=1 SLOT_CALLER_HOST=$(printf '%q' "$HOST") SLOT_CALLER_PID=${CALLER_PID:-} SLOT_POLL=$SLOT_POLL bash -c \"\$(echo $b64 | base64 -d)\" slot.sh $args"
}

# slot_state <dir>: prints "free" | "held <reason>" | "stale <reason>"
slot_state() {
  d=$1
  [ -d "$d" ] || { echo free; return; }
  o=$d/owner
  pid=$(kv "$o" pid); host=$(kv "$o" host); start=$(kv "$o" start); max=$(kv "$o" max_minutes)
  case $max in ''|*[!0-9]*) max=$SLOT_DEFAULT_MAX ;; esac
  if [ -z "$pid" ] || [ -z "$start" ]; then
    start=$(mtime "$d"); kind=no-owner-file
  else
    kind=pid
  fi
  age=$(( $(now) - start ))
  limit=$(( (max + GRACE_MIN) * 60 ))
  if [ "$kind" = pid ] && [ "$host" = "$HOST" ]; then
    if pid_alive "$pid"; then echo "held pid $pid alive, age ${age}s"; else echo "stale pid $pid on $host is dead"; fi
    return
  fi
  if [ "$age" -gt "$limit" ]; then
    echo "stale $kind (host ${host:-?}, pid ${pid:-?} not visible) age ${age}s > max ${max}m + ${GRACE_MIN}m"
  else
    echo "held $kind (host ${host:-?}, pid ${pid:-?} not visible) age ${age}s within max ${max}m + ${GRACE_MIN}m"
  fi
}

break_slot() { # break_slot <dir> <reason>
  d=$1; reason=$2
  before=$(cat "$d/owner" 2>/dev/null)
  tmp="$d.broken.$$.$(now)"
  mv "$d" "$tmp" 2>/dev/null || return 1
  after=$(cat "$tmp/owner" 2>/dev/null)
  if [ "$before" != "$after" ]; then # someone re-took it between check and mv
    mv "$tmp" "$d" 2>/dev/null || rm -rf "$tmp"
    return 1
  fi
  printf '%s broke %s by %s@%s pid %s: %s | owner: %s\n' "$(utc)" "$(basename "$d")" \
    "${LANE:-?}" "$HOST" "$$" "$reason" "$(echo "$before" | tr '\n' ' ')" >> "$SLOT_ROOT/slot-breaks.log"
  echo "slot.sh: broke $(basename "$d"): $reason" >&2
  rm -rf "$tmp"
}

sweep_stale() { # sweep_stale <pool>
  for s in $(pool_slots "$1"); do
    st=$(slot_state "$SLOT_ROOT/$s")
    case $st in stale*) break_slot "$SLOT_ROOT/$s" "${st#stale }" ;; esac
  done
}

free_count() {
  n=0
  for s in $(pool_slots "$1"); do [ -d "$SLOT_ROOT/$s" ] || n=$((n + 1)); done
  echo $n
}

# Tickets: <qdir>/<99999-prio, 5 digits>-<epoch, 10 digits>-<host>-<pid>-<lane>
ticket_live() { # ticket_live <file>
  tf=$1
  th=$(kv "$tf" host); tp=$(kv "$tf" pid)
  if [ "$th" = "$HOST" ]; then pid_alive "$tp"; return; fi
  [ $(( $(now) - $(mtime "$tf") )) -le $TICKET_STALE_SEC ]
}

rank_of() { # rank_of <qdir> <myticket-basename>: live tickets sorting before mine
  r=0
  for t in $(ls "$1" 2>/dev/null | LC_ALL=C sort); do
    [ "$t" = "$2" ] && { echo $r; return; }
    if ticket_live "$1/$t"; then r=$((r + 1)); else rm -f "$1/$t"; fi
  done
  echo $r
}

write_owner() { # write_owner <dir>
  tmpf="$1/.owner.$$"
  {
    echo "lane=$LANE"; echo "pid=$OWNER_PID"; echo "host=$OWNER_HOST"
    echo "start=$(now)"; echo "start_utc=$(utc)"
    echo "max_minutes=$MAXMIN"; echo "priority=$PRIO"
  } > "$tmpf" && mv "$tmpf" "$1/owner"
}

parse_opts() {
  MAXMIN=$SLOT_DEFAULT_MAX; PRIO=50; WAITMIN=""; OPT_PID=""; RUN_ARGS=""
  while [ $# -gt 0 ]; do
    case $1 in
      --max-minutes) MAXMIN=${2:-}; shift 2 ;;
      --priority) PRIO=${2:-}; shift 2 ;;
      --pid) OPT_PID=${2:-}; shift 2 ;;
      --wait-minutes) WAITMIN=${2:-}; shift 2 ;;
      --) shift; RUN_ARGS=yes; break ;;
      *) die "unknown option '$1'" ;;
    esac
  done
  case $MAXMIN in ''|*[!0-9]*) die "--max-minutes needs a whole number" ;; esac
  case $PRIO in ''|*[!0-9]*) die "--priority needs a whole number 0..99999" ;; esac
  [ "$PRIO" -le 99999 ] || die "--priority needs a whole number 0..99999"
  case $WAITMIN in *[!0-9]*) die "--wait-minutes needs a whole number" ;; esac
  REST_COUNT=$#
}

# do_acquire: sets ACQUIRED to the slot name; returns 1 on wait timeout
do_acquire() {
  POOL=$1
  mkdir -p "$SLOT_ROOT/$POOL.queue" || die "cannot create $SLOT_ROOT/$POOL.queue"
  qdir=$SLOT_ROOT/$POOL.queue
  key=$(printf '%05d-%010d' $((99999 - PRIO)) "$(now)")
  safe_lane=$(echo "$LANE" | tr -c 'A-Za-z0-9._-' '_')
  ticket="$key-$OWNER_HOST-$OWNER_PID-$$-$safe_lane"
  printf 'lane=%s\npid=%s\nhost=%s\npriority=%s\nsince=%s\n' "$LANE" "$OWNER_PID" "$OWNER_HOST" "$PRIO" "$(utc)" > "$qdir/$ticket"
  trap 'rm -f "$qdir/$ticket"' EXIT
  trap 'rm -f "$qdir/$ticket"; exit 130' INT TERM HUP
  deadline=""
  [ -n "$WAITMIN" ] && deadline=$(( $(now) + WAITMIN * 60 ))
  announced=""
  while :; do
    touch "$qdir/$ticket"
    sweep_stale "$POOL"
    free=$(free_count "$POOL")
    rank=$(rank_of "$qdir" "$ticket")
    if [ "$rank" -lt "$free" ]; then
      for s in $(pool_slots "$POOL"); do
        if mkdir "$SLOT_ROOT/$s" 2>/dev/null; then
          write_owner "$SLOT_ROOT/$s"
          rm -f "$qdir/$ticket"; trap - EXIT INT TERM HUP
          ACQUIRED=$s
          return 0
        fi
      done
    fi
    if [ -z "$announced" ]; then
      echo "slot.sh: $LANE waiting for $POOL (priority $PRIO, $rank ahead, $free free)" >&2
      announced=1
    fi
    if [ -n "$deadline" ] && [ "$(now)" -ge "$deadline" ]; then
      rm -f "$qdir/$ticket"; trap - EXIT INT TERM HUP
      echo "slot.sh: $LANE gave up waiting for $POOL after $WAITMIN min" >&2
      return 1
    fi
    sleep "$SLOT_POLL"
  done
}

do_release() { # do_release <pool> <lane> [slot]
  n=0
  for s in $(pool_slots "$1"); do
    [ -n "${3:-}" ] && [ "$s" != "$3" ] && continue
    d=$SLOT_ROOT/$s
    [ -d "$d" ] || continue
    if [ "$(kv "$d/owner" lane)" = "$2" ]; then
      rm -rf "$d" && n=$((n + 1)) && echo "released $s"
    fi
  done
  [ $n -gt 0 ] || { echo "slot.sh: $2 holds no slot in $1${3:+ named $3}" >&2; return 1; }
}

do_status() { # do_status <pool>
  echo "pool $1 (root $SLOT_ROOT on $HOST)"
  for s in $(pool_slots "$1"); do
    d=$SLOT_ROOT/$s
    st=$(slot_state "$d")
    if [ "$st" = free ]; then
      printf '  %-16s free\n' "$s"
    else
      if [ -n "$(kv "$d/owner" lane)" ]; then
        printf '  %-16s lane=%s since=%s max=%sm: %s\n' "$s" "$(kv "$d/owner" lane)" \
          "$(kv "$d/owner" start_utc)" "$(kv "$d/owner" max_minutes)" "$st"
      else
        printf '  %-16s hand-made lock, owner file: %s: %s\n' "$s" \
          "$(head -c 80 "$d/owner" 2>/dev/null | tr '\n' ' ' || true)" "$st"
      fi
    fi
  done
  q=$SLOT_ROOT/$1.queue
  if [ -d "$q" ] && [ -n "$(ls "$q" 2>/dev/null)" ]; then
    echo "  queue (next first):"
    for t in $(ls "$q" | LC_ALL=C sort); do
      printf '    %s priority=%s since=%s pid=%s@%s\n' "$(kv "$q/$t" lane)" "$(kv "$q/$t" priority)" \
        "$(kv "$q/$t" since)" "$(kv "$q/$t" pid)" "$(kv "$q/$t" host)"
    done
  fi
}

case $CMD in
  acquire|run)
    [ $# -ge 2 ] || die "usage: slot.sh $CMD <pool> <lane> [options]"
    POOL=$1; LANE=$2; shift 2
    pool_slots "$POOL" >/dev/null
    if pool_remote "$POOL"; then
      CALLER_PID=$PPID
      [ "$CMD" = acquire ] && for a in "$@"; do [ "$a" = --pid ] && CALLER_PID=""; done
      run_remote "$CMD" "$POOL" "$LANE" "$@"; exit $?
    fi
    parse_opts "$@"
    shift $(( $# - REST_COUNT ))
    mkdir -p "$SLOT_ROOT" || die "cannot create $SLOT_ROOT"
    if [ "$CMD" = run ]; then
      [ -n "$RUN_ARGS" ] && [ $# -gt 0 ] || die "usage: slot.sh run <pool> <lane> [options] -- <command>"
      OWNER_PID=$$; OWNER_HOST=$HOST
    else
      [ -z "$RUN_ARGS" ] || die "acquire takes no command; use run"
      OWNER_PID=${OPT_PID:-${SLOT_CALLER_PID:-$PPID}}
      OWNER_HOST=$HOST
      [ -n "$OPT_PID" ] || [ -z "${SLOT_CALLER_HOST:-}" ] || OWNER_HOST=$SLOT_CALLER_HOST
    fi
    do_acquire "$POOL" || exit 3
    if [ "$CMD" = acquire ]; then echo "$ACQUIRED"; exit 0; fi
    echo "slot.sh: $LANE holds $ACQUIRED" >&2
    trap 'do_release "$POOL" "$LANE" "$ACQUIRED" >/dev/null' EXIT
    trap 'exit 130' INT TERM HUP
    "$@"
    rc=$?
    exit $rc
    ;;
  release)
    [ $# -ge 2 ] || die "usage: slot.sh release <pool> <lane> [<slot-name>]"
    pool_slots "$1" >/dev/null
    if pool_remote "$1"; then run_remote release "$@"; exit $?; fi
    do_release "$@"
    ;;
  status)
    pools=${1:-"mac-build pc"}
    for p in $pools; do
      pool_slots "$p" >/dev/null
      if pool_remote "$p"; then run_remote status "$p" || echo "pool $p: ssh $SLOT_PC_HOST failed"; else do_status "$p"; fi
    done
    ;;
  ''|-h|--help|help)
    sed -n '2,/^set -u/p' "$SELF" | sed '$d' | sed 's/^# \{0,1\}//'
    ;;
  *) die "unknown command '$CMD' (acquire, run, release, status)" ;;
esac
