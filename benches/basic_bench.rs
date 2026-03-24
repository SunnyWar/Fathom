use criterion::{criterion_group, criterion_main, Criterion};
use rfathom::syzygy::{parse_material_key, probe_dtz_syzygy, probe_wdl_syzygy};
use std::hint::black_box;

fn bench_parse_material_key(c: &mut Criterion) {
    let keys = ["kvk", "kqvk", "kqpvkr", "krpkr", "kbbkn"];
    c.bench_function("parse_material_key", |b| {
        b.iter(|| {
            for key in &keys {
                black_box(parse_material_key(key));
            }
        })
    });
}

fn bench_probe_wdl_syzygy(c: &mut Criterion) {
    // Use a mock data buffer with correct magic for WDL
    let mut data = [0u8; 128];
    data[0..4].copy_from_slice(&0x5d23e871u32.to_le_bytes());
    let meta = parse_material_key("kvk").unwrap();
    c.bench_function("probe_wdl_syzygy", |b| {
        b.iter(|| {
            let result = probe_wdl_syzygy(&data, &meta, false, true, 0, 0, 0, 0, 0, 0, 0, 0);
            black_box(result);
        })
    });
}

fn bench_probe_dtz_syzygy(c: &mut Criterion) {
    // Use a mock data buffer with correct magic for DTZ
    let mut data = [0u8; 128];
    data[0..4].copy_from_slice(&0xa50c66d7u32.to_le_bytes());
    let meta = parse_material_key("kvk").unwrap();
    c.bench_function("probe_dtz_syzygy", |b| {
        b.iter(|| {
            let result = probe_dtz_syzygy(&data, &meta, false, true, 0, 0, 0, 0, 0, 0, 0, 0, 0);
            black_box(result);
        })
    });
}

fn bench_trivial(c: &mut Criterion) {
    c.bench_function("trivial", |b| b.iter(|| 1 + 1));
}

criterion_group!(
    benches,
    bench_trivial,
    bench_parse_material_key,
    bench_probe_wdl_syzygy,
    bench_probe_dtz_syzygy
);
criterion_main!(benches);
