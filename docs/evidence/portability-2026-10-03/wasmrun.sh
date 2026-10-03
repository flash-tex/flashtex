#!/bin/sh
# The WASI spike run (README.md §3; WORK as for wasm.sh, which built the .wasm): INITEX plain.tex under wasmtime, then a
# one-page PDF from the dumped format, compared with the native engine built
# with the same features (NATIVE, default $WORK/target-nodef/...). Needs MacTeX for the inputs.
S=${WORK:?set WORK to a scratch directory}; ROOT=$(cd "$(dirname "$0")/../../.." && pwd)
W=$S/wasmrun
K=/Library/TeX/texbin/kpsewhich
rm -rf "$W"; mkdir -p "$W"; cd "$W" || exit 1
cp "$($K plain.tex)" "$($K hyphen.tex)" .
cp "$(dirname "$($K cmr10.tfm)")"/*.tfm .
cp "$($K pdftex.map)" . ; cp $ROOT/crates/flashtex-engine/pdftex.pool .
cp "$(dirname "$($K cmr10.pfb)")"/cm*.pfb .
cat > hello.tex <<'EOF'
\pdfoutput=1 \pdfcompresslevel=0 \pdfobjcompresslevel=0
\pdfinfo{/CreationDate (D:20261003000000Z) /ModDate (D:20261003000000Z)}
\pdftrailerid{}
Hello from {\bf WASI}: $a^2+b^2=c^2$. \end
EOF
WASM=$S/target/wasm32-wasip1/release/flashtex-initex.wasm
run() { wasmtime run -W exceptions=y --dir . --env SOURCE_DATE_EPOCH=1759449600 --env FORCE_SOURCE_DATE=1 --env FLASHTEX_POOL=pdftex.pool "$WASM" "$@"; }
time run -ini -interaction=nonstopmode plain '\dump' >ini.out 2>&1; echo "wasm ini exit=$?"
tail -3 plain.log
time run -fmt=plain -interaction=nonstopmode hello >hello.out 2>&1; echo "wasm run exit=$?"
tail -3 hello.log; ls -la hello.pdf plain.fmt
# The same two runs with the native engine (same sources, default features,
# working-directory resolver), in a sibling directory.
N=$S/nativerun; rm -rf "$N"; cp -R "$W" "$N"; cd "$N" || exit 1
rm -f plain.fmt plain.log hello.pdf hello.log
NATIVE=${NATIVE:-$S/target-nodef/release/flashtex-initex}
export FLASHTEX_RESOLVER=cwd SOURCE_DATE_EPOCH=1759449600 FORCE_SOURCE_DATE=1 FLASHTEX_POOL=pdftex.pool
"$NATIVE" -ini -interaction=nonstopmode plain '\dump' >ini.out 2>&1; echo "native ini exit=$?"
"$NATIVE" -fmt=plain -interaction=nonstopmode hello >hello.out 2>&1; echo "native run exit=$?"
for f in plain.fmt plain.log hello.log hello.pdf ini.out hello.out; do
  if cmp -s "$W/$f" "$N/$f"; then echo "IDENTICAL $f ($(wc -c <"$N/$f") bytes)"; else echo "DIFFER $f"; fi
done
shasum -a 256 "$W/hello.pdf" "$N/hello.pdf"
