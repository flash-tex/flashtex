// name: vitest.config.ts
// purpose: Vitest configuration for the FlashTeX editor web package -- pure
//   Node.js unit tests (no browser/WebView2 runtime needed) covering
//   byte-offsets, the LaTeX Lezer grammar, the native<->JS bridge protocol,
//   and diagnostics decorations.
// author: Claude Sonnet 5
// date: 2026-09-14

import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    environment: "node",
    include: ["test/**/*.test.ts"],
    reporters: ["default"],
  },
});
