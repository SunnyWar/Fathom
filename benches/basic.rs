use criterion::{criterion_group, criterion_main, Criterion};
fn bench_hello(c: &mut Criterion) {
    c.bench_function("hello", |b| b.iter(|| 42));
}

criterion_group!(benches, bench_hello);
criterion_main!(benches);
