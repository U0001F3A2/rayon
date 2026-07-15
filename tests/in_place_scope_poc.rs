//! PoC / validation for the `in_place_scope` doc clarification (issue #1165).
//!
//! This is NOT part of the doc-fix PR — it is kept on the branch to empirically
//! back the claims the revised docs make. Run with:
//!     cargo test --test in_place_scope_poc -- --nocapture
//!
//! Claims under test (from the revised docs on `in_place_scope` /
//! `in_place_scope_fifo`):
//!   (1) The scope closure runs on the *calling* thread.
//!   (2) Tasks spawned into the scope (`scope.spawn` / `spawn_broadcast`) run in
//!       the scope's (custom) thread pool.
//!   (3) Other Rayon ops inside the closure — `join`, `spawn`, parallel
//!       iterators — do NOT inherit the scope's pool; from a non-worker caller
//!       they use the global pool, never the custom pool.
//!   (4) Contrast: plain `pool.scope` runs its closure ON a pool worker — this
//!       is exactly the difference the docs call out.

use rayon::prelude::*;
use rayon::ThreadPoolBuilder;
use std::sync::Mutex;
use std::thread;

fn tname() -> String {
    thread::current()
        .name()
        .unwrap_or("<unnamed>")
        .to_string()
}

fn is_custom(name: &str) -> bool {
    name.starts_with("custom-pool-")
}

#[test]
fn in_place_scope_pool_selection() {
    let pool = ThreadPoolBuilder::new()
        .num_threads(3)
        .thread_name(|i| format!("custom-pool-{i}"))
        .build()
        .unwrap();

    let caller_id = thread::current().id();

    let closure_on_caller = Mutex::new(false);
    let spawn_names = Mutex::new(Vec::<String>::new());
    let broadcast_names = Mutex::new(Vec::<String>::new());
    let join_names = Mutex::new(Vec::<String>::new());
    let par_names = Mutex::new(Vec::<String>::new());

    pool.in_place_scope(|s| {
        // (1) closure runs on the calling thread.
        *closure_on_caller.lock().unwrap() = thread::current().id() == caller_id;

        // (2a) scope.spawn -> scope's pool.
        s.spawn(|_| spawn_names.lock().unwrap().push(tname()));

        // (2b) scope.spawn_broadcast -> scope's pool, once per worker.
        s.spawn_broadcast(|_, _| broadcast_names.lock().unwrap().push(tname()));

        // (3a) free join -> current (global) pool, not the scope's pool.
        rayon::join(
            || join_names.lock().unwrap().push(tname()),
            || join_names.lock().unwrap().push(tname()),
        );

        // (3b) parallel iterator -> current (global) pool, not the scope's pool.
        (0..2_000).into_par_iter().for_each(|_| {
            par_names.lock().unwrap().push(tname());
        });
    });

    let spawn_names = spawn_names.into_inner().unwrap();
    let broadcast_names = broadcast_names.into_inner().unwrap();
    let join_names = join_names.into_inner().unwrap();
    let par_names = par_names.into_inner().unwrap();

    // (1)
    assert!(
        *closure_on_caller.lock().unwrap(),
        "claim 1: in_place_scope closure must run on the calling thread"
    );

    // (2)
    assert!(!spawn_names.is_empty());
    assert!(
        spawn_names.iter().all(|n| is_custom(n)),
        "claim 2: scope.spawn must run in the scope's pool, got {spawn_names:?}"
    );
    assert_eq!(
        broadcast_names.len(),
        3,
        "claim 2: spawn_broadcast must run once per worker of the scope's pool"
    );
    assert!(
        broadcast_names.iter().all(|n| is_custom(n)),
        "claim 2: spawn_broadcast must run in the scope's pool, got {broadcast_names:?}"
    );

    // (3) — the core of the reported confusion: these do NOT touch the custom pool.
    assert!(
        join_names.iter().all(|n| !is_custom(n)),
        "claim 3: join must not inherit the scope's pool, got {join_names:?}"
    );
    assert!(
        !par_names.is_empty() && par_names.iter().all(|n| !is_custom(n)),
        "claim 3: par_iter must not inherit the scope's pool, got a custom-pool name in {par_names:?}"
    );

    // (4) contrast: pool.scope DOES run its closure on a pool worker.
    let scope_closure = Mutex::new(String::new());
    pool.scope(|_| *scope_closure.lock().unwrap() = tname());
    assert!(
        is_custom(&scope_closure.lock().unwrap()),
        "claim 4: pool.scope closure should run on a pool worker (contrast with in_place_scope), got {:?}",
        scope_closure.lock().unwrap()
    );
}

#[test]
fn in_place_scope_fifo_pool_selection() {
    // Same as above, abbreviated, for the _fifo variant the docs also edit.
    let pool = ThreadPoolBuilder::new()
        .num_threads(2)
        .thread_name(|i| format!("custom-pool-{i}"))
        .build()
        .unwrap();

    let caller_id = thread::current().id();
    let closure_on_caller = Mutex::new(false);
    let spawn_names = Mutex::new(Vec::<String>::new());
    let par_names = Mutex::new(Vec::<String>::new());

    pool.in_place_scope_fifo(|s| {
        *closure_on_caller.lock().unwrap() = thread::current().id() == caller_id;
        s.spawn_fifo(|_| spawn_names.lock().unwrap().push(tname()));
        (0..2_000).into_par_iter().for_each(|_| {
            par_names.lock().unwrap().push(tname());
        });
    });

    let spawn_names = spawn_names.into_inner().unwrap();
    let par_names = par_names.into_inner().unwrap();

    assert!(*closure_on_caller.lock().unwrap());
    assert!(
        !spawn_names.is_empty() && spawn_names.iter().all(|n| is_custom(n)),
        "scope.spawn_fifo must run in the scope's pool, got {spawn_names:?}"
    );
    assert!(
        !par_names.is_empty() && par_names.iter().all(|n| !is_custom(n)),
        "par_iter must not inherit the fifo scope's pool, got {par_names:?}"
    );
}
