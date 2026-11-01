//! tex.ch's `search_string` (`changes/filenames.ch`) as
//! `changes/throughput.ch` [8] computes it, with the pool scanned by
//! `flashtex_search_pool` (`src/strsearch.rs`), against tex.ch's own loop,
//! transcribed below: the same string (or 0) for every string of pools with
//! many equal strings and many of equal length, including a string's
//! equals in the same block of 16, across block boundaries and at 256.

use flashtex_engine::Globals;

/// tex.ch's loop: the newest string below `search`, above 255, equal to it.
fn reference(start: &[i32], pool: &[i32], search: usize) -> i32 {
    let len = start[search + 1] - start[search];
    if len == 0 {
        return 345; // `""`, as the translation has it
    }
    let want = &pool[start[search] as usize..start[search + 1] as usize];
    let mut s = search - 1;
    while s > 255 {
        if start[s + 1] - start[s] == len && &pool[start[s] as usize..start[s + 1] as usize] == want
        {
            return s as i32;
        }
        s -= 1;
    }
    0
}

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        // xorshift64*
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }
}

#[test]
fn search_string_matches_the_loop() {
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    for round in 0..4 {
        let mut g = Globals::new();
        let n = 256 + 700 + 300 * round;
        let (mut start, mut pool) = (vec![0i32], vec![]);
        for s in 0..n {
            // strings 0..255 one character each; then lengths 0..4 over a
            // three-letter alphabet, so equal strings and lengths abound
            let len = if s < 256 {
                1
            } else {
                (rng.next() % 5) as usize
            };
            for _ in 0..len {
                pool.push(if s < 256 {
                    s as i32
                } else {
                    b'a' as i32 + (rng.next() % 3) as i32
                });
            }
            start.push(pool.len() as i32);
        }
        for (i, &v) in start.iter().enumerate() {
            g.str_start[i] = v;
        }
        for (i, &v) in pool.iter().enumerate() {
            g.str_pool[i] = v;
        }
        g.str_ptr = n as i32;
        g.pool_ptr = pool.len() as i32;
        let mut found = 0;
        for search in 257..n {
            let want = reference(&start, &pool, search);
            assert_eq!(
                g.search_string(search as i32),
                want,
                "round {round}, string {search}"
            );
            found += (want > 255) as usize;
        }
        assert!(found > n / 2, "too few equal strings to test: {found}");
    }
}
