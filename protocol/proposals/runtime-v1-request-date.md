# Proposal: `payload.date` — the compile request carries the document date

Status: **PROPOSAL** (owner bug "`\date{\today}` always renders January 1, 1970",
issue #2, lane `nixos-today`, branch `engine/today-date-compiler`). Additive and
optional. It changes no frozen shape: `protocol/rendering-v2.schema.json`, the
`compile_result` payload, and any request that omits the field stay byte-for-byte
as they are.

Producer co-sign needed before the Mac relies on it: `apps/mac` (`RuntimeV1.CompileRequest`)
and `crates/document-runtime` are the request producers; `crates/render-pipeline`
is the second consumer and is blocked on a `vendor/compiler` re-pin (§6).

## 1. Summary

`\today` must print today's date. A compiler that reads the wall clock cannot,
because runtime-v1 requires byte-identical output for byte-identical input. The
resolution is the one reproducible builds already uses: **the clock is read by the
caller, not by the compiler.** The date becomes an ordinary request input.

1. New optional request field `payload.date`: a proleptic-Gregorian **civil date**
   `YYYY-MM-DD`.
2. When present, `\today` — and the `\maketitle` date line that defaults to it —
   renders that date in LaTeX's default English format.
3. When **absent**, the compiler uses the Unix epoch civil date, `1970-01-01`,
   printing `January 1, 1970` exactly as it does today. Every existing client and
   every committed fixture is therefore unchanged, byte for byte.
4. A malformed or out-of-range value is an **error**, never a silent fallback.
5. `compile_result` is unchanged — no echo field. The caller knows what it sent.

Determinism is preserved and, in fact, stated more precisely than before: the
output is a pure function of `(documents, entry_path, layout_capabilities, date)`.
Byte-identical input *including `date`* still gives byte-identical output.

## 2. Why a civil date and not a timestamp

`\today` is a local calendar date, not an instant. A Unix timestamp or an RFC 3339
instant would force the compiler to choose a timezone in order to render it, and
that timezone would be an input the request does not carry — exactly the hidden
non-determinism this field exists to remove. The caller already knows the user's
zone; it resolves the instant to a civil date and sends the result.

This also keeps the field trivially comparable, loggable and cache-keyable.

## 3. Wire format

```json
{"protocol_version":1,"id":"c-1","type":"compile","payload":{
  "project_id":"demo","revision":1,"entry_path":"main.tex",
  "documents":[{"path":"main.tex","text":"\\today\n"}],
  "date":"2026-09-13"}}
```

`date` is a string of exactly 10 ASCII characters matching `YYYY-MM-DD`:
four digits, `-`, two digits, `-`, two digits. No timezone, no time, no offset,
no leading `+`, no other separator. The year is `0001`–`9999`; the month is
`01`–`12`; the day is valid for that month in the proleptic Gregorian calendar,
leap years included (`2026-02-29` is rejected, `2024-02-29` is accepted).

A value that is not a string, or does not match, or names a day that does not
exist, fails the compile with one `error` diagnostic and no pages. The compiler
must not round, clamp, reinterpret or fall back to the epoch on a malformed
value: a caller that sent a date meant it, and quietly typesetting a different
one is the failure mode this proposal exists to end.

An **absent** field is not malformed. It means "no date supplied", and the
compiler uses `1970-01-01`.

## 4. Rendering

LaTeX's kernel `\today` is, verbatim from `\meaning\today` (TeX Live 2025):

```
\ifcase\month\or January\or February\or March\or April\or May\or June\or
July\or August\or September\or October\or November\or December\fi
\space\number\day, \number\year
```

so the rendered form is `<MonthName><space><day>,<space><year>` with **no**
zero padding on the day or the year: `September 13, 2026`, `January 1, 1970`.
Verified against `pdflatex` 3.141592653-2.6-1.40.27 (TeX Live 2025) rather than
assumed.

The month names are the kernel's English set. `babel`/`polyglossia` and any
other locale-dependent date format are **out of scope** here and unimplemented;
this field carries a date, not a locale. A locale would be a separate field and
a separate proposal.

## 5. Caller obligations

A caller that wants real dates supplies `payload.date` on every compile request.
The date it supplies is resolved in this precedence order:

1. an explicit override, where the caller offers one (`flashtex --date YYYY-MM-DD`);
2. **`SOURCE_DATE_EPOCH`** in the caller's environment, if set to a valid
   non-negative integer — the civil date of that instant **in UTC**, per the
   reproducible-builds specification;
3. otherwise the caller's current **local** civil date.

Note a deliberate divergence from pdfTeX: pdfTeX honours `SOURCE_DATE_EPOCH` for
`\today` only when `FORCE_SOURCE_DATE=1` is also set (with `SOURCE_DATE_EPOCH`
alone it changes only the PDF `/CreationDate`). FlashTeX callers honour
`SOURCE_DATE_EPOCH` on its own, which is the behaviour the reproducible-builds
spec asks for and which costs nothing here. Harnesses that compare against
pdflatex output must therefore set **both** variables for the oracle and pass
the epoch explicitly to FlashTeX.

Committed-expectation harnesses (the real-world corpus, the visual oracle, the
amsmath/amssymb/tabular corpora, the page-frame oracle) pin `1970-01-01`
explicitly so their committed reference data stays byte-identical no matter what
the default becomes later. They must never be regenerated to accommodate a
clock-derived date.

## 6. Migration gates

1. The compiler accepts the field, rejects malformed values, and is unchanged for
   requests that omit it. Its warm-session cache must key on the date, or a second
   compile of the same document on a different date would reuse the first day's
   pages.
2. Harnesses pin the epoch explicitly (see §5) before any producer starts sending
   real dates.
3. `crates/render-pipeline` reads the field into `RenderOptions` and threads it to
   the parser. This is **blocked** on re-pinning `crates/render-pipeline/vendor/compiler`
   to a revision carrying `parser::parse_project_with`; until then the pipeline's
   arm sits behind its `request-date` cargo feature, off by default, exactly as
   `amsmath-inline` and `compiler-text-nucleus` did before their re-pins.
4. Only after 1–3 may `apps/mac` and `crates/document-runtime` start sending
   `date`, and only then does the owner-reported bug disappear from the app.
5. A producer that sends `date` must not assume it was honoured: an old worker
   ignores the field silently, and this proposal deliberately adds no echo field
   to `compile_result`. A caller that must prove the date was used should read it
   back out of the rendered output, not infer it from a successful compile.

## 7. Compatibility

- Old producer, new consumer: no `date`, epoch, output unchanged.
- New producer, old consumer: the unknown key is ignored (`scripts/check_runtime.py`
  and both workers tolerate unknown payload keys); the document renders with the
  epoch, as it does today. No error, no corruption — but also no real date, which
  is why gate 6.4 exists.
- `protocol/fixtures/compile-request.json` is unchanged; the fixture keeps
  exercising the absent-field path.
