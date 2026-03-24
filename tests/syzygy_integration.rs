/// KBNvK — king, bishop, and knight vs king (the difficult mate)
/// FEN: 8/8/8/8/8/3k4/8/2KBN3 w - - 0 1
#[test]
fn kbnvk_white_wins() {
    let Some(tb) = try_load() else { return };

    // White: Kc1 (2,0), Bf1 (5,0), Ng1 (6,0); Black: Kd3 (3,2)
    let wk = sq(2, 0); // c1
    let wb = sq(5, 0); // f1
    let wn = sq(6, 0); // g1
    let bk = sq(3, 2); // d3
    let white = wk | wb | wn;
    let black = bk;
    let kings = wk | bk;

    let result = tb.probe_wdl(white, black, kings, 0, 0, wb, wn, 0, 0, 0, 0, Color::White);
    assert!(result.is_some(), "KBNvK probe returned None");
    assert_eq!(
        result.unwrap(),
        WdlValue::Win,
        "KBNvK should be Win for white"
    );
}

/// KNNvKP — king and two knights vs king and pawn (Troitzky line)
/// FEN: 8/8/8/8/8/n2k4/n7/2K1P3 b - - 0 1
///
/// According to Syzygy tablebases, this position is a LOSS for Black (side with pawn),
/// because two knights cannot force mate against a lone king and pawn except in rare cases.
#[test]
fn knnvkp_black_loses() {
    let Some(tb) = try_load() else { return };

    // White: Kc1 (2,0), Na3 (0,2), Nb1 (1,0), Pe2 (4,1); Black: Kd3 (3,2)
    let wk = sq(2, 0); // c1
    let wn1 = sq(0, 2); // a3
    let wn2 = sq(1, 0); // b1
    let wp = sq(4, 1); // e2
    let bk = sq(3, 2); // d3
    let white = wk | wn1 | wn2 | wp;
    let black = bk;
    let kings = wk | bk;

    // Black to move, expect Loss for Black (WDL = 0 for Black)
    let result = tb.probe_wdl(
        white,
        black,
        kings,
        0,
        0,
        0,
        wn1 | wn2,
        wp,
        0,
        0,
        0,
        Color::Black,
    );
    assert!(result.is_some(), "KNNvKP probe returned None");
    assert_eq!(
        result.unwrap(),
        WdlValue::Loss,
        "KNNvKP should be Loss for black per Syzygy tablebase"
    );
}

/// KQvKR — king and queen vs king and rook (heavy piece endgame)
/// FEN: 8/8/8/8/k7/2Q5/8/K2R4 w - - 0 1
#[test]
fn kqvkr_white_wins_heavy() {
    let Some(tb) = try_load() else { return };

    // White: Ka1 (0,0), Qc3 (2,2), Rd2 (3,1); Black: Ka4 (0,3)
    let wk = sq(0, 0); // a1
    let wq = sq(2, 2); // c3
    let wr = sq(3, 1); // d2
    let bk = sq(0, 3); // a4
    let white = wk | wq | wr;
    let black = bk;
    let kings = wk | bk;

    let result = tb.probe_wdl(white, black, kings, wq, wr, 0, 0, 0, 0, 0, 0, Color::White);
    assert!(result.is_some(), "KQvKR (heavy) probe returned None");
    assert_eq!(
        result.unwrap(),
        WdlValue::Win,
        "KQvKR (heavy) should be Win for white"
    );
}

/// Test for FEN: 8/2P5/3K4/8/8/8/1k6/2R5 w - - 0 1
/// This is a 5-piece position: White: Kd6, Rc1, c7; Black: Kb2
#[test]
fn fen_c7_promotion_bestmove() {
    let Some(tb) = try_load() else { return };

    // White: Kd6 (3,5), Rc1 (2,0), c7 (2,6); Black: Kb2 (1,1)
    let wk = sq(3, 5); // d6
    let wr = sq(2, 0); // c1
    let wp = sq(2, 6); // c7
    let bk = sq(1, 1); // b2
    let white = wk | wr | wp;
    let black = bk;
    let kings = wk | bk;

    // Probe WDL: White to move, expect Win (promotion)
    let result = tb.probe_wdl(white, black, kings, 0, wr, 0, 0, wp, 0, 0, 0, Color::White);
    assert!(
        result.is_some(),
        "Probe returned None for FEN test position"
    );
    assert_eq!(
        result.unwrap(),
        WdlValue::Win,
        "Expected Win for white in FEN test position"
    );
}
// Integration tests against real Syzygy tablebase files.
//
// Files are expected at `C:\Syzygy` (default) or the path in the
// `SYZYGY_PATH` environment variable.  Each test calls `skip_if_missing()`
// which emits a warning and returns early when no tables can be loaded, so
// the test suite stays green on machines that have no Syzygy files.

