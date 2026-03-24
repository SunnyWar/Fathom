// benches/bitboard_simd.rs
// Benchmark to compare current bitboard functions vs. SIMD batch operations (if available)

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rfathom::bitboard::{isolate_lsb, lsb, pop_count, pop_lsb};

fn bench_pop_count(c: &mut Criterion) {
    let bbs = [0u64, 1, 0xFF, 0xFFFF_FFFF_FFFF_FFFF, 0x8000_0000_0000_0001];
    c.bench_function("pop_count", |b| {
        b.iter(|| {
            for &bb in &bbs {
                black_box(pop_count(bb));
            }
        })
    });
}

fn bench_lsb(c: &mut Criterion) {
    let bbs = [1, 2, 0x100, 0x8000_0000_0000_0000];
    c.bench_function("lsb", |b| {
        b.iter(|| {
            for &bb in &bbs {
                black_box(lsb(bb));
            }
        })
    });
}

fn bench_pop_lsb(c: &mut Criterion) {
    let bbs = [0b1010, 0x8000_0000_0000_0001];
    c.bench_function("pop_lsb", |b| {
        b.iter(|| {
            for &bb in &bbs {
                black_box(pop_lsb(bb));
            }
        })
    });
}

fn bench_isolate_lsb(c: &mut Criterion) {
    let bbs = [0b1010, 0x8000_0000_0000_0001];
    c.bench_function("isolate_lsb", |b| {
        b.iter(|| {
            for &bb in &bbs {
                black_box(isolate_lsb(bb));
            }
        })
    });
}

criterion_group!(
    benches,
    bench_pop_count,
    bench_lsb,
    bench_pop_lsb,
    bench_isolate_lsb
);
criterion_main!(benches);
