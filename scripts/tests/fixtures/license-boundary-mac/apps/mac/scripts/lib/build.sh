#!/usr/bin/env bash
# Fixture: scripts/lib is scanned too.
swift build -Xswiftc -lgpl_host # FIXTURE-VIOLATION
