// name: byte-offsets.ts
// purpose: UTF-8 byte offset <-> JS UTF-16 code-unit index conversion helpers,
//   ported from apps/windows/src/FlashTeX.Protocol/ByteOffsets.cs (itself ported
//   from apps/mac/Sources/FlashTeXProtocol/ByteOffsets.swift). Safety-critical:
//   keeps diagnostics/selections/edits in sync with the Rust engine's byte-range
//   addressing across the native<->WebView2 bridge. Every offset that crosses
//   `bridge.ts` must be converted through this module.
// author: Claude Sonnet 5
// date: 2026-09-14

/** Inclusive-start, exclusive-end pair of UTF-16 code-unit offsets. */
export interface Utf16Range {
  readonly utf16Start: number;
  readonly utf16End: number;
}

/** Inclusive-start, exclusive-end pair of UTF-8 byte offsets. */
export interface Utf8ByteRange {
  readonly startByte: number;
  readonly endByte: number;
}

/**
 * Maps a UTF-8 byte range of `text` to the equivalent UTF-16 code-unit range,
 * or `null` if either offset is out of bounds, reversed, or not on a Unicode
 * scalar (code point) boundary.
 *
 * Mirrors `ByteOffsets.Utf16RangeForUtf8Bytes` in ByteOffsets.cs exactly: we
 * walk `text` one Unicode code point at a time (JS `for...of` over a string
 * iterates by code point, combining surrogate pairs, the same way C#'s
 * `string.EnumerateRunes()` iterates by Unicode scalar value), accumulating
 * both the UTF-8 byte length and the UTF-16 code-unit length of each code
 * point, and only accept an offset that lands exactly on a code-point
 * boundary.
 */
export function utf16RangeForUtf8Bytes(text: string, startByte: number, endByte: number): Utf16Range | null {
  if (startByte < 0 || endByte < startByte) {
    return null;
  }

  let utf16Start: number | null = startByte === 0 ? 0 : null;
  let utf16End: number | null = endByte === 0 ? 0 : null;
  let byteOffset = 0;
  let charOffset = 0;

  for (const codePointString of text) {
    byteOffset += utf8SequenceLength(codePointString.codePointAt(0)!);
    charOffset += codePointString.length; // 1 for BMP scalars, 2 for a surrogate pair
    if (byteOffset === startByte) {
      utf16Start = charOffset;
    }
    if (byteOffset === endByte) {
      utf16End = charOffset;
    }
  }

  return utf16Start !== null && utf16End !== null ? { utf16Start, utf16End } : null;
}

/**
 * Reverse mapping: the UTF-8 byte range corresponding to a UTF-16 code-unit
 * range (e.g. a CodeMirror editor selection), or `null` if either endpoint is
 * out of bounds, reversed, or splits a surrogate pair.
 *
 * Mirrors `ByteOffsets.Utf8ByteRangeForUtf16Range` in ByteOffsets.cs.
 */
export function utf8ByteRangeForUtf16Range(text: string, utf16Start: number, utf16End: number): Utf8ByteRange | null {
  if (utf16Start < 0 || utf16End < utf16Start || utf16End > text.length) {
    return null;
  }

  let startByte: number | null = utf16Start === 0 ? 0 : null;
  let endByte: number | null = utf16End === 0 ? 0 : null;
  let byteOffset = 0;
  let charOffset = 0;

  for (const codePointString of text) {
    byteOffset += utf8SequenceLength(codePointString.codePointAt(0)!);
    charOffset += codePointString.length;
    if (charOffset === utf16Start) {
      startByte = byteOffset;
    }
    if (charOffset === utf16End) {
      endByte = byteOffset;
    }
  }

  return startByte !== null && endByte !== null ? { startByte, endByte } : null;
}

/**
 * Byte-for-byte comparison of two strings' UTF-8 encodings. JS `===` on
 * strings already compares UTF-16 code units exactly (no Unicode
 * normalization), so this gives the "did the underlying bytes change" answer
 * without an extra UTF-8 encode of either operand -- matching the ordinal
 * comparison `ByteOffsets.SameBytes` uses on the C# side.
 */
export function sameBytes(a: string, b: string): boolean {
  return a === b;
}

/** UTF-8 byte length of `text`. */
export function utf8ByteCount(text: string): number {
  return new TextEncoder().encode(text).length;
}

/** UTF-8 byte length of a single Unicode code point. */
function utf8SequenceLength(codePoint: number): number {
  if (codePoint <= 0x7f) {
    return 1;
  }
  if (codePoint <= 0x7ff) {
    return 2;
  }
  if (codePoint <= 0xffff) {
    return 3;
  }
  return 4;
}
