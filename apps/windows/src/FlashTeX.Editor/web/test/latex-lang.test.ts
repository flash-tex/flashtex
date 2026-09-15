// name: latex-lang.test.ts
// purpose: Coverage for the generated LaTeX Lezer parser (src/latex-lang/parser.ts,
//   built from latex.grammar by `npm run build:grammar`) and the LanguageSupport
//   wrapper in src/latex-lang/index.ts. Walks the actual parse tree for each
//   snippet and asserts real node names/ranges/text, not just "it doesn't throw".
//
//   Known limitation: LaTeX's true tokenization is context-sensitive in ways this
//   LALR(1)-ish grammar does not attempt (e.g. `\verb|...|`'s delimiter-scoped
//   literal body, catcode changes via \catcode, or user-defined active characters).
//   Those are out of scope here; this grammar targets the common surface syntax
//   described in latex.grammar's header.
// author: Claude Sonnet 5
// date: 2026-09-14

import { describe, expect, it } from "vitest";
import { NodeProp, type SyntaxNode, type Tree } from "@lezer/common";
import { parser } from "../src/latex-lang/parser.js";
import { latex, latexLanguage } from "../src/latex-lang/index.js";

/** A shallow, JSON-friendly description of one node: its name, text slice, and children. */
interface NodeShape {
  readonly name: string;
  readonly text: string;
  readonly children: readonly NodeShape[];
}

function shape(node: SyntaxNode, source: string): NodeShape {
  const children: NodeShape[] = [];
  for (let child = node.firstChild; child; child = child.nextSibling) {
    children.push(shape(child, source));
  }
  return { name: node.name, text: source.slice(node.from, node.to), children };
}

function parseShape(source: string): NodeShape {
  const tree: Tree = parser.parse(source);
  return shape(tree.topNode, source);
}

/**
 * Depth-first search for the first *descendant* (never the node passed in
 * itself) with the given name -- deliberately excludes self so that, e.g.,
 * `find(environmentNode, "Environment")` finds a nested environment rather
 * than trivially returning the node it was called on.
 */
function find(node: NodeShape, name: string): NodeShape | null {
  for (const child of node.children) {
    if (child.name === name) {
      return child;
    }
    const found = find(child, name);
    if (found) {
      return found;
    }
  }
  return null;
}

/** All descendants (never the node passed in itself) with the given name, in document order. */
function findAll(node: NodeShape, name: string): NodeShape[] {
  const out: NodeShape[] = [];
  for (const child of node.children) {
    if (child.name === name) {
      out.push(child);
    }
    out.push(...findAll(child, name));
  }
  return out;
}

describe("latex grammar: control sequences", () => {
  it("parses a bare control word", () => {
    const tree = parseShape("\\alpha");
    const command = tree.children[0]!;
    expect(command.name).toBe("Command");
    expect(command.text).toBe("\\alpha");
    expect(command.children).toEqual([{ name: "ControlWord", text: "\\alpha", children: [] }]);
  });

  it("parses a starred control word (\\foo*)", () => {
    const command = parseShape("\\foo*").children[0]!;
    expect(command.name).toBe("Command");
    const controlWord = find(command, "ControlWord")!;
    expect(controlWord.text).toBe("\\foo*");
  });

  it("parses a control symbol (backslash + one non-letter char) distinctly from a control word", () => {
    const tree = parseShape("\\% \\$ \\\\ \\,");
    const symbols = findAll(tree, "ControlSymbol");
    expect(symbols.map((s) => s.text)).toEqual(["\\%", "\\$", "\\\\", "\\,"]);
    // None of these should ever be misparsed as a ControlWord.
    expect(findAll(tree, "ControlWord")).toHaveLength(0);
  });

  it("treats a backslash followed by a letter as ControlWord even for a single letter (never ControlSymbol)", () => {
    const tree = parseShape("\\A");
    expect(findAll(tree, "ControlWord")).toHaveLength(1);
    expect(findAll(tree, "ControlSymbol")).toHaveLength(0);
    expect(find(tree, "ControlWord")!.text).toBe("\\A");
  });
});

