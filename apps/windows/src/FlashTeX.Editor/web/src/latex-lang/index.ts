// name: index.ts
// purpose: Packages the generated Lezer LaTeX parser (parser.ts, built from
//   latex.grammar by `npm run build:grammar`) as a CodeMirror 6
//   `LanguageSupport` -- the same shape as an official `@codemirror/lang-*`
//   package: an `LRLanguage` with syntax-highlighting tags, indentation and
//   folding node props, plus bracket-matching wired to the grammar's
//   closedBy/openedBy pairs (OpenBrace/CloseBrace, OpenBracket/CloseBracket).
// author: Claude Sonnet 5
// date: 2026-09-14

import { parser } from "./parser.js";
import {
  LRLanguage,
  LanguageSupport,
  bracketMatching,
  continuedIndent,
  delimitedIndent,
  foldInside,
  foldNodeProp,
  indentNodeProp,
} from "@codemirror/language";
import { styleTags, tags as t } from "@lezer/highlight";

/**
 * The bare `LRLanguage`: the parser plus the node props CodeMirror's
 * language-generic extensions (highlighting, indentation, folding, bracket
 * matching) key off of. Exported separately from `latex()` so other code
 * (e.g. a language-data facet extension, or tests that want the parser
 * without pulling in bracket matching) can depend on just the language.
 */
export const latexLanguage = LRLanguage.define({
  name: "latex",
  parser: parser.configure({
    props: [
      styleTags({
        BeginKw: t.keyword,
        EndKw: t.keyword,
        ControlWord: t.macroName,
        ControlSymbol: t.macroName,
        EnvName: t.className,
        LineComment: t.lineComment,
        "InlineMathDelim DisplayMathDollarDelim DisplayMathOpen DisplayMathClose": t.processingInstruction,
        Text: t.content,
        // Math content reuses the plain `Text` token rather than a separate
        // "MathText" token (see latex.grammar for why introducing a second
        // token with an identical character class was itself the source of
        // an unresolvable overlap) -- so math text is instead picked out
        // here by tree position, via @lezer/highlight's path-selector syntax:
        // "Parent/..." tags the matched node *and every descendant*, so any
        // Text/Command/ControlSymbol/Group/BracketGroup nested inside a math
        // span is styled as math regardless of nesting depth, while the same
        // node types outside math keep their normal prose styling above.
        "InlineMath/... DisplayMathDollar/... DisplayMathBracket/...": t.number,
        "OpenBrace CloseBrace": t.brace,
        "OpenBracket CloseBracket": t.squareBracket,
      }),
      indentNodeProp.add({
        // Indent the body of `\begin{name} ... \end{name}` one unit deeper
        // than the line the `\begin{...}` sits on, and let `\end{...}` itself
        // dedent back out -- the same `delimitedIndent`-style pattern
        // `@codemirror/lang-*` packages use for brace/tag bodies, applied
        // here to LaTeX's begin/end delimiters instead of braces.
        Environment: delimitedIndent({ closing: "\\end", align: false }),
        Group: delimitedIndent({ closing: "}", align: false }),
        BracketGroup: delimitedIndent({ closing: "]", align: false }),
        // A `\foo{bar}` command whose argument group wraps across lines
        // continues the indentation of the command's own line.
        Command: continuedIndent(),
      }),
      foldNodeProp.add({
        Environment: foldInside,
        Group: foldInside,
        BracketGroup: foldInside,
        InlineMath: foldInside,
        DisplayMathDollar: foldInside,
        DisplayMathBracket: foldInside,
      }),
    ],
  }),
  languageData: {
    commentTokens: { line: "%" },
    closeBrackets: { brackets: ["{", "[", "$"] },
    indentOnInput: /^\s*\\end\b/,
  },
});

/**
 * The full `LanguageSupport` for LaTeX: the language plus bracket matching
 * wired to the grammar's `closedBy`/`openedBy` node props (declared on
 * OpenBrace/CloseBrace and OpenBracket/CloseBracket in latex.grammar), the
 * same shape returned by e.g. `python()` from `@codemirror/lang-python`.
 */
export function latex(): LanguageSupport {
  return new LanguageSupport(latexLanguage, [bracketMatching()]);
}
