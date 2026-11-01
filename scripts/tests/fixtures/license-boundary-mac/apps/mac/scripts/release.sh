#!/usr/bin/env bash
# Fixture: a case arm starting with * is code, not a comment.
case "${1:-}" in
  debug) swift build ;;
  *) swift build -c release -Xlinker -lgpl_host ;; # FIXTURE-VIOLATION
esac
