#!/usr/bin/env bash
# Fixture for scripts/tests/check-license-boundary-mac.test.sh; never run.
swift build -c release
swift build -c release -Xlinker -L../../crates/gpl-host/target/release # FIXTURE-VIOLATION
cp ../../crates/gpl-host/target/release/gpl-host App.app/Contents/Helpers/gpl-host
