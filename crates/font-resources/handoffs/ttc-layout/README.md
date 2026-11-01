# Existing owner patch: complete bounded collection layout

Base is exact f2fdb08; `bounded-layout.patch` edits only the existing font-engine
parser/export and its existing collection test file. These are patch bytes for
review, not edits to authoritative peer source. Do not adopt until the owner
reviews/applies/tests/publishes the fix.

The patch adds `CollectionLimits` and `collection_layout_with_limits`, keeping the
old function as a default-limits wrapper. It exposes header and face-directory
ranges, validates version1 TTC and supported sfnt versions, explicitly refuses
TTC2/DSIG, and rejects header/directory/table overlap. Same-tag identical table
sharing across different faces remains valid. Face/table/work limits are checked
before allocating parser vectors or scanning pairs. The work charge conservatively
bounds duplicate-tag, within/across-face and structural-intersection comparisons;
no latency claim follows. Existing consumer table alignment checks remain required.

Actual scratch validation: compile unchanged peer source plus this patch as a
no-feature Rust library; compile/run its ttc_layout integration tests:10 passed,
including8 original tests. Compile/run our prior `tools/ttc_peer_contract.rs`: valid
directory accepted, malformed TTC/sfnt versions and header/directory overlap all
rejected. Exact source/patch hashes are in evidence.json. This validates directory
contracts only; selected font table semantics still use the existing parser.

Reproduction from a disposable checkout of the exact base:

```
git apply /path/to/bounded-layout.patch
rustc --edition=2024 --crate-name flashtex_font_engine --crate-type rlib crates/font-engine/src/lib.rs -o /tmp/libengine.rlib
rustc --edition=2024 --test crates/font-engine/tests/ttc_layout.rs --extern flashtex_font_engine=/tmp/libengine.rlib -o /tmp/ttc-tests
/tmp/ttc-tests
```

No dependency merge was made. When the owner publishes the accepted implementation,
our registry can map its returned verified ranges directly without decoding headers.