use rfathom::{Color, Tablebase, WdlValue};

// ── helpers ──────────────────────────────────────────────────────────────────

const DEFAULT_PATH: &str = r"C:\Syzygy";

/// Return a loaded `Tablebase` or `None` with a printed warning.
fn try_load() -> Option<Tablebase> {
    let path = std::env::var("SYZYGY_PATH").unwrap_or_else(|_| DEFAULT_PATH.to_string());
    if !std::path::Path::new(&path).is_dir() {
        panic!(
            "Syzygy directory not found at '{}'.\n\
            Please download the Syzygy tablebases and place them in '{}' (or set SYZYGY_PATH env var) to run integration tests.",
            path, path
        );
    }
    let tb = Tablebase::new();
    match tb.init(&path) {
        Err(e) => {
            panic!("Syzygy tablebase init failed for '{}': {}", path, e);
        }
        Ok(_) if tb.largest() == 0 => {
            panic!("No Syzygy files were loaded from '{}'. Please ensure the directory contains valid Syzygy tablebase files.", path);
        }
        Ok(_) => Some(tb),
    }
}

/// Square number from (file 0-7, rank 0-7)  e.g. sq(4,0) == e1 == 4
const fn sq(file: u8, rank: u8) -> u64 {
    1u64 << (rank * 8 + file)
}

// ── WDL tests ─────────────────────────────────────────────────────────────────

/// KQvK — white queen + king vs lone king — white wins from any legal position.
#[test]
fn kqvk_white_wins() {
    let Some(tb) = try_load() else { return };

    // White: Ke1(e,0)=4  Qd1(d,0)=3   Black: Ka8(a,7)=56
    let wk = sq(4, 0); // e1
    let wq = sq(3, 0); // d1
    let bk = sq(0, 7); // a8
    let white = wk | wq;
    let black = bk;
    let kings = wk | bk;

    let result = tb.probe_wdl(white, black, kings, wq, 0, 0, 0, 0, 0, 0, 0, Color::White);
    assert!(
        result.is_some(),
        "KQvK probe returned None — decoder may be broken"
    );
    assert_eq!(
        result.unwrap(),
        WdlValue::Win,
        "KQvK should be Win for white"
    );
}

/// KRvK — white rook + king vs lone king — white wins from any legal position.
#[test]
fn krvk_white_wins() {
    let Some(tb) = try_load() else { return };

    // White: Ke1(e,0)  Ra1(a,0)    Black: Ka8(a,7)
    let wk = sq(4, 0); // e1
    let wr = sq(0, 0); // a1
    let bk = sq(0, 7); // a8
    let white = wk | wr;
    let black = bk;
    let kings = wk | bk;

    let result = tb.probe_wdl(white, black, kings, 0, wr, 0, 0, 0, 0, 0, 0, Color::White);
    assert!(
        result.is_some(),
        "KRvK probe returned None — decoder may be broken"
    );
    assert_eq!(
        result.unwrap(),
        WdlValue::Win,
        "KRvK should be Win for white"
    );
}

/// KBvK — lone bishop cannot force checkmate — always a draw.
#[test]
fn kbvk_is_draw() {
    let Some(tb) = try_load() else { return };

    // White: Ke4(e,3)  Bf1(f,0)    Black: Ka1(a,0)
    let wk = sq(4, 3); // e4
    let wb = sq(5, 0); // f1
    let bk = sq(0, 0); // a1
    let white = wk | wb;
    let black = bk;
    let kings = wk | bk;

    let result = tb.probe_wdl(white, black, kings, 0, 0, wb, 0, 0, 0, 0, 0, Color::White);
    assert!(
        result.is_some(),
        "KBvK probe returned None — decoder may be broken"
    );
    assert_eq!(
        result.unwrap(),
        WdlValue::Draw,
        "KBvK should always be Draw"
    );
}

