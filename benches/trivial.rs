use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;

fn bench_trivial(c: &mut Criterion) {
    c.bench_function("trivial", |b| b.iter(|| black_box(1 + 1)));
}

criterion_group!(benches, bench_trivial);
criterion_main!(benches);
