# LM conditional-header context: explicit dependency contract

Pinned font SHA84eb01245abb17c0530ca3909427256d73df8bed2d7243b9f2717ca08c010ac8.
Exact prologue/header/trailer ranges and hashes: fixtures/pfb-matrix-context.json.
The192byte prologue checks an existing named FontDirectory resource and compares
its UniqueID0 and FontType1. That branch may push a save object and true. The
trailer then conditionally restores. Thus the new literal dictionary can be built
and subsequently discarded, retaining prior state. Its matrix/paint cannot be
established as the final active font solely by finding their literal definitions.

Primary semantics: Adobe [PostScript Language Reference, third edition](https://www.adobe.com/jp/print/postscript/pdfs/PLRM.pdf),
FontDirectory, definefont, findfont, save and restore operator descriptions.
FontDirectory maps names to in-memory font resources; restore returns VM to the
saved state. This supports the external-state ambiguity; no PostScript was run.

The current API therefore keeps its typed refusal. A byte hash proves immutable
program identity, not a branch outcome. A future passive profile needs an explicit
semantic scope that the caller can actually establish, for example an isolated
new-resource-definition mode with no prior named font and standard built-in
operators. It must not claim equivalence to executing the program in arbitrary
PostScript VM. Until that scope is authorized and represented, no acceptance flag
or guessed matrix is added.

Minimal ready-for-owner contract:

1. Identify whether the API means extracting this immutable resource's literal
   definition or reproducing the font remaining after arbitrary PS execution.
   The latter requires external VM state and is not a passive file-only operation.
2. For an isolated profile, require whole-font/header/trailer/private-declaration
   identity and exact recognized prologue/body/closure grammar. Validate the
   complete branch and encoding construction, not just FontMatrix/PaintType tokens.
3. Reject changed branch predicates, duplicate/redefined keys, aliases/evaluated
   definitions, unsupported procedures and trailer overrides. Carry the profile
   identity and all rejected semantics with transformed output.
4. Test a fresh-name case separately from an existing UID/type match and a
   nonmatching existing resource. Never substitute the new literal definition for
   a previous active one without representing that decision.

Concrete safety improvement in this checkpoint: the already-supported simple
literal-header profile now ALSO requires exactly512padding zeros (whitespace
allowed) followed solely by cleartomark. A conditional restore or post-eexec
matrix/paint mutation is refused. Added branch/redefinition/trailer-tamper tests.
The real LM prologue and trailer remain unsupported. No default is relaxed.

TTC followup: fetched exact float-layout4422cff/f2fdb08; reviewed all17 Daniel remote
refs for corrected collection_layout_with_limits exports and found none. Existing
review patch82a478f remains ready; no uncorrected peer dependency was consumed.
