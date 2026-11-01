//! Retry and mirror-fallback logic in [`http`](flashtex_package_resolver::http),
//! with no socket: every test drives
//! [`fetch_with_retry`](flashtex_package_resolver::http::fetch_with_retry)
//! with a scripted `fetch` closure that records the URLs it was asked for.

use std::cell::RefCell;

use flashtex_package_resolver::http::{candidate_urls, fetch_with_retry, FALLBACK_MIRROR_ROOT, MAX_ATTEMPTS};
use flashtex_package_resolver::FetchError;

const URL: &str = "https://mirrors.ctan.org/macros/latex/contrib/cancel/cancel.sty";

/// A scripted transport: each attempt consumes the next queued result in
/// order (a test that runs out of queued results fails loudly rather than
/// inventing an answer), and every attempted URL is recorded.
struct Script {
    results: RefCell<Vec<Result<Vec<u8>, FetchError>>>,
    attempted: RefCell<Vec<String>>,
}

impl Script {
    fn new(results: Vec<Result<Vec<u8>, FetchError>>) -> Self {
        Script { results: RefCell::new(results), attempted: RefCell::new(Vec::new()) }
    }

    fn fetch(&self, url: &str) -> Result<Vec<u8>, FetchError> {
        self.attempted.borrow_mut().push(url.to_string());
        if self.results.borrow().is_empty() {
            panic!("more attempts than queued results");
        }
        self.results.borrow_mut().remove(0)
    }
}

fn transport(message: &str) -> Result<Vec<u8>, FetchError> {
    Err(FetchError::Transport(message.to_string()))
}

#[test]
fn redirector_urls_try_redirector_then_fallback_while_others_try_once() {
    assert_eq!(MAX_ATTEMPTS, 3);
    let urls = candidate_urls(URL);
    assert_eq!(urls.len(), MAX_ATTEMPTS);
    assert_eq!(&urls[0], URL);
    assert_eq!(&urls[1], URL, "earlier attempts roll the redirector again");
    assert_eq!(urls[2], format!("{FALLBACK_MIRROR_ROOT}/macros/latex/contrib/cancel/cancel.sty"), "the final attempt pins a fixed mirror, keeping the archive path");
    assert!(!urls[2].contains("mirrors.ctan.org"));

    assert_eq!(candidate_urls("https://ctan.org/json/2.0/pkg/cancel"), ["https://ctan.org/json/2.0/pkg/cancel"]);
    assert_eq!(candidate_urls("https://registry.example/tex/macros/latex/contrib/p/p.sty"), ["https://registry.example/tex/macros/latex/contrib/p/p.sty"]);
}

#[test]
fn transport_failures_then_a_fallback_success_return_the_fallback_bytes() {
    let script = Script::new(vec![
        transport("cannot connect: UnknownIssuer"),
        transport("cannot connect: Connection reset"),
        Ok(b"% fallback bytes".to_vec()),
    ]);
    let body = fetch_with_retry(URL, &|next| script.fetch(next)).expect("the fallback succeeds");
    assert_eq!(body, b"% fallback bytes");
    let attempted = script.attempted.borrow();
    assert_eq!(attempted.len(), MAX_ATTEMPTS);
    assert_eq!(&attempted[2], &candidate_urls(URL)[2], "the success came from the fallback mirror");
    assert!(attempted[2].starts_with(FALLBACK_MIRROR_ROOT));
}

#[test]
fn exhausting_every_attempt_reports_only_the_final_failure() {
    let script = Script::new(vec![transport("first: UnknownIssuer"), transport("second: Connection reset"), transport("final: timed out")]);
    let Err(FetchError::Transport(message)) = fetch_with_retry(URL, &|next| script.fetch(next)) else {
        panic!("all attempts fail");
    };
    assert_eq!(script.attempted.borrow().len(), MAX_ATTEMPTS);
    assert_eq!(message, "final: timed out");
    assert!(!message.contains("first") && !message.contains("second"), "one failure, named once: {message}");
}

#[test]
fn a_status_answer_is_never_retried() {
    let script = Script::new(vec![Err(FetchError::Status(404))]);
    let Err(FetchError::Status(404)) = fetch_with_retry(URL, &|next| script.fetch(next)) else {
        panic!("a 404 stays a 404");
    };
    assert_eq!(script.attempted.borrow().len(), 1, "a legitimate answer is not replayed against more mirrors");
}
