//! qsort.c (Nelson Beebe's quicksort with an insertion-sort finish), on an
//! array of element handles. The comparison has side effects (it counts,
//! prints progress dots and marks duplicates), so the order and the
//! arguments of every call are kept exactly; pointers become indices and
//! byte counts element counts.

const THRESH: isize = 4; // threshold for insertion
const MTHRESH: isize = 6; // threshold for median

/// `qqsort(base, n, sizeof(FIELD_PTR), compar)`.
pub fn qqsort<T: Copy>(a: &mut [T], cmp: &mut dyn FnMut(T, T) -> i32) {
    let n = a.len();
    if n <= 1 {
        return;
    }
    let max = n;
    let mut hi;
    if n as isize >= THRESH {
        qst(a, 0, max as isize, cmp);
        hi = THRESH as usize;
    } else {
        hi = max;
    }
    // First put smallest element, which must be in the first THRESH, in
    // the first position as a sentinel.
    let mut j = 0;
    let mut lo = 0;
    loop {
        lo += 1;
        if lo >= hi {
            break;
        }
        if cmp(a[j], a[lo]) > 0 {
            j = lo;
        }
    }
    if j != 0 {
        a.swap(0, j);
    }
    // With our sentinel in place, the insertion sort.
    let mut min = 0;
    loop {
        min += 1;
        hi = min;
        if hi >= max {
            break;
        }
        loop {
            // The sentinel stops this at index 0 for a consistent
            // comparison; C would read before the array otherwise.
            if hi == 0 {
                break;
            }
            hi -= 1;
            if cmp(a[hi], a[min]) <= 0 {
                break;
            }
        }
        hi += 1;
        if hi != min {
            a[hi..=min].rotate_right(1);
        }
    }
}

/// `qst(base, max)`: elements `base..max`. Signed indices, as C's
/// pointers may step below `mid` before the comparisons stop them.
fn qst<T: Copy>(a: &mut [T], mut base: isize, mut max: isize, cmp: &mut dyn FnMut(T, T) -> i32) {
    let at = |a: &[T], k: isize| a[k as usize];
    let mut lo = max - base; // number of elements
    loop {
        // Find the median of the first, last, and middle element and make
        // that the middle element.
        let mut mid = base + ((lo as usize) >> 1) as isize;
        let mut i = mid;
        let mut j;
        let mut jj;
        if lo >= MTHRESH {
            jj = base;
            j = if cmp(at(a, jj), at(a, i)) > 0 { jj } else { i };
            let tmp = max - 1;
            if cmp(at(a, j), at(a, tmp)) > 0 {
                // switch to first loser
                j = if j == jj { i } else { jj };
                if cmp(at(a, j), at(a, tmp)) < 0 {
                    j = tmp;
                }
            }
            if j != i {
                a.swap(i as usize, j as usize);
            }
        }
        // Semi-standard quicksort partitioning/swapping
        i = base;
        j = max - 1;
        loop {
            while i < mid && cmp(at(a, i), at(a, mid)) <= 0 {
                i += 1;
            }
            let tmp;
            let mut to_swap = false;
            while j > mid {
                if cmp(at(a, mid), at(a, j)) <= 0 {
                    j -= 1;
                    continue;
                }
                to_swap = true;
                break;
            }
            if to_swap {
                tmp = i + 1; // value of i after swap
                if i == mid {
                    // j <-> mid, new mid is j
                    mid = j;
                    jj = j;
                } else {
                    // i <-> j
                    jj = j;
                    j -= 1;
                }
            } else if i == mid {
                break;
            } else {
                // i <-> mid, new mid is i
                jj = mid;
                mid = i;
                tmp = mid; // value of i after swap
                j -= 1;
            }
            a.swap(i as usize, jj as usize);
            i = tmp;
        }
        // Do the smaller partition by recursion and the larger by
        // iteration, while a partition has at least THRESH elements.
        j = mid;
        i = mid + 1;
        lo = j - base;
        let hi = max - i;
        if lo <= hi {
            if lo >= THRESH {
                qst(a, base, j, cmp);
            }
            base = i;
            lo = hi;
        } else {
            if hi >= THRESH {
                qst(a, i, max, cmp);
            }
            max = j;
        }
        if lo < THRESH {
            break;
        }
    }
}
