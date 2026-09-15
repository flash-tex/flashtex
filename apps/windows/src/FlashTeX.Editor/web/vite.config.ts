// name: vite.config.ts
// purpose: Production build configuration for the editor's WebView2 host
//   page. Output is plain `dist/index.html` + `dist/assets/*.js` (no
//   single-file inlining plugin) -- WebView2 serves the whole `dist/`
//   directory via SetVirtualHostNameToFolderMapping, a local virtual-host
//   mapping rather than a real HTTP server, so a multi-file build works fully
//   offline exactly like a single inlined file would. `base: "./"` keeps the
//   emitted asset URLs relative so the page also works if ever opened via
//   `file://` or a different virtual host name.
// author: Claude Sonnet 5
// date: 2026-09-14

import { defineConfig } from "vite";

export default defineConfig({
  root: ".",
  base: "./",
  build: {
    outDir: "dist",
    emptyOutDir: true,
    target: "es2022",
    sourcemap: true,
  },
});
