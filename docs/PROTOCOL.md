# FlashTeX JSON Lines Protocol — v1

> Specification for communicating with the FlashTeX compiler over stdin/stdout.

## Transport

- **Channel**: stdin (requests) / stdout (responses)
- **Framing**: One complete JSON object per line (JSON Lines / NDJSON)
- **Encoding**: UTF-8
- **Newline**: \n (LF), not \r\n
- **Max request size**: 10 MB
- **Max documents per request**: 100

## Request Schema

```typescript
interface CompileRequest {
  /** Protocol version. Must be 1. */
  protocol_version: 1;

  /** Request type. Currently only "compile" is supported. */
  type: "compile";

  /** Unique request identifier. Echoed back in the response. */
  id: string;

  /** Request payload. */
  payload: {
    /** Array of documents to compile. */
    documents: Document[];

    /** Optional compilation options. */
    options?: CompileOptions;
  };
}

interface Document {
  /** Relative path of the document (e.g., "main.tex"). */
  path: string;

  /** Full text content of the document. */
  text: string;
}

interface CompileOptions {
  /** Project identifier for cache keying. Default: "default". */
  project_id?: string;

  /** Output format. Default: "pdf". */
  output_format?: "pdf" | "json";

  /** Output directory for PDF files. Default: "/tmp/flashtex". */
  output_dir?: string;

  /** Whether to include source spans in the response. Default: true. */
  include_source_spans?: boolean;

  /** Page size. Default: "a4". */
  page_size?: "a4" | "letter" | "legal";

  /** Font size for body text in points. Default: 12. */
  font_size_pt?: number;
}
```

## Response Schema

### Success Response

```typescript
interface CompileResponse {
  /** Echoed request ID. */
  id: string;

  /** "ok" on success, "error" on failure. */
  status: "ok" | "error";

  /** Whether the result was served from cache. */
  cache_hit: boolean;

  /** Compilation time in microseconds (0 if cache hit). */
  compile_time_us: number;

  /** Rendered pages with positioned items. */
  pages: Page[];

  /** Path to the generated PDF file, if output_format is "pdf". */
  pdf_path: string | null;

  /** Array of warnings and errors. Empty on clean compile. */
  diagnostics: Diagnostic[];
}

interface Page {
  /** Page dimensions in points. */
  width_pt: number;
  height_pt: number;

  /** Positioned items on this page. */
  items: PageItem[];
}

interface PageItem {
  /** Item type. */
  type: "text" | "rule" | "space";

  /** X position in points from left edge. */
  x_pt: number;

  /** Y position in points from top edge. */
  y_pt: number;

  /** Text content (only for type: "text"). */
  text?: string;

  /** Font size in points (only for type: "text"). */
  font_size_pt?: number;

  /** Font style (only for type: "text"). */
  font_style?: "normal" | "bold" | "italic" | "bold-italic";

  /** Source location in the .tex file (if include_source_spans is true). */
  source?: SourceSpan;
}

interface SourceSpan {
  /** Start byte offset in the source file. */
  start_byte: number;

  /** End byte offset (exclusive) in the source file. */
  end_byte: number;

  /** Source file path (matches Document.path). */
  file?: string;
}
```

### Error Response

```typescript
interface ErrorResponse {
  id: string;
  status: "error";
  cache_hit: false;
  compile_time_us: number;

  /** Partial output — whatever was rendered before the error. */
  pages: Page[];

  /** No PDF generated on error. */
  pdf_path: null;

  /** One or more diagnostics explaining the failure. */
  diagnostics: Diagnostic[];
}

interface Diagnostic {
  /** Severity level. */
  severity: "error" | "warning" | "info";

  /** Human-readable error message. */
  message: string;

  /** Source location of the error. */
  span: SourceSpan;

  /** Optional fix suggestion. */
  suggestion?: string;

  /** Related locations (e.g., where a brace was opened). */
  related?: RelatedInfo[];
}

interface RelatedInfo {
  message: string;
  span: SourceSpan;
}
```

## Examples

### Minimal Request

```json
{"protocol_version":1,"type":"compile","id":"r1","payload":{"documents":[{"path":"main.tex","text":"Hello, world!"}]}}
```

### Request with Options

```json
{
  "protocol_version": 1,
  "type": "compile",
  "id": "r2",
  "payload": {
    "documents": [
      {
        "path": "main.tex",
        "text": "\\documentclass{article}\n\\begin{document}\n\\section{Introduction}\nFlashTeX compiles LaTeX at the speed of typing.\n\\end{document}"
      }
    ],
    "options": {
      "project_id": "my-paper",
      "output_format": "pdf",
      "output_dir": "/tmp/my-paper",
      "page_size": "letter",
      "font_size_pt": 11
    }
  }
}
```

