//! A local HTTP/1.1 server for one file, with byte ranges and keep-alive,
//! counting requests and bytes: the fixture that bundle tests and
//! measurements fetch from (no network).

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Default)]
pub struct Counters {
    pub requests: AtomicU64,
    pub bytes: AtomicU64,
    pub connections: AtomicU64,
}

pub struct FixtureServer {
    pub url: String,
    pub counters: Arc<Counters>,
}

impl FixtureServer {
    /// Serve `data` at `http://127.0.0.1:<port>/<name>` on a background thread
    /// for the rest of the process.
    pub fn start(data: Vec<u8>, name: &str) -> std::io::Result<FixtureServer> {
        let l = TcpListener::bind("127.0.0.1:0")?;
        let url = format!("http://{}/{name}", l.local_addr()?);
        let counters = Arc::new(Counters::default());
        let c = counters.clone();
        let data = Arc::new(data);
        std::thread::spawn(move || {
            for s in l.incoming().flatten() {
                let (c, data) = (c.clone(), data.clone());
                c.connections.fetch_add(1, Ordering::SeqCst);
                std::thread::spawn(move || {
                    let _ = serve_conn(s, &data, &c);
                });
            }
        });
        Ok(FixtureServer { url, counters })
    }

    pub fn requests(&self) -> u64 {
        self.counters.requests.load(Ordering::SeqCst)
    }
    pub fn bytes(&self) -> u64 {
        self.counters.bytes.load(Ordering::SeqCst)
    }
}

fn serve_conn(s: TcpStream, data: &[u8], c: &Counters) -> std::io::Result<()> {
    let mut r = BufReader::new(s.try_clone()?);
    let mut w = s;
    loop {
        let mut line = String::new();
        if r.read_line(&mut line)? == 0 {
            return Ok(());
        }
        let mut range: Option<(u64, u64)> = None;
        loop {
            let mut h = String::new();
            r.read_line(&mut h)?;
            let h = h.trim_end();
            if h.is_empty() {
                break;
            }
            if let Some(v) = h
                .strip_prefix("Range: bytes=")
                .or_else(|| h.strip_prefix("range: bytes="))
            {
                if let Some((a, b)) = v.split_once('-') {
                    if let (Ok(a), Ok(b)) = (a.parse(), b.parse()) {
                        range = Some((a, b));
                    }
                }
            }
        }
        c.requests.fetch_add(1, Ordering::SeqCst);
        let (status, body) = match range {
            Some((a, b)) if a <= b && (b as usize) < data.len() => {
                ("206 Partial Content", &data[a as usize..=b as usize])
            }
            Some(_) => ("416 Range Not Satisfiable", &data[..0]),
            None => ("200 OK", data),
        };
        c.bytes.fetch_add(body.len() as u64, Ordering::SeqCst);
        write!(
            w,
            "HTTP/1.1 {status}\r\nContent-Length: {}\r\nAccept-Ranges: bytes\r\n\r\n",
            body.len()
        )?;
        w.write_all(body)?;
        w.flush()?;
    }
}