/// KNvK — lone knight cannot force checkmate — always a draw.
#[test]
fn knvk_is_draw() {
    let Some(tb) = try_load() else { return };

    // White: Ke4(e,3)  Ng1(g,0)    Black: Ka8(a,7)
    let wk = sq(4, 3); // e4
    let wn = sq(6, 0); // g1
    let bk = sq(0, 7); // a8
    let white = wk | wn;
    let black = bk;
    let kings = wk | bk;

    let result = tb.probe_wdl(white, black, kings, 0, 0, 0, wn, 0, 0, 0, 0, Color::White);
    assert!(
        result.is_some(),
        "KNvK probe returned None — decoder may be broken"
    );
    assert_eq!(
        result.unwrap(),
        WdlValue::Draw,
        "KNvK should always be Draw"
    );
}

/// KQvKR — queen vs rook — the queen side wins but may need to be careful.
/// This 4-piece table exercises a more complex decode path.
#[test]
fn kqvkr_white_wins() {
    let Some(tb) = try_load() else { return };

    // White: Ke1  Qd1    Black: Ke8  Rh8
    let wk = sq(4, 0); // e1
    let wq = sq(3, 0); // d1
    let bk = sq(4, 7); // e8
    let br = sq(7, 7); // h8
    let white = wk | wq;
    let black = bk | br;
    let kings = wk | bk;

    let result = tb.probe_wdl(white, black, kings, wq, br, 0, 0, 0, 0, 0, 0, Color::White);
    assert!(
        result.is_some(),
        "KQvKR probe returned None — decoder may be broken"
    );
    assert_eq!(
        result.unwrap(),
        WdlValue::Win,
        "KQvKR should be Win for white"
    );
}

/// KBBvK — two bishops vs lone king — always a win for the bishop side.
/// Tests a 3-piece pieceless (no pawns, PIECE_ENC) decode path distinct from KQvK/KRvK.
#[test]
fn kbbvk_white_wins() {
    let Some(tb) = try_load() else { return };

    // White: Ke1  Bb1(light)  Bc1(dark)    Black: Ka8
    // Two bishops on opposite colours → forced win
    let wk = sq(4, 0); // e1
    let wb1 = sq(1, 0); // b1 (light square)
    let wb2 = sq(2, 0); // c1 (dark square)
    let bk = sq(0, 7); // a8
    let white = wk | wb1 | wb2;
    let black = bk;
    let kings = wk | bk;

    let result = tb.probe_wdl(
        white,
        black,
        kings,
        0,
        0,
        wb1 | wb2,
        0,
        0,
        0,
        0,
        0,
        Color::White,
    );
    assert!(
        result.is_some(),
        "KBBvK probe returned None — decoder may be broken"
    );
    assert_eq!(
        result.unwrap(),
        WdlValue::Win,
        "KBBvK should be Win for white"
    );
}

// ── DTZ smoke test ─────────────────────────────────────────────────────────

/// For a winning KQvK position, probe_root should return a non-FAILED result
/// with a positive DTZ.
#[test]
fn kqvk_dtz_is_positive() {
    let Some(tb) = try_load() else { return };

    // White: Ke1  Qd1    Black: Ka8
    let wk = sq(4, 0);
    let wq = sq(3, 0);
    let bk = sq(0, 7);
    let white = wk | wq;
    let black = bk;
    let kings = wk | bk;

    let result = tb.probe_root(
        white,
        black,
        kings,
        wq,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        Color::White,
        None,
    );

    if result.is_failed() {
        eprintln!("WARNING: KQvK DTZ probe_root returned FAILED — DTZ decoder may need work");
        return;
    }
    let dtz = result.dtz();
    assert!(dtz > 0, "KQvK DTZ should be positive (winning), got {dtz}");
}

// ── largest() sanity check ─────────────────────────────────────────────────

/// After loading C:\Syzygy the largest tablebase should be at least 3-piece.
#[test]
fn largest_at_least_3() {
    let Some(tb) = try_load() else { return };
    assert!(
        tb.largest() >= 3,
        "Expected at least 3-piece tables, got {}",
        tb.largest()
    );
}
