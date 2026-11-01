//! Byte ranges of a bundle: from a local file, over plain HTTP/1.1 with one
//! kept-alive connection, or over HTTPS through the system's `curl`.
//!
//! HTTPS is `curl` (present on every macOS and on the Linux systems TeX Live
//! supports) rather than a TLS stack linked into the engine: the engine is
//! GPL and would carry it, and a bundle fetch is a one-off per file on a
//! cold cache, where a process start (a few ms) is small against the network
//! round trip. Plain HTTP is built in because it is small, is what a local
//! mirror or the tests' fixture server speaks, and lets requests be counted
//! exactly.

use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::net::TcpStream;
use std::time::Duration;

/// A source of byte ranges.
pub trait RangeSource: Send {
    /// Exactly `len` bytes from `start`.
    fn read_range(&mut self, start: u64, len: u64) -> Result<Vec<u8>, String>;
    fn describe(&self) -> String;
}

/// Open `url`: `file://PATH` or an absolute path, `http://`, or `https://`.
pub fn open(url: &str) -> Result<Box<dyn RangeSource>, String> {
    if let Some(p) = url.strip_prefix("file://") {
        return Ok(Box::new(FileSource::open(p)?));
    }
    if url.starts_with('/') {
        return Ok(Box::new(FileSource::open(url)?));
    }
    if url.starts_with("http://") {
        return Ok(Box::new(HttpSource::new(url)?));
    }
    if url.starts_with("https://") {
        return Ok(Box::new(CurlSource { url: url.into() }));
    }
    Err(format!("unsupported bundle URL {url}"))
}

pub struct FileSource {
    path: String,
    f: File,
}

impl FileSource {
    pub fn open(p: &str) -> Result<FileSource, String> {
        Ok(FileSource {
            path: p.into(),
            f: File::open(p).map_err(|e| format!("{p}: {e}"))?,
        })
    }
}

impl RangeSource for FileSource {
    fn read_range(&mut self, start: u64, len: u64) -> Result<Vec<u8>, String> {
        let mut v = vec![0u8; len as usize];
        self.f
            .seek(SeekFrom::Start(start))
            .and_then(|_| self.f.read_exact(&mut v))
            .map_err(|e| format!("{}: {e}", self.path))?;
        Ok(v)
    }
    fn describe(&self) -> String {
        format!("file {}", self.path)
    }
}

/// HTTP/1.1 range requests over one persistent connection.
pub struct HttpSource {
    url: String,
    host: String,
    addr: String,
    path: String,
    conn: Option<BufReader<TcpStream>>,
}

impl HttpSource {
    pub fn new(url: &str) -> Result<HttpSource, String> {
        let rest = url.strip_prefix("http://").ok_or("not an http:// URL")?;
        let (hostport, path) = match rest.find('/') {
            Some(i) => (&rest[..i], &rest[i..]),
            None => (rest, "/"),
        };
        let addr = if hostport.contains(':') {
            hostport.to_string()
        } else {
            format!("{hostport}:80")
        };
        Ok(HttpSource {
            url: url.into(),
            host: hostport.into(),
            addr,
            path: path.into(),
            conn: None,
        })
    }

    fn connect(&mut self) -> Result<&mut BufReader<TcpStream>, String> {
        if self.conn.is_none() {
            let s = TcpStream::connect(&self.addr).map_err(|e| format!("{}: {e}", self.url))?;
            let _ = s.set_read_timeout(Some(Duration::from_secs(60)));
            let _ = s.set_nodelay(true);
            self.conn = Some(BufReader::new(s));
        }
        Ok(self.conn.as_mut().unwrap())
    }

    fn try_range(&mut self, start: u64, len: u64) -> Result<Vec<u8>, String> {
        let req = format!(
            "GET {} HTTP/1.1\r\nHost: {}\r\nRange: bytes={}-{}\r\nUser-Agent: flashtex\r\nAccept-Encoding: identity\r\n\r\n",
            self.path,
            self.host,
            start,
            start + len - 1
        );
        let url = self.url.clone();
        let c = self.connect()?;
        c.get_mut()
            .write_all(req.as_bytes())
            .map_err(|e| format!("{url}: {e}"))?;
        let mut status = String::new();
        c.read_line(&mut status)
            .map_err(|e| format!("{url}: {e}"))?;
        if status.is_empty() {
            return Err(format!("{url}: connection closed"));
        }
        let code: u32 = status
            .split_whitespace()
            .nth(1)
            .and_then(|c| c.parse().ok())
            .unwrap_or(0);
        let mut clen: Option<u64> = None;
        let mut close = false;
        loop {
            let mut h = String::new();
            c.read_line(&mut h).map_err(|e| format!("{url}: {e}"))?;
            let h = h.trim_end();
            if h.is_empty() {
                break;
            }
            if let Some((k, v)) = h.split_once(':') {
                let k = k.trim().to_ascii_lowercase();
                let v = v.trim();
                if k == "content-length" {
                    clen = v.parse().ok();
                } else if k == "connection" && v.eq_ignore_ascii_case("close") {
                    close = true;
                } else if k == "transfer-encoding" && !v.eq_ignore_ascii_case("identity") {
                    return Err(format!("{url}: transfer-encoding {v} is not supported"));
                }
            }
        }
        let clen = clen.ok_or_else(|| format!("{url}: no Content-Length"))?;
        let mut body = vec![0u8; clen as usize];
        c.read_exact(&mut body).map_err(|e| format!("{url}: {e}"))?;
        if close {
            self.conn = None;
        }
        match code {
            206 if clen == len => Ok(body),
            // A server without range support sends everything.
            200 if clen >= start + len => Ok(body[start as usize..(start + len) as usize].to_vec()),
            _ => Err(format!("{url}: HTTP {code} for bytes {start}+{len}")),
        }
    }
}

impl RangeSource for HttpSource {
    fn read_range(&mut self, start: u64, len: u64) -> Result<Vec<u8>, String> {
        if len == 0 {
            return Ok(vec![]);
        }
        // A kept-alive connection the server has since closed fails on
        // first use; one retry on a fresh connection.
        match self.try_range(start, len) {
            Ok(v) => Ok(v),
            Err(_) => {
                self.conn = None;
                self.try_range(start, len)
            }
        }
    }
    fn describe(&self) -> String {
        self.url.clone()
    }
}

/// HTTPS through `/usr/bin/curl` (or `curl` on PATH).
pub struct CurlSource {
    url: String,
}

impl RangeSource for CurlSource {
    fn read_range(&mut self, start: u64, len: u64) -> Result<Vec<u8>, String> {
        if len == 0 {
            return Ok(vec![]);
        }
        let curl = ["/usr/bin/curl", "/bin/curl"]
            .into_iter()
            .find(|p| std::path::Path::new(p).is_file())
            .unwrap_or("curl");
        let out = std::process::Command::new(curl)
            .args(["-sSfL", "--retry", "2", "--range"])
            .arg(format!("{}-{}", start, start + len - 1))
            .arg(&self.url)
            .output()
            .map_err(|e| format!("cannot run curl: {e}"))?;
        if !out.status.success() {
            return Err(format!(
                "curl {}: {}",
                self.url,
                String::from_utf8_lossy(&out.stderr).trim()
            ));
        }
        let v = out.stdout;
        if v.len() as u64 == len {
            Ok(v)
        } else if v.len() as u64 >= start + len {
            Ok(v[start as usize..(start + len) as usize].to_vec())
        } else {
            Err(format!(
                "curl {}: got {} bytes, wanted {len}",
                self.url,
                v.len()
            ))
        }
    }
    fn describe(&self) -> String {
        self.url.clone()
    }
}
