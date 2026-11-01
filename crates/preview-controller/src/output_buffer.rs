//! JSONL buffer whose byte budget includes the newline and bounds requested growth.
use std::io::{self, Write};

/// Batch small serializer writes; return only a complete, bounded JSONL frame.
pub fn serialize<T: serde::Serialize + ?Sized>(value: &T, limit: usize) -> io::Result<Vec<u8>> {
    let mut writer = io::BufWriter::with_capacity(8192, OutputBuffer::new(limit));
    serde_json::to_writer(&mut writer, value).map_err(io::Error::other)?;
    writer
        .into_inner()
        .map_err(|error| error.into_error())?
        .finish()
}

pub struct OutputBuffer {
    bytes: Vec<u8>,
    limit: usize,
}
impl OutputBuffer {
    pub fn new(limit: usize) -> Self {
        Self {
            bytes: Vec::new(),
            limit,
        }
    }
    fn reserve(&mut self, required: usize) -> io::Result<()> {
        if required > self.limit {
            return Err(io::Error::other("output frame exceeds byte budget"));
        }
        if required > self.bytes.capacity() {
            let target = required
                .max(self.bytes.capacity().saturating_mul(2))
                .min(self.limit);
            self.bytes
                .try_reserve_exact(target - self.bytes.len())
                .map_err(io::Error::other)?;
        }
        Ok(())
    }
    pub fn finish(mut self) -> io::Result<Vec<u8>> {
        let required = self
            .bytes
            .len()
            .checked_add(1)
            .ok_or_else(|| io::Error::other("output size overflow"))?;
        self.reserve(required)?;
        self.bytes.push(b'\n');
        Ok(self.bytes)
    }
}
impl Write for OutputBuffer {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let required = self
            .bytes
            .len()
            .checked_add(bytes.len())
            .and_then(|n| n.checked_add(1))
            .ok_or_else(|| io::Error::other("output size overflow"))?;
        // Reserve the delimiter now so finish cannot double a full JSON buffer.
        self.reserve(required)?;
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_jsonl_limit_and_rejected_write_preserve_prior_bytes() {
        let mut buffer = OutputBuffer::new(16);
        buffer.write_all(b"123456789012345").unwrap();
        assert!(buffer.write_all(b"x").is_err());
        assert!(buffer.bytes.capacity() <= 16);
        assert_eq!(buffer.finish().unwrap(), b"123456789012345\n");
        assert!(OutputBuffer::new(0).finish().is_err());
        assert_eq!(OutputBuffer::new(1).finish().unwrap(), b"\n");
    }
    #[test]
    fn irregular_growth_never_requests_more_than_frame_budget() {
        let mut buffer = OutputBuffer::new(1000);
        for chunk in [300, 301, 200, 198] {
            buffer.write_all(&vec![b'x'; chunk]).unwrap();
            assert!(buffer.bytes.capacity() <= 1000);
        }
        let frame = buffer.finish().unwrap();
        assert_eq!(frame.len(), 1000);
        assert!(frame.capacity() <= 1000);
        assert_eq!(frame.last(), Some(&b'\n'));
    }
}
