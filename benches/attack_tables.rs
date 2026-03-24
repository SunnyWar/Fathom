use criterion::{criterion_group, criterion_main, Criterion};
use rfathom::helper::{king_attacks, knight_attacks, pawn_attacks};
use rfathom::types::Color;
use std::hint::black_box;

fn bench_king_attacks(c: &mut Criterion) {
    c.bench_function("king_attacks_all_squares", |b| {
        b.iter(|| {
            let mut sum = 0u64;
            for sq in 0u8..64 {
                sum ^= black_box(king_attacks(sq));
            }
            black_box(sum)
        })
    });
}

fn bench_knight_attacks(c: &mut Criterion) {
    c.bench_function("knight_attacks_all_squares", |b| {
        b.iter(|| {
            let mut sum = 0u64;
            for sq in 0u8..64 {
                sum ^= black_box(knight_attacks(sq));
            }
            black_box(sum)
        })
    });
}

fn bench_pawn_attacks_white(c: &mut Criterion) {
    c.bench_function("pawn_attacks_white_all_squares", |b| {
        b.iter(|| {
            let mut sum = 0u64;
            for sq in 0u8..64 {
                sum ^= black_box(pawn_attacks(sq, Color::White));
            }
            black_box(sum)
        })
    });
}

fn bench_pawn_attacks_black(c: &mut Criterion) {
    c.bench_function("pawn_attacks_black_all_squares", |b| {
        b.iter(|| {
            let mut sum = 0u64;
            for sq in 0u8..64 {
                sum ^= black_box(pawn_attacks(sq, Color::Black));
            }
            black_box(sum)
        })
    });
}

criterion_group!(
    benches,
    bench_king_attacks,
    bench_knight_attacks,
    bench_pawn_attacks_white,
    bench_pawn_attacks_black,
);
criterion_main!(benches);
