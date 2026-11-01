//! The C library's stdio as makeindex uses it: input files read with
//! `getc`, `fseek` and `fscanf("%d")` (Apple's libc, which is FreeBSD's),
//! and output streams.

pub const EOF: i32 = -1;

/// An input `FILE *` opened with `fopen(name, "rb")`, read whole. A file
/// that cannot be read (a directory) reads as empty, as `getc` on it
/// returns `EOF`.
pub struct InFile {
    data: Vec<u8>,
    pos: i64,
}

impl InFile {
    pub fn new(data: Vec<u8>) -> InFile {
        InFile { data, pos: 0 }
    }

    /// `getc`.
    pub fn getc(&mut self) -> i32 {
        if self.pos >= 0 && (self.pos as usize) < self.data.len() {
            let b = self.data[self.pos as usize];
            self.pos += 1;
            b as i32
        } else {
            EOF
        }
    }

    /// `fseek(fp, off, whence)`: 0 or -1 (a position before the start).
    pub fn fseek(&mut self, off: i64, whence: i32) -> i32 {
        let base = match whence {
            0 => 0,
            1 => self.pos,
            _ => self.data.len() as i64,
        };
        let n = base + off;
        if n < 0 {
            -1
        } else {
            self.pos = n;
            0
        }
    }

    pub fn pos(&self) -> i64 {
        self.pos
    }

    fn peek(&self) -> i32 {
        if self.pos >= 0 && (self.pos as usize) < self.data.len() {
            self.data[self.pos as usize] as i32
        } else {
            EOF
        }
    }

    /// `fscanf(fp, "%d", dst)` as FreeBSD's `__svfscanf` (Apple's libc)
    /// does it: skip white space (`isspace` in the C locale), then an
    /// optional sign and decimal digits, at most 512 characters. Without a
    /// digit nothing is stored and a sign read is pushed back; otherwise
    /// the digits go through `strtoimax` (saturating) and are stored
    /// truncated to `int`.
    pub fn scan_int(&mut self, dst: &mut i32) {
        loop {
            let c = self.peek();
            if c == EOF {
                return;
            }
            if matches!(c as u8, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r') {
                self.pos += 1;
            } else {
                break;
            }
        }
        let mut buf: Vec<u8> = Vec::new();
        let mut sign_ok = true;
        let mut digits = false;
        let mut width = 512;
        while width > 0 {
            let c = self.peek();
            if c == EOF {
                break;
            }
            let b = c as u8;
            if b.is_ascii_digit() {
                sign_ok = false;
                digits = true;
            } else if (b == b'+' || b == b'-') && sign_ok {
                sign_ok = false;
            } else {
                break;
            }
            buf.push(b);
            self.pos += 1;
            width -= 1;
        }
        if !digits {
            if !buf.is_empty() {
                self.pos -= 1;
            }
            return;
        }
        let (neg, ds) = match buf[0] {
            b'-' => (true, &buf[1..]),
            b'+' => (false, &buf[1..]),
            _ => (false, &buf[..]),
        };
        let mut v: i128 = 0;
        for &d in ds {
            v = v * 10 + (d - b'0') as i128;
            if v > i64::MAX as i128 + 1 {
                v = i64::MAX as i128 + 1;
            }
        }
        let v: i64 = if neg {
            (-v).max(i64::MIN as i128) as i64
        } else {
            v.min(i64::MAX as i128) as i64
        };
        *dst = v as i32;
    }
}

/// `mk_getc` (mkind.c): either Unix or Windows line ends. Its one
/// character of lookahead is a single static, shared by every stream.
pub fn mk_getc(lookahead: &mut i32, f: &mut InFile) -> i32 {
    let ch = if *lookahead != -2 {
        *lookahead
    } else {
        f.getc()
    };
    *lookahead = if ch == b'\r' as i32 { f.getc() } else { -2 };
    if *lookahead == b'\n' as i32 {
        *lookahead = -2;
        return b'\n' as i32;
    }
    ch
}

/// An output `FILE *`: a file, or the process's standard output or error.
pub enum Out {
    /// The file and what was written to it, kept until the end: the C
    /// program's `FILE` buffers (BUFSIZ) are lost when it dies of a signal.
    File(std::fs::File, Vec<u8>),
    Stdout,
    Stderr,
}
