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

/// Lane BEAMER: `instances_at` puts back, for each file opened before `n`
/// and again from `n` on, its last instance before `n`; it refuses one
/// open for output at `n` or whose instance was not kept.
#[test]
fn instances_at_gives_the_last_instance_before_n() {
    let arc = |s: &str| Some(std::sync::Arc::new(s.as_bytes().to_vec()));
    truncate_opens(0);
    append_opens_kept(
        &["a.vrb", "x.aux", "a.vrb", "a.vrb", "y.toc"].map(String::from),
        &[arc("one"), None, arc("two"), None],
    );
    // before any open, after the first, between the second and the third
    assert_eq!(instances_at(0, &[]), Ok(vec![]));
    let got = instances_at(1, &[]).unwrap();
    assert_eq!(got.len(), 1);
    assert_eq!(
        (got[0].0.as_str(), got[0].1.as_slice()),
        ("a.vrb", &b"one"[..])
    );
    let got = instances_at(3, &[]).unwrap();
    assert_eq!(got[0].1.as_slice(), b"two");
    // open for output there: its instance is not done with
    assert_eq!(
        instances_at(3, &["a.vrb".to_string()]),
        Err("a.vrb".to_string())
    );
    // after the last open: nothing to put back
    assert_eq!(instances_at(4, &[]), Ok(vec![]));
    // an instance not kept
    truncate_opens(3);
    forget_open_instances(&["a.vrb".to_string()]);
    append_opens(&["a.vrb".to_string()]);
    assert_eq!(instances_at(3, &[]), Err("a.vrb".to_string()));
    truncate_opens(0);
}

/// `instances_at` on `rewritten_at_is_cheap`'s opens, each instance kept.
#[test]
fn instances_at_is_cheap() {
    let mut o: Vec<String> = (0..2600).map(|i| format!("chapter{i}.aux")).collect();
    for k in 0..400 {
        o.insert(k * 7, "doc-tmp.tex".to_string());
    }
    let kept: Vec<_> = o
        .iter()
        .map(|_| Some(std::sync::Arc::new(vec![0u8; 16])))
        .collect();
    truncate_opens(0);
    append_opens_kept(&o, &kept);
    let t = std::time::Instant::now();
    let mut put = 0;
    for n in 0..3000 {
        put += instances_at(n, &[]).unwrap().len();
    }
    let per = t.elapsed().as_secs_f64() / 3000.0;
    eprintln!(
        "instances_at: {:.3} us a question (3,000 opens), {put} put back",
        per * 1e6
    );
    assert!(put > 2000);
    assert!(per < 1e-4, "{per} s a question");
    truncate_opens(0);
}
