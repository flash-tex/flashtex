// Tests of `system::rewritten_at` (the restart point's rewritten-output
// rule, `crate::checkpoint`'s `rewritten_since`).

use super::*;

fn set_opens(v: Vec<String>) {
    OPENS.with(|o| *o.borrow_mut() = v);
    opens_changed();
}

/// The rule's first form: a file among the opens before `n` and among
/// those from `n` on.
fn brute(o: &[String], n: usize) -> bool {
    let before: std::collections::HashSet<&String> =
        o[..n].iter().filter(|p| !p.is_empty()).collect();
    o[n..].iter().any(|p| before.contains(p))
}

#[test]
fn rewritten_at_is_the_brute_rule() {
    let mut seed = 7u64;
    let mut next = move |m: u64| {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (seed >> 33) % m
    };
    for _ in 0..200 {
        let len = next(40) as usize;
        let o: Vec<String> = (0..len)
            .map(|_| match next(8) {
                0 => String::new(),
                k => format!("f{}", k % 5),
            })
            .collect();
        set_opens(o.clone());
        for n in 0..=len {
            assert_eq!(rewritten_at(n).is_some(), brute(&o, n), "{o:?} at {n}");
        }
    }
}

/// The restart point's question on a large document: 3,000 output opens (a
/// temporary file rewritten 400 times among 2,600 opens of other files),
/// asked for each of 3,000 checkpoints after one change to the opens (the
/// index is rebuilt once). Prints the cost of a question.
#[test]
fn rewritten_at_is_cheap() {
    let mut o: Vec<String> = (0..2600).map(|i| format!("chapter{i}.aux")).collect();
    for k in 0..400 {
        o.insert(k * 7, "doc-tmp.tex".to_string());
    }
    set_opens(o);
    let t = std::time::Instant::now();
    let mut hits = 0;
    for n in 0..3000 {
        hits += rewritten_at(n).is_some() as usize;
    }
    let per = t.elapsed().as_secs_f64() / 3000.0;
    eprintln!(
        "rewritten_at: {:.3} us a question (3,000 opens, one rebuild), {hits} unsafe",
        per * 1e6
    );
    assert!(hits > 2000);
    assert!(per < 1e-4, "{per} s a question");
}
