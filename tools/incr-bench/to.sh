#!/usr/bin/env bash
# to.sh SECONDS CMD...: run CMD with a time limit (SIGALRM; macOS has no timeout(1)).
exec perl -e 'alarm shift; exec @ARGV or die "exec: $!"' "$@"
