# Independent boundary oracle request

Local inventory found no tex, pdftex, luatex, tftopl, pltotf, dvitype or kpsewhich.
The oracle has NOT run here. Five original synthetic TFM programs are represented
as exact hex bytes and SHA256 in cases.json; no existing engine is production code.
They cover both boundary kerns, left/right replacement ligatures, retained-left
right-boundary ligature and a real glyph with the same code as the boundary.

On the existing Mac TeX installation, from a checkout containing this fixture:

```
python3 crates/font-resources/tools/run_boundary_oracle.py --output /tmp/flashtex-boundary-oracle-mac
```

Use a fresh output directory. The runner invokes only installed tex/pdftex and
tftopl with bounded timeouts, no shell escape and no page shipment; no renderer or
font installation is needed. It records exact executable/version, fixture and raw
log hashes. TeX's showbox can return a nonzero exit despite producing its requested
node list; the result deliberately does not label return-code success as parity.
Review tftopl diagnostics first: a repaired or rejected synthetic font is an oracle
failure, not acceptance. Review the log for font-load errors before comparing.

For each case, record the ordered glyph codes and signed kern nodes from box0.
At10pt, fix_word -131072 is exactly -1.25pt; both-kerns expects kern,A,kern. The
replacement cases expect B; retained-right-ligature expects A,B; real-boundary-code
expects A,kern,B. Also inspect ligature annotations. The input intervals in JSON
are shared API provenance hypotheses: TeX's node log does not independently expose
our byte-range API, so do not claim those ranges oracle-verified from widths alone.
Preserve raw logs and add an explicit reviewed outcome; do not overwrite pending
status solely because the self-test passes. Missing tools remain pending.

The licensed real LM metric replay is separate and uses no boundary programs;
it does not close this boundary-specific evidence gap. Synthetic expected actions
are deliberately distinct from observed TeX evidence.
