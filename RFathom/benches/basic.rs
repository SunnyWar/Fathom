use criterion::{criterion_group, criterion_main, Criterion};
use rfathom::*;

fn bench_probe(c: &mut Criterion) {
    // Example: benchmark a function from the library
    // Replace with actual performance-critical function(s)
    c.bench_function("probe_example", |b| {
        b.iter(|| {
            // Example: call a function to benchmark
            // probe_function(args)
        })
    });
}

criterion_group!(benches, bench_probe);
criterion_main!(benches);