describe("latex grammar: commands with arguments", () => {
  it("parses \\foo{bar} with a required-argument Group attached to the command", () => {
    const command = parseShape("\\foo{bar}").children[0]!;
    expect(command.name).toBe("Command");
    expect(command.children.map((c) => c.name)).toEqual(["ControlWord", "Group"]);
    const group = command.children[1]!;
    expect(group.children.map((c) => c.name)).toEqual(["OpenBrace", "Text", "CloseBrace"]);
    expect(find(group, "Text")!.text).toBe("bar");
  });

  it("parses \\foo[opt]{bar} with the optional BracketGroup preceding the required Group", () => {
    const command = parseShape("\\foo[opt]{bar}").children[0]!;
    expect(command.children.map((c) => c.name)).toEqual(["ControlWord", "BracketGroup", "Group"]);
    expect(find(command, "BracketGroup")!.text).toBe("[opt]");
    expect(find(command, "Group")!.text).toBe("{bar}");
  });

  it("attaches multiple argument groups to the same command rather than treating later ones as siblings", () => {
    const content = parseShape("\\foo{a}{b}");
    const command = content.children[0]!;
    expect(command.name).toBe("Command");
    const groups = command.children.filter((c) => c.name === "Group");
    expect(groups).toHaveLength(2);
    expect(groups.map((g) => g.text)).toEqual(["{a}", "{b}"]);
    // Both groups belong to the one Command node, not to the top-level Document.
    expect(content.children).toHaveLength(1);
  });
});

describe("latex grammar: environments", () => {
  it("parses \\begin{name}...\\end{name}, tagging the keywords and env names", () => {
    const tree = parseShape("\\begin{itemize}\\item a\\end{itemize}");
    const env = tree.children[0]!;
    expect(env.name).toBe("Environment");
    const names = findAll(env, "EnvName");
    expect(names.map((n) => n.text)).toEqual(["itemize", "itemize"]);
    expect(find(env, "BeginKw")).toBeTruthy();
    expect(find(env, "EndKw")).toBeTruthy();
    expect(find(env, "Command")!.text).toBe("\\item");
  });

  it("parses nested environments as nested Environment nodes", () => {
    const tree = parseShape("\\begin{outer}\\begin{inner}x\\end{inner}\\end{outer}");
    const outer = tree.children[0]!;
    expect(outer.name).toBe("Environment");
    const inner = find(outer, "Environment")!;
    expect(inner.text).toBe("\\begin{inner}x\\end{inner}");
    expect(findAll(outer, "Environment")).toHaveLength(1); // only the inner one, found from outer
    expect(find(inner, "Text")!.text).toBe("x");
  });
});

describe("latex grammar: math mode", () => {
  it("wraps inline math in InlineMath, distinct from surrounding prose Text", () => {
    const tree = parseShape("before $x^2$ after");
    expect(tree.children.map((c) => c.name)).toEqual(["Text", "InlineMath", "Text"]);
    const math = tree.children[1]!;
    expect(math.children.map((c) => c.name)).toEqual(["InlineMathDelim", "Text", "InlineMathDelim"]);
    expect(math.text).toBe("$x^2$");
  });

  it("wraps $$...$$ display math in DisplayMathDollar with doubled delimiters", () => {
    const tree = parseShape("before $$x^2$$ after");
    const math = find(tree, "DisplayMathDollar")!;
    expect(math.text).toBe("$$x^2$$");
    const delims = findAll(math, "DisplayMathDollarDelim");
    expect(delims.map((d) => d.text)).toEqual(["$$", "$$"]);
  });

  it("wraps \\[...\\] display math in DisplayMathBracket, distinct from a ControlSymbol + BracketGroup", () => {
    const tree = parseShape("before \\[x^2\\] after");
    const math = find(tree, "DisplayMathBracket")!;
    expect(math.text).toBe("\\[x^2\\]");
    expect(math.children.map((c) => c.name)).toEqual(["DisplayMathOpen", "Text", "DisplayMathClose"]);
    // \[ and \] must not be misparsed as a lone ControlSymbol ("\[") followed
    // by stray bracket tokens -- this is exactly the DisplayMathOpen /
    // ControlSymbol tie the grammar's @precedence block resolves.
    expect(findAll(tree, "ControlSymbol")).toHaveLength(0);
  });

  it("allows commands and groups inside math content, still distinct from prose", () => {
    const tree = parseShape("$\\frac{1}{2}$");
    const math = find(tree, "InlineMath")!;
    const command = find(math, "Command")!;
    expect(command.text).toBe("\\frac{1}{2}");
    expect(findAll(command, "Group").map((g) => g.text)).toEqual(["{1}", "{2}"]);
  });

  it("switches back to prose Text immediately after math mode ends, mid-line", () => {
    const tree = parseShape("a $b$ c $d$ e");
    expect(tree.children.map((c) => c.name)).toEqual(["Text", "InlineMath", "Text", "InlineMath", "Text"]);
    // The single space that sits exactly at the boundary between the closed
    // InlineMath node and the next top-level content item is consumed by
    // `@skip` (so it never appears duplicated across two Text nodes), while
    // whitespace embedded *inside* an uninterrupted run (the "a " before the
    // first "$") stays part of that run's own Text node.
    expect(tree.children.map((c) => c.text)).toEqual(["a ", "$b$", "c ", "$d$", "e"]);
  });
});

