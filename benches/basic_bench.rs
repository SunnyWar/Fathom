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
    use rfathom::{Color, Tablebase};
    // Use the same setup as integration tests
    let path = std::env::var("SYZYGY_PATH").unwrap_or_else(|_| r"C:\Syzygy".to_string());
    let tb = Tablebase::new();
    if tb.init(&path).is_err() || tb.largest() == 0 {
        eprintln!("Syzygy tables not found at {path}, skipping benchmark");
        return;
    }

    // Example: KBNvK position (same as integration test)
    let wk = 1u64 << (0 * 8 + 2); // c1
    let wb = 1u64 << (0 * 8 + 5); // f1
    let wn = 1u64 << (0 * 8 + 6); // g1
    let bk = 1u64 << (2 * 8 + 3); // d3
    let white = wk | wb | wn;
    let black = bk;
    let kings = wk | bk;

    c.bench_function("probe_wdl_syzygy", |b| {
        b.iter(|| {
            let result = tb.probe_wdl(white, black, kings, 0, 0, wb, wn, 0, 0, 0, 0, Color::White);
            black_box(result);
        })
    });
}

fn bench_probe_dtz_syzygy(c: &mut Criterion) {
    use rfathom::{Color, Tablebase};
    let path = std::env::var("SYZYGY_PATH").unwrap_or_else(|_| r"C:\Syzygy".to_string());
    let tb = Tablebase::new();
    if tb.init(&path).is_err() || tb.largest() == 0 {
        eprintln!("Syzygy tables not found at {path}, skipping benchmark");
        return;
    }

    // Example: KBNvK position (same as integration test)
    let wk = 1u64 << (0 * 8 + 2); // c1
    let wb = 1u64 << (0 * 8 + 5); // f1
    let wn = 1u64 << (0 * 8 + 6); // g1
    let bk = 1u64 << (2 * 8 + 3); // d3
    let white = wk | wb | wn;
    let black = bk;
    let kings = wk | bk;

    c.bench_function("probe_dtz_syzygy", |b| {
        b.iter(|| {
            // DTZ probing is usually done via probe_root or probe_root_dtz
            let result = tb.probe_root_dtz(
                white,
                black,
                kings,
                0,
                0,
                wb,
                wn,
                0,
                0,
                0,
                0,
                Color::White,
                false,
                false,
            );
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
