//! The PDF byte writer: `pdftex.h`'s `writepdf` and `writezip.c`.
//!
//! This lane's writer is a no-op (DESIGN.md section 12, P2; the PDF backend
//! is ported in P3): pdftex.web still assembles every PDF byte in its buffer
//! and keeps its offsets, which is all its logic depends on, but the bytes are
//! dropped instead of being written or compressed.

use crate::generated::Globals;

impl Globals {
    /// `writepdf(a, b)`: bytes `a..=b` of the PDF buffer.
    pub fn write_pdf(&mut self, _a: i32, _b: i32) {}

    /// `writezip` (writezip.c): compress the PDF buffer into the file. The
    /// C code adds the compressed bytes written to `pdf_gone` and sets
    /// `pdf_stream_length` to the stream's compressed length so far; here
    /// nothing is compressed, so both count the bytes as they are
    /// (`pdf_flush` then empties the buffer).
    pub fn write_zip(&mut self, _finish: bool) {
        self.pdf_gone += self.pdf_ptr as i64;
        self.pdf_stream_length += self.pdf_ptr as i64;
    }

    /// `getc(f)` on a binary file: the next byte, or -1 at the end.
    pub fn getc(&mut self, f: &mut crate::system::ByteFile) -> i32 {
        crate::system::getc(f)
    }
}
