use criterion::{criterion_group, criterion_main, Criterion};
use rfathom::*;

const DEFAULT_PATH: &str = r"C:\Syzygy";

fn try_load() -> Option<Tablebase> {
    let path = std::env::var("SYZYGY_PATH").unwrap_or_else(|_| DEFAULT_PATH.to_string());
    if !std::path::Path::new(&path).is_dir() {
        eprintln!(
            "\nWARNING: Syzygy directory not found at '{}' — skipping benchmark\n(set SYZYGY_PATH env var to override)",
            path
        );
        return None;
    }
    let tb = Tablebase::new();
    match tb.init(&path) {
        Err(e) => {
            eprintln!("\nWARNING: tb.init({path}) failed: {e} — skipping\n");
            None
        }
        Ok(_) if tb.largest() == 0 => {
            eprintln!("\nWARNING: No Syzygy files were loaded from '{path}' — skipping\n");
            None
        }
        Ok(_) => Some(tb),
    }
}

const fn sq(file: u8, rank: u8) -> u64 {
    1u64 << (rank * 8 + file)
}

fn bench_kbnvk(c: &mut Criterion) {
    let Some(tb) = try_load() else { return };
    // KBNvK: 8/8/8/8/8/3k4/8/2KBN3 w - - 0 1
    let wk = sq(2, 0); // c1
    let wb = sq(5, 0); // f1
    let wn = sq(6, 0); // g1
    let bk = sq(3, 2); // d3
    let white = wk | wb | wn;
    let black = bk;
    let kings = wk | bk;
    c.bench_function("probe_wdl_KBNvK", |b| {
        b.iter(|| tb.probe_wdl(white, black, kings, 0, 0, wb, wn, 0, 0, 0, 0, Color::White))
    });
}

fn bench_kqvkr(c: &mut Criterion) {
    let Some(tb) = try_load() else { return };
    // KQvKR: 8/8/8/8/k7/2Q5/8/K2R4 w - - 0 1
    let wk = sq(0, 0); // a1
    let wq = sq(2, 2); // c3
    let wr = sq(3, 1); // d2
    let bk = sq(0, 3); // a4
    let white = wk | wq | wr;
    let black = bk;
    let kings = wk | bk;
    c.bench_function("probe_wdl_KQvKR", |b| {
        b.iter(|| tb.probe_wdl(white, black, kings, wq, wr, 0, 0, 0, 0, 0, 0, Color::White))
    });
}

fn bench_fen_c7_promotion(c: &mut Criterion) {
    let Some(tb) = try_load() else { return };
    // 8/2P5/3K4/8/8/8/1k6/2R5 w - - 0 1
    let wk = sq(3, 5); // d6
    let wr = sq(2, 0); // c1
    let wp = sq(2, 6); // c7
    let bk = sq(1, 1); // b2
    let white = wk | wr | wp;
    let black = bk;
    let kings = wk | bk;
    c.bench_function("probe_wdl_c7_promotion", |b| {
        b.iter(|| tb.probe_wdl(white, black, kings, 0, wr, 0, 0, wp, 0, 0, 0, Color::White))
    });
}

criterion_group!(benches, bench_kbnvk, bench_kqvkr, bench_fen_c7_promotion);
criterion_main!(benches);
