// Repro for issue #1142: par_bridge deadlock under work-stealing recursion.
// Amplifies the stock `par_bridge_recursion` test with heavy thread
// oversubscription and many iterations to trigger the intermittent hang.
//
//   N=<items> THREADS=<n> ITERS=<n> cargo run --example par_bridge_deadlock_repro
//
// Prints progress to stderr; if it stops advancing, it has deadlocked.
use rayon::prelude::*;
use std::iter::once_with;

fn env(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

fn main() {
    let n = env("N", 20_000);
    let threads = env("THREADS", 32);
    let iters = env("ITERS", 200);
    eprintln!("N={n} THREADS={threads} ITERS={iters}");

    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .unwrap();

    let seq: Vec<_> = (0..n).map(|i| (i, i.to_string())).collect();

    for it in 0..iters {
        eprintln!("iter {it} ...");
        pool.broadcast(|_| {
            let mut par: Vec<_> = (0..n)
                .into_par_iter()
                .flat_map(|i| {
                    once_with(move || rayon::join(move || i, move || i.to_string())).par_bridge()
                })
                .collect();
            par.par_sort_unstable();
            assert_eq!(seq, par);
        });
    }
    eprintln!("done (no deadlock)");
}
