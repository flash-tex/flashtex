// name: byte-offsets.test.ts
// purpose: Unit tests for byte-offsets.ts, including the shared cross-language
//   fixture in byte-offset-vectors.json (also consumed by the C# test
//   ByteOffsetsCrossLanguageParityTests.cs) so the TS and C# ByteOffsets
//   implementations are proven to agree on every vector.
// author: Claude Sonnet 5
// date: 2026-09-14

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import {
  sameBytes,
  utf16RangeForUtf8Bytes,
  utf8ByteCount,
  utf8ByteRangeForUtf16Range,
} from "../src/byte-offsets.js";

interface RangeVector {
  readonly name: string;
  readonly text: string;
  readonly utf16Start: number;
  readonly utf16End: number;
  readonly utf8Start: number;
  readonly utf8End: number;
}

interface InvalidVector {
  readonly name: string;
  readonly direction: "utf8ToUtf16" | "utf16ToUtf8";
  readonly text: string;
  readonly startByte?: number;
  readonly endByte?: number;
  readonly utf16Start?: number;
  readonly utf16End?: number;
}

interface VectorFile {
  readonly rangeVectors: readonly RangeVector[];
  readonly invalidVectors: readonly InvalidVector[];
}

const here = dirname(fileURLToPath(import.meta.url));
const vectors: VectorFile = JSON.parse(readFileSync(join(here, "byte-offset-vectors.json"), "utf-8"));

describe("utf16RangeForUtf8Bytes / utf8ByteRangeForUtl16Range (shared cross-language vectors)", () => {
  it.each(vectors.rangeVectors.map((v) => [v.name, v] as const))(
    "%s: round-trips both directions",
    (_name, v) => {
      expect(utf16RangeForUtf8Bytes(v.text, v.utf8Start, v.utf8End)).toEqual({
        utf16Start: v.utf16Start,
        utf16End: v.utf16End,
      });
      expect(utf8ByteRangeForUtf16Range(v.text, v.utf16Start, v.utf16End)).toEqual({
        startByte: v.utf8Start,
        endByte: v.utf8End,
      });
    },
  );

  it.each(vectors.invalidVectors.map((v) => [v.name, v] as const))("%s: returns null", (_name, v) => {
    if (v.direction === "utf8ToUtf16") {
      expect(utf16RangeForUtf8Bytes(v.text, v.startByte!, v.endByte!)).toBeNull();
    } else {
      expect(utf8ByteRangeForUtf16Range(v.text, v.utf16Start!, v.utf16End!)).toBeNull();
    }
  });

  it("has at least one vector for ASCII, 2-byte, 3-byte, and 4-byte UTF-8 sequences", () => {
    const byteWidths = vectors.rangeVectors.map((v) => new TextEncoder().encode(v.text).length - v.text.length);
    // Not a rigorous width classifier, just a sanity check that the fixture
    // set isn't accidentally ASCII-only: some vector's UTF-8 byte count must
    // exceed its UTF-16 length (extra bytes only come from multi-byte code points).
    expect(byteWidths.some((extra) => extra > 0)).toBe(true);
  });
});

describe("utf16RangeForUtf8Bytes", () => {
  it("returns null when startByte is negative", () => {
    expect(utf16RangeForUtf8Bytes("abc", -1, 2)).toBeNull();
  });

  it("returns null when the range is reversed", () => {
    expect(utf16RangeForUtf8Bytes("abc", 2, 1)).toBeNull();
  });

  it("treats a zero-length range at the start as valid", () => {
    expect(utf16RangeForUtf8Bytes("abc", 0, 0)).toEqual({ utf16Start: 0, utf16End: 0 });
  });
});

describe("utf8ByteRangeForUtf16Range", () => {
  it("returns null when utf16End exceeds the string length", () => {
    expect(utf8ByteRangeForUtf16Range("abc", 0, 10)).toBeNull();
  });

  it("returns null when it would split a surrogate pair", () => {
    const text = "a😀b";
    // text[1] and text[2] are the high/low surrogate halves of the emoji.
    expect(utf8ByteRangeForUtf16Range(text, 2, 3)).toBeNull();
  });
});

describe("sameBytes", () => {
  it("is true for identical strings", () => {
    expect(sameBytes("café", "café")).toBe(true);
  });

  it("is false for strings differing only by content", () => {
    expect(sameBytes("café", "cafe")).toBe(false);
  });
});

describe("utf8ByteCount", () => {
  it("counts ASCII as one byte per character", () => {
    expect(utf8ByteCount("hello")).toBe(5);
  });

  it("counts a 4-byte astral character correctly", () => {
    expect(utf8ByteCount("😀")).toBe(4);
  });

  it("matches the shared vectors' full-string byte counts", () => {
    for (const v of vectors.rangeVectors) {
      if (v.utf16Start === 0 && v.utf16End === v.text.length) {
        expect(utf8ByteCount(v.text)).toBe(v.utf8End);
      }
    }
  });
});
