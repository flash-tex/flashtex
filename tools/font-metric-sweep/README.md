# Latin Modern versus Computer Modern metric sweep

Measurement-only tool for issue #710. It reuses the render pipeline's
OpenType and TFM loaders and writes no layout or rendering changes.

From the repository root:

    tools/font-metric-sweep/run.sh \
      --output docs/evidence/font-metric-sweep-710.md

The wrapper sets the shared-machine Cargo limits required for this sweep:
CARGO_TARGET_DIR=/Users/dqi26/flashtex/target-metricsweep and
CARGO_BUILD_JOBS=4. The Rust binary defaults to the shipped faces in
apps/mac/Fonts, discovers fontmath.ltx and cmr10.tfm, cmmi10.tfm, cmsy10.tfm,
and cmex10.tfm with kpsewhich, and accepts --font-dir, --tfm-dir, --fontmath,
and --output overrides.
