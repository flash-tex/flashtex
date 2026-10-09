#!/bin/sh
# Regenerate crates/bibtex/src/generated/ from third_party/bibtex/bibtex.web
# with the configuration in crates/bibtex/web2rust.args (TeX Live's bibtex.ch,
# then crates/bibtex/changes/flashtex.ch). `cargo test -p web2rust --test
# drift` checks that the committed translation is this one.
set -e
cd "$(dirname "$0")/.."
rm -rf crates/bibtex/src/generated
mkdir -p crates/bibtex/src/generated
cargo run --release -p web2rust -- third_party/bibtex/bibtex.web \
    @crates/bibtex/web2rust.args --out-dir crates/bibtex/src/generated
