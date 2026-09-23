//! The one real [`Fetcher`]: blocking `reqwest` over rustls, the same
//! configuration `crates/bridge` uses, with bounded timeouts and a bounded
//! body. `file://` URLs are served from disk (a directory answers with its
//! `index.html`) so the CLI and the project-files helper can be exercised
//! against an on-disk fake archive without a socket — never by a unit test
//! of this crate, which uses the in-memory fake instead.

use std::path::PathBuf;
use std::time::Duration;

use crate::{FetchError, Fetcher};

/// Per-request timeout: a package file is small; a mirror that takes
/// longer than this is one to give up on.
pub const REQUEST_TIMEOUT_SECS: u64 = 30;
pub const CONNECT_TIMEOUT_SECS: u64 = 10;
/// The largest body accepted (a `.sty` is kilobytes; a listing at most a
/// few hundred kilobytes).
pub const MAX_BODY_BYTES: usize = 8 * 1024 * 1024;

pub struct HttpFetcher {
    client: reqwest::blocking::Client,
}

impl HttpFetcher {
    pub fn new() -> Result<HttpFetcher, String> {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
            .connect_timeout(Duration::from_secs(CONNECT_TIMEOUT_SECS))
            // mirrors.ctan.org answers every request with a redirect to a mirror.
            .redirect(reqwest::redirect::Policy::limited(5))
            .user_agent(concat!("flashtex-package-resolver/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| format!("cannot initialise the HTTPS client: {e}"))?;
        Ok(HttpFetcher { client })
    }

    fn get_file(path: &str) -> Result<Vec<u8>, FetchError> {
        let mut p = PathBuf::from(path);
        if path.ends_with('/') || p.is_dir() {
            p = p.join("index.html");
        }
        match std::fs::read(&p) {
            Ok(b) => Ok(b),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(FetchError::Status(404)),
            Err(e) => Err(FetchError::Transport(format!("{}: {e}", p.display()))),
        }
    }
}

impl Fetcher for HttpFetcher {
    fn get(&self, url: &str) -> Result<Vec<u8>, FetchError> {
        if let Some(path) = url.strip_prefix("file://") {
            return Self::get_file(path);
        }
        if !(url.starts_with("https://") || url.starts_with("http://")) {
            return Err(FetchError::Transport(format!("{url}: only http(s) and file URLs are fetched")));
        }
        let response = self.client.get(url).send().map_err(|e| FetchError::Transport(describe(&e)))?;
        let status = response.status();
        if !status.is_success() {
            return Err(FetchError::Status(status.as_u16()));
        }
        if let Some(len) = response.content_length() {
            if len > MAX_BODY_BYTES as u64 {
                return Err(FetchError::Transport(format!("{url}: {len} bytes exceeds the {MAX_BODY_BYTES}-byte limit")));
            }
        }
        let bytes = response.bytes().map_err(|e| FetchError::Transport(describe(&e)))?;
        if bytes.len() > MAX_BODY_BYTES {
            return Err(FetchError::Transport(format!("{url}: body exceeds the {MAX_BODY_BYTES}-byte limit")));
        }
        Ok(bytes.to_vec())
    }
}

/// A reqwest error without the URL repeated (the caller names it).
fn describe(e: &reqwest::Error) -> String {
    if e.is_timeout() {
        "timed out".into()
    } else if e.is_connect() {
        format!("cannot connect: {}", root_cause(e))
    } else {
        root_cause(e)
    }
}

fn root_cause(e: &reqwest::Error) -> String {
    let mut source: &dyn std::error::Error = e;
    while let Some(next) = source.source() {
        source = next;
    }
    source.to_string()
}
