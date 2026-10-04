#!/bin/bash
set -e
cd "$(dirname "$0")/proto-copy"
T=../target/release/tbench
mkdir -p docs
$T gen docs/d10 10
$T gen docs/d100 100
$T gen docs/d300 300
$T gen docs/d1000 1000
$T gen docs/c300 300 14
ls docs
wc -c docs/*/main.typ