### Success Response

```json
{
  "id": "r2",
  "status": "ok",
  "cache_hit": false,
  "compile_time_us": 5200,
  "pages": [
    {
      "width_pt": 612,
      "height_pt": 792,
      "items": [
        {
          "type": "text",
          "x_pt": 72.0,
          "y_pt": 72.0,
          "text": "1 Introduction",
          "font_size_pt": 17,
          "font_style": "bold",
          "source": {
            "start_byte": 47,
            "end_byte": 59,
            "file": "main.tex"
          }
        },
        {
          "type": "text",
          "x_pt": 72.0,
          "y_pt": 102.0,
          "text": "FlashTeX compiles LaTeX at the speed of typing.",
          "font_size_pt": 11,
          "font_style": "normal",
          "source": {
            "start_byte": 60,
            "end_byte": 108,
            "file": "main.tex"
          }
        }
      ]
    }
  ],
  "pdf_path": "/tmp/my-paper/rev0.pdf",
  "diagnostics": []
}
```

### Error Response with Recovery

```json
{
  "id": "r3",
  "status": "error",
  "cache_hit": false,
  "compile_time_us": 3800,
  "pages": [
    {
      "width_pt": 595.276,
      "height_pt": 841.89,
      "items": [
        {
          "type": "text",
          "x_pt": 72.0,
          "y_pt": 72.0,
          "text": "This part rendered fine.",
          "font_size_pt": 12,
          "font_style": "normal",
          "source": {"start_byte": 0, "end_byte": 23}
        }
      ]
    }
  ],
  "pdf_path": null,
  "diagnostics": [
    {
      "severity": "error",
      "message": "Unknown command: \\foobar",
      "span": {"start_byte": 24, "end_byte": 31, "file": "main.tex"},
      "suggestion": "Did you mean \\textbf?",
      "related": []
    }
  ]
}
```

## Security Constraints

The following inputs are rejected before compilation:

| Check | Rejected Input | Error Code |
|-------|---------------|-----------|
| Path traversal | `../etc/passwd` | SEC_001 |
| Absolute path | `/etc/passwd` | SEC_002 |
| Null byte | `main\0.tex` | SEC_003 |
| Control characters | Bytes 0x00-0x1F (except tab/newline) | SEC_004 |
| Oversized document | > 10 MB | SEC_005 |
| Too many documents | > 100 in one request | SEC_006 |

## Integration Guide

### Node.js

```javascript
const { spawn } = require('child_process');

function compile(texSource) {
  return new Promise((resolve, reject) => {
    const proc = spawn('./flashtex-compiler');
    let output = '';

    proc.stdout.on('data', (chunk) => { output += chunk; });
    proc.on('close', () => resolve(JSON.parse(output)));
    proc.on('error', reject);

    proc.stdin.write(JSON.stringify({
      protocol_version: 1,
      type: 'compile',
      id: 'req-' + Date.now(),
      payload: {
        documents: [{ path: 'main.tex', text: texSource }]
      }
    }));
    proc.stdin.end();
  });
}

// Usage
compile('\\section{Hello}World').then(result => {
  console.log(`Compiled in ${result.compile_time_us}μs`);
  console.log(`PDF at: ${result.pdf_path}`);
});
```

### Python

```python
import subprocess
import json

def compile_tex(source: str) -> dict:
    request = json.dumps({
        "protocol_version": 1,
        "type": "compile",
        "id": "py-1",
        "payload": {
            "documents": [{"path": "main.tex", "text": source}]
        }
    })

    result = subprocess.run(
        ["./flashtex-compiler"],
        input=request,
        capture_output=True,
        text=True,
    )

    return json.loads(result.stdout)

# Usage
result = compile_tex(r"\section{Hello}World")
print(f"Compiled in {result['compile_time_us']}μs")
print(f"Pages: {len(result['pages'])}")
```

### Rust

```rust
use std::process::{Command, Stdio};
use std::io::Write;

fn compile(source: &str) -> serde_json::Value {
    let request = serde_json::json!({
        "protocol_version": 1,
        "type": "compile",
        "id": "rs-1",
        "payload": {
            "documents": [{"path": "main.tex", "text": source}]
        }
    });

    let mut child = Command::new("./flashtex-compiler")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to spawn compiler");

    child.stdin.as_mut().unwrap()
        .write_all(request.to_string().as_bytes())
        .unwrap();

    let output = child.wait_with_output().unwrap();
    serde_json::from_slice(&output.stdout).unwrap()
}
```

## Versioning

The protocol version is specified in every request (`protocol_version: 1`). Future versions will be backwards-compatible within the same major version. Breaking changes will increment the protocol version number.

Current version: **1** (initial release, SpaceXAI Hackathon 2026).
