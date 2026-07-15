// Throughput check for NORMAL (non-recursive) par_bridge, to measure any
// parallelism regression from the #1142 fix. next() here does NOT re-enter rayon.
use rayon::prelude::*;
use std::time::Instant;

fn work(x: u64) -> u64 {
    // a little CPU per item so parallel consumption matters
    let mut h = x;
    for _ in 0..200 {
        h = h
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
    }
    h % 1000
}

fn main() {
    let n: u64 = std::env::var("N")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(2_000_000);
    // warm up pool
    let _ = (0..1000u64).into_par_iter().sum::<u64>();

    let mut best = f64::MAX;
    for _ in 0..3 {
        let t = Instant::now();
        let s: u64 = (0..n).par_bridge().map(work).sum();
        let el = t.elapsed().as_secs_f64();
        best = best.min(el);
        std::hint::black_box(s);
    }
    println!(
        "par_bridge N={n} threads={} best={best:.3}s",
        rayon::current_num_threads()
    );
}
