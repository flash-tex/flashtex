//! A read-only C `FILE *`, for the ports of the image readers
//! (`writejpg.c`, `writejbig2.c`, `writepng.c`'s `copy_png`), which read
//! their files with `getc`, `fread`, `fseek`, `ftell` and `feof`, and whose
//! behaviour at and beyond the end of a file (malformed images) depends on
//! exactly how those behave:
//!
//! * `getc` at the end returns `EOF` (-1) and sets the end-of-file flag;
//! * `fseek` may go past the end (a later read then fails), never before
//!   the start, and clears the end-of-file flag;
//! * `fread` returns the number of bytes read and sets the flag on a short
//!   read.
//!
//! The file is read into memory when it is opened; image files are read
//! whole by pdfTeX anyway.

/// `SEEK_SET`, `SEEK_CUR`, `SEEK_END`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Whence {
    Set,
    Cur,
    End,
}

pub struct CFile {
    data: Vec<u8>,
    pos: u64,
    eof: bool,
}

impl CFile {
    /// `fopen(name, "rb")`.
    pub fn open(name: &[u8]) -> Option<CFile> {
        std::fs::read(os_path(name)).ok().map(|data| CFile {
            data,
            pos: 0,
            eof: false,
        })
    }

    /// A file with these contents.
    pub fn from_bytes(data: Vec<u8>) -> CFile {
        CFile {
            data,
            pos: 0,
            eof: false,
        }
    }

    /// `getc`.
    pub fn getc(&mut self) -> i32 {
        match usize::try_from(self.pos)
            .ok()
            .and_then(|p| self.data.get(p))
        {
            Some(&b) => {
                self.pos += 1;
                b as i32
            }
            _ => {
                self.eof = true;
                -1
            }
        }
    }

    /// `fread(buf, 1, buf.len(), f)`: the number of bytes read into `buf`.
    pub fn read(&mut self, buf: &mut [u8]) -> usize {
        let start = (self.pos as usize).min(self.data.len());
        let n = buf.len().min(self.data.len() - start);
        buf[..n].copy_from_slice(&self.data[start..start + n]);
        self.pos += n as u64;
        if n < buf.len() {
            self.eof = true;
        }
        n
    }

    /// `fseek`: false (and no move) if the new position would be negative.
    pub fn seek(&mut self, offset: i64, whence: Whence) -> bool {
        let base = match whence {
            Whence::Set => 0i64,
            Whence::Cur => self.pos as i64,
            Whence::End => self.data.len() as i64,
        };
        match base.checked_add(offset) {
            Some(p) if p >= 0 => {
                self.pos = p as u64;
                self.eof = false;
                true
            }
            _ => false,
        }
    }

    /// `ftell`.
    pub fn tell(&self) -> i64 {
        self.pos as i64
    }

    /// `feof`.
    pub fn feof(&self) -> bool {
        self.eof
    }

    /// The file's length.
    pub fn len(&self) -> u64 {
        self.data.len() as u64
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

/// A file name as the C code sees it (bytes) as a path.
pub fn os_path(name: &[u8]) -> std::path::PathBuf {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        std::path::PathBuf::from(std::ffi::OsStr::from_bytes(name))
    }
    #[cfg(not(unix))]
    {
        std::path::PathBuf::from(String::from_utf8_lossy(name).into_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eof_and_seek_follow_c() {
        let mut f = CFile::from_bytes(vec![1, 2]);
        assert_eq!(f.getc(), 1);
        assert_eq!(f.getc(), 2);
        assert!(!f.feof());
        assert_eq!(f.getc(), -1);
        assert!(f.feof());
        assert!(f.seek(10, Whence::Set));
        assert!(!f.feof());
        assert_eq!(f.getc(), -1);
        assert!(!f.seek(-11, Whence::Cur));
        assert_eq!(f.tell(), 10);
        let mut b = [0u8; 4];
        assert!(f.seek(1, Whence::Set));
        assert_eq!(f.read(&mut b), 1);
        assert!(f.feof());
    }
}
