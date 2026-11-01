#!/usr/bin/env bash
# Fixture: a script directly in apps/mac is scanned too.
swift build -Xlinker -L../../crates/gpl-host/target/release # FIXTURE-VIOLATION
