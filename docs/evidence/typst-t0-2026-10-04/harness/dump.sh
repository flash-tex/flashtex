#!/bin/bash
set -e
cd "$(dirname "$0")/proto-copy"
T=../target/release/tbench
mkdir -p px/d10 px/d300
$T dump docs/d10 1 px/d10/p2.json
$T dump docs/d300 149 px/d300/d300p150.json
swiftc -O px/pxdiff.swift -o ../pxdiff
ls -la px/d10 px/d300
