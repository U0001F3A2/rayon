//! PoC / validation for the "# Ordering" module docs (issue #669).
//!
//! Branch-local (NOT part of the docs PR). Backs the claims the new docs make:
//! order-dependent adaptors/consumers preserve order, while `find_*` positional
//! methods and `par_bridge` behave as documented. Run with:
//!     cargo test --test order_preservation_poc

use rayon::prelude::*;
use rayon::ThreadPoolBuilder;

const N: i32 = 200_000;

/// Force real parallelism so any order bug would actually surface.
fn in_big_pool<R: Send>(f: impl FnOnce() -> R + Send) -> R {
    ThreadPoolBuilder::new()
        .num_threads(8)
        .build()
        .unwrap()
        .install(f)
}

#[test]
fn map_filter_collect_preserves_order() {
    in_big_pool(|| {
        let par: Vec<i32> = (0..N)
            .into_par_iter()
            .map(|x| x * 2)
            .filter(|x| x % 3 != 0)
            .collect();
        let seq: Vec<i32> = (0..N).map(|x| x * 2).filter(|x| x % 3 != 0).collect();
        assert_eq!(par, seq, "map+filter+collect must match sequential order");
    });
}

#[test]
fn enumerate_preserves_indices() {
    in_big_pool(|| {
        let par: Vec<(usize, i32)> = (0..N).into_par_iter().map(|x| x * 7).enumerate().collect();
        // Every element must sit at the index a sequential iterator would give it.
        assert!(
            par.iter().enumerate().all(|(i, &(idx, val))| idx == i && val == (i as i32) * 7),
            "enumerate must yield sequential indices aligned with values"
        );
    });
}

#[test]
fn zip_aligns_by_position() {
    in_big_pool(|| {
        let a: Vec<i64> = (0..N as i64).collect();
        let b: Vec<i64> = (0..N as i64).map(|x| x * x).collect();
        let zipped: Vec<(i64, i64)> = a.par_iter().copied().zip(b.par_iter().copied()).collect();
        assert!(
            zipped.iter().all(|&(x, y)| y == x * x),
            "zip must pair elements by position, not arrival order"
        );
    });
}

#[test]
fn find_first_and_last_are_positional() {
    in_big_pool(|| {
        // Many elements satisfy the predicate; first/last must be positional.
        let v: Vec<i32> = (0..N).collect();
        let first = v.par_iter().find_first(|&&x| x % 100 == 0);
        let last = v.par_iter().find_last(|&&x| x % 100 == 0);
        assert_eq!(first, Some(&0), "find_first must return the first match");
        assert_eq!(
            last,
            Some(&(((N - 1) / 100) * 100)),
            "find_last must return the last match"
        );

        // find_any just returns *some* match (order not guaranteed). It must at
        // least return a valid one.
        let any = v.par_iter().find_any(|&&x| x % 100 == 0);
        assert!(any.map_or(false, |&x| x % 100 == 0), "find_any must return a valid match");
    });
}

#[test]
fn par_bridge_collects_all_items_even_if_reordered() {
    in_big_pool(|| {
        // par_bridge does NOT guarantee order; but it must still yield every item.
        let mut got: Vec<i32> = (0..50_000).par_bridge().map(|x| x * 3).collect();
        got.sort_unstable();
        let want: Vec<i32> = (0..50_000).map(|x| x * 3).collect();
        assert_eq!(got, want, "par_bridge must produce all items (order aside)");
    });
}
