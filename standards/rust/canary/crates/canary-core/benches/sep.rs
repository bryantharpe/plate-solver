//! Criterion micro-benchmark. Its job in the canary is to prove the perf gate
//! runs and emits a comparable number; in a real rig, `criterion` output feeds a
//! perf-regression comparison against the base branch.
//!
//! Benchmarks are not public API, so the workspace `missing_docs` deny does not
//! apply to the harness functions `criterion_group!` generates.
#![allow(missing_docs)]

use canary_core::angular_separation;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_separation(c: &mut Criterion) {
    c.bench_function("angular_separation", |b| {
        b.iter(|| angular_separation(black_box([0.2, 0.5, 0.84]), black_box([0.9, 0.1, 0.4])));
    });
}

criterion_group!(benches, bench_separation);
criterion_main!(benches);