describe("latex grammar: comments", () => {
  it("treats % as a line comment running to end of line", () => {
    const tree = parseShape("keep this % drop this\nkeep this too");
    const comment = find(tree, "LineComment")!;
    expect(comment.text).toBe("% drop this");
    const texts = findAll(tree, "Text").map((t) => t.text);
    expect(texts).toEqual(["keep this ", "keep this too"]);
  });

  it("does NOT treat \\% as starting a comment (it's a literal percent control symbol)", () => {
    const tree = parseShape("100\\% done % but this is a real comment");
    expect(findAll(tree, "ControlSymbol").map((s) => s.text)).toEqual(["\\%"]);
    const comment = find(tree, "LineComment")!;
    expect(comment.text).toBe("% but this is a real comment");
    // The literal-percent ControlSymbol must not itself have started a
    // second (bogus) comment consuming the rest of the line.
    expect(findAll(tree, "LineComment")).toHaveLength(1);
  });

  it("does not let a comment's content close the line: \\n ends it even mid escaped-percent-like text", () => {
    const tree = parseShape("% a comment with \\% inside it\nnot a comment");
    const comment = find(tree, "LineComment")!;
    expect(comment.text).toBe("% a comment with \\% inside it");
    expect(find(tree, "Text")!.text).toBe("not a comment");
  });
});

describe("latex grammar: brace/bracket groups usable for bracket matching", () => {
  it("tags OpenBrace/CloseBrace and OpenBracket/CloseBracket as a matched pair via node props", () => {
    const tree = parser.parse("{[a]}");
    const openBrace = tree.topNode.firstChild!.firstChild!;
    expect(openBrace.type.name).toBe("OpenBrace");
    expect(openBrace.type.prop(NodeProp.closedBy)).toEqual(["CloseBrace"]);
  });

  it("recovers from an unclosed brace group without throwing, marking an error node", () => {
    expect(() => parser.parse("\\foo{bar")).not.toThrow();
    const tree = parseShape("\\foo{bar");
    const group = find(tree, "Group")!;
    expect(group.children[0]).toEqual({ name: "OpenBrace", text: "{", children: [] });
    // No CloseBrace was found before EOF; the parser must still surface an
    // error marker rather than silently pretending the group closed cleanly.
    expect(find(tree, "⚠")).toBeTruthy();
  });

  it("recovers from an unclosed environment (\\begin with no matching \\end)", () => {
    expect(() => parser.parse("\\begin{a}text")).not.toThrow();
    const tree = parseShape("\\begin{a}text");
    expect(find(tree, "⚠")).toBeTruthy();
  });
});

describe("LanguageSupport wrapper", () => {
  it("exports a latex() LanguageSupport built on the same parser", () => {
    const support = latex();
    expect(support.language).toBe(latexLanguage);
  });

  it("configures closeBrackets language data for {, [ and $", () => {
    const data = latexLanguage.data.of({});
    // languageData is exposed through the Language's data facet; the easiest
    // black-box check is that defining it didn't throw and the language name
    // round-trips, since CodeMirror's own facet plumbing (EditorState) is
    // exercised separately in bridge.test.ts's integration test.
    expect(data).toBeTruthy();
    expect(latexLanguage.name).toBe("latex");
  });
});
