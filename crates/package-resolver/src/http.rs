//! The one real [`Fetcher`]: blocking `reqwest` over rustls with the
//! platform trust store, bounded timeouts and a bounded body.
//!
//! TLS roots are the platform's (macOS Security.framework's system roots
//! with intermediate-fetching, via `rustls-platform-verifier`), never
//! rustls's bundled list: `mirrors.ctan.org` answers every request with a
//! redirect to an essentially random real mirror, and some of those
//! mirrors serve chains the bundle cannot build (`UnknownIssuer` where a
//! platform client succeeds).
//!
//! That same redirect lottery is why a `mirrors.ctan.org` URL is attempted
//! up to [`MAX_ATTEMPTS`] times: a transport failure (DNS, connect, TLS,
//! timeout) may be one bad mirror, so the next attempt rolls the redirector
//! again — except the final one, which targets the fixed mirror
//! [`FALLBACK_MIRROR_ROOT`] instead. Only the final attempt's failure is
//! ever returned, so the caller names it exactly once. A status answer
//! (e.g. 404) came from a real server and is never retried.
//!
//! `file://` URLs are served from disk (a directory answers with its
//! `index.html`) so the CLI and the project-files helper can be exercised
//! against an on-disk fake archive without a socket — never by a unit test
//! of this crate, which uses the in-memory fake instead.

use std::path::PathBuf;
use std::time::Duration;

use rustls_platform_verifier::BuilderVerifierExt;

use crate::source::CTAN_MIRROR;
use crate::{FetchError, Fetcher};

/// Per-request timeout: a package file is small; a mirror that takes
/// longer than this is one to give up on.
pub const REQUEST_TIMEOUT_SECS: u64 = 30;
pub const CONNECT_TIMEOUT_SECS: u64 = 10;
/// The largest body accepted (a `.sty` is kilobytes; a listing at most a
/// few hundred kilobytes).
pub const MAX_BODY_BYTES: usize = 8 * 1024 * 1024;
/// Total attempts for one `mirrors.ctan.org` URL: two rolls of the
/// redirector absorb a single bad-mirror landing, and the third (final)
/// attempt goes to the fixed fallback mirror instead, so a bad draw can
/// never loop forever. Three bounds the worst case at roughly three
/// request timeouts while still surviving one bad mirror.
pub const MAX_ATTEMPTS: usize = 3;
/// The fixed mirror the final attempt of a `mirrors.ctan.org` URL targets:
/// the University of Illinois CTAN mirror, a long-standing host that
/// serves the archive root directly under `https://` (so a redirector URL
/// maps onto it by host swap alone) with a platform-trusted certificate.
pub const FALLBACK_MIRROR_ROOT: &str = "https://ctan.math.illinois.edu";

pub struct HttpFetcher {
    client: reqwest::blocking::Client,
}

impl HttpFetcher {
    pub fn new() -> Result<HttpFetcher, String> {
        // The provider is chosen explicitly (ring, the same one reqwest
        // itself uses) rather than relying on rustls's process-default
        // provider, so this library never depends on or disturbs the host
        // process's global choice.
        let provider = rustls::crypto::ring::default_provider();
        let tls = rustls::ClientConfig::builder_with_provider(provider.into())
            .with_protocol_versions(rustls::DEFAULT_VERSIONS)
            .map_err(|e| format!("cannot initialise TLS protocol versions: {e}"))?
            .with_platform_verifier()
            .map_err(|e| format!("cannot initialise the platform TLS verifier: {e}"))?
            .with_no_client_auth();
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
            .connect_timeout(Duration::from_secs(CONNECT_TIMEOUT_SECS))
            // mirrors.ctan.org answers every request with a redirect to a mirror.
            .redirect(reqwest::redirect::Policy::limited(5))
            .user_agent(concat!("flashtex-package-resolver/", env!("CARGO_PKG_VERSION")))
            .use_preconfigured_tls(tls)
            .build()
            .map_err(|e| format!("cannot initialise the HTTPS client: {e}"))?;
        Ok(HttpFetcher { client })
    }

    /// One `GET` with no retry: the status/body rules shared by every attempt.
    fn fetch_once(&self, url: &str) -> Result<Vec<u8>, FetchError> {
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
        fetch_with_retry(url, &|next| self.fetch_once(next))
    }
}

/// The URLs one `get` attempts in order: a `mirrors.ctan.org` URL is tried
/// up to [`MAX_ATTEMPTS`] times — every attempt but the last rolls the
/// redirector again, and the last targets [`FALLBACK_MIRROR_ROOT`] — while
/// any other URL is attempted exactly once.
pub fn candidate_urls(url: &str) -> Vec<String> {
    if !is_ctan_redirector_url(url) {
        return vec![url.to_owned()];
    }
    let mut urls = Vec::with_capacity(MAX_ATTEMPTS.max(1));
    for _ in 1..MAX_ATTEMPTS {
        urls.push(url.to_owned());
    }
    urls.push(fallback_url(url));
    urls
}

/// Tries `url`'s [`candidate_urls`] in order with `fetch`, retrying only
/// transport failures (DNS, connect, TLS chain, timeout — the "cannot
/// connect" class) and stopping at the first success, the first status
/// answer, or the exhausted list. The error of an exhausted list is the
/// final attempt's failure alone, never a concatenation of every attempt.
pub fn fetch_with_retry(url: &str, fetch: &dyn Fn(&str) -> Result<Vec<u8>, FetchError>) -> Result<Vec<u8>, FetchError> {
    let urls = candidate_urls(url);
    let mut last: Option<FetchError> = None;
    for next in &urls {
        match fetch(next) {
            Ok(body) => return Ok(body),
            Err(e) if is_retriable(&e) => last = Some(e),
            Err(e) => return Err(e),
        }
    }
    Err(last.expect("candidate_urls always yields at least one URL"))
}

/// Only the "cannot connect" class retries: a status answer came from a
/// real server (a 404 is a legitimate "no such package"), so retrying it
/// against another mirror could only hide the answer.
fn is_retriable(error: &FetchError) -> bool {
    matches!(error, FetchError::Transport(_))
}

fn is_ctan_redirector_url(url: &str) -> bool {
    url == CTAN_MIRROR || url.starts_with(&format!("{CTAN_MIRROR}/"))
}

/// `url` (a redirector URL) rewritten onto the fixed fallback mirror,
/// preserving the archive path: the fallback serves the archive root
/// directly, so only the host changes.
fn fallback_url(url: &str) -> String {
    format!("{FALLBACK_MIRROR_ROOT}{}", &url[CTAN_MIRROR.len()..])
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
