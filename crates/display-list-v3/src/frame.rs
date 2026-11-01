//! Framing (spec §2): `u32` little-endian length of what follows, one kind
//! byte, then the body. A file of display lists is the same frames back to
//! back.

use std::io::{self, Read, Write};

/// Largest frame accepted (kind byte + body), 256 MiB: a font program or a
/// page of a few million glyphs fits; anything larger is a corrupt stream.
pub const MAX_FRAME: u32 = 1 << 28;

/// Write one frame.
pub fn write_frame(w: &mut impl Write, kind: u8, body: &[u8]) -> io::Result<()> {
    let len = body.len() as u64 + 1;
    if len > MAX_FRAME as u64 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "frame too large",
        ));
    }
    let mut head = [0u8; 5];
    head[..4].copy_from_slice(&(len as u32).to_le_bytes());
    head[4] = kind;
    w.write_all(&head)?;
    w.write_all(body)
}

/// Read one frame: `Ok(None)` at a clean end of stream (no partial header).
pub fn read_frame(r: &mut impl Read) -> io::Result<Option<(u8, Vec<u8>)>> {
    let mut head = [0u8; 5];
    let mut got = 0;
    while got < 5 {
        let n = r.read(&mut head[got..])?;
        if n == 0 {
            if got == 0 {
                return Ok(None);
            }
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "truncated frame header",
            ));
        }
        got += n;
    }
    let len = u32::from_le_bytes(head[..4].try_into().unwrap());
    if len == 0 || len > MAX_FRAME {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("bad frame length {len}"),
        ));
    }
    let mut body = vec![0u8; len as usize - 1];
    r.read_exact(&mut body)?;
    Ok(Some((head[4], body)))
}

/// Split a byte buffer into complete frames, returning how many bytes were
/// consumed (a relay forwards whole frames only).
pub fn complete_frames(buf: &[u8]) -> Result<usize, String> {
    let mut i = 0;
    while buf.len() - i >= 5 {
        let len = u32::from_le_bytes(buf[i..i + 4].try_into().unwrap());
        if len == 0 || len > MAX_FRAME {
            return Err(format!("bad frame length {len}"));
        }
        let end = i + 4 + len as usize;
        if end > buf.len() {
            break;
        }
        i = end;
    }
    Ok(i)
}

/// Little-endian readers over a byte slice, failing closed.
pub struct Cursor<'a> {
    pub b: &'a [u8],
    pub i: usize,
}

impl<'a> Cursor<'a> {
    pub fn new(b: &'a [u8]) -> Self {
        Cursor { b, i: 0 }
    }
    pub fn left(&self) -> usize {
        self.b.len() - self.i
    }
    pub fn take(&mut self, n: usize) -> Result<&'a [u8], String> {
        if self.left() < n {
            return Err(format!("truncated at byte {} (need {n})", self.i));
        }
        let s = &self.b[self.i..self.i + n];
        self.i += n;
        Ok(s)
    }
    pub fn u8(&mut self) -> Result<u8, String> {
        Ok(self.take(1)?[0])
    }
    pub fn u16(&mut self) -> Result<u16, String> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().unwrap()))
    }
    pub fn u32(&mut self) -> Result<u32, String> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    pub fn i32(&mut self) -> Result<i32, String> {
        Ok(i32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    pub fn f64(&mut self) -> Result<f64, String> {
        Ok(f64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }
    /// A `u32` count, checked against the bytes left (each element at least
    /// `min` bytes), so a corrupt count cannot make a decoder allocate.
    pub fn count(&mut self, min: usize) -> Result<usize, String> {
        let n = self.u32()? as usize;
        if n.saturating_mul(min.max(1)) > self.left() {
            return Err(format!("count {n} exceeds the data at byte {}", self.i));
        }
        Ok(n)
    }
}

/// Little-endian writers.
pub trait Put {
    fn put_u8(&mut self, v: u8);
    fn put_u16(&mut self, v: u16);
    fn put_u32(&mut self, v: u32);
    fn put_i32(&mut self, v: i32);
    fn put_f64(&mut self, v: f64);
}

impl Put for Vec<u8> {
    fn put_u8(&mut self, v: u8) {
        self.push(v)
    }
    fn put_u16(&mut self, v: u16) {
        self.extend_from_slice(&v.to_le_bytes())
    }
    fn put_u32(&mut self, v: u32) {
        self.extend_from_slice(&v.to_le_bytes())
    }
    fn put_i32(&mut self, v: i32) {
        self.extend_from_slice(&v.to_le_bytes())
    }
    fn put_f64(&mut self, v: f64) {
        self.extend_from_slice(&v.to_le_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_round_trip() {
        let mut v = Vec::new();
        write_frame(&mut v, 7, b"abc").unwrap();
        write_frame(&mut v, 8, b"").unwrap();
        assert_eq!(complete_frames(&v).unwrap(), v.len());
        assert_eq!(complete_frames(&v[..v.len() - 1]).unwrap(), 8);
        let mut r = &v[..];
        assert_eq!(read_frame(&mut r).unwrap(), Some((7, b"abc".to_vec())));
        assert_eq!(read_frame(&mut r).unwrap(), Some((8, vec![])));
        assert_eq!(read_frame(&mut r).unwrap(), None);
        let mut r = &v[..3];
        assert!(read_frame(&mut r).is_err());
    }
}
