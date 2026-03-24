//! Basic usage example for RFathom

use rfathom::{Color, Tablebase, WdlValue};

fn main() {
    // Create a new tablebase instance
    let tb = Tablebase::new();

    // Initialize with path to Syzygy tables
    // Replace with actual path on your system
    match tb.init("./syzygy") {
        Ok(()) => {
            println!("Tablebase initialized successfully");
            println!("Largest tablebase: {} pieces", tb.largest());
        }
        Err(e) => {
            eprintln!("Failed to initialize tablebase: {}", e);
            return;
        }
    }

    // Example position: KPk (white king on e4, white pawn on e5, black king on e7)
    // This is a simplified example - in real usage, you'd have complete bitboards

    let white = 0x0000_0010_1000_0000; // e4 and e5
    let black = 0x0010_0000_0000_0000; // e7
    let kings = 0x0010_0000_1000_0000; // e4 and e7
    let queens = 0;
    let rooks = 0;
    let bishops = 0;
    let knights = 0;
    let pawns = 0x0000_0010_0000_0000; // e5

    // Probe WDL for the position
    match tb.probe_wdl(
        white,
        black,
        kings,
        queens,
        rooks,
        bishops,
        knights,
        pawns,
        0, // rule50 counter
        0, // no castling rights
        0, // no en passant
        Color::White,
    ) {
        Some(wdl) => {
            println!("\nWDL probe result: {:?}", wdl);
            match wdl {
                WdlValue::Win => println!("Position is winning for white"),
                WdlValue::CursedWin => {
                    println!("Position is winning but can be drawn by the 50-move rule")
                }
                WdlValue::Draw => println!("Position is drawn"),
                WdlValue::BlessedLoss => {
                    println!("Position is losing but can be drawn by the 50-move rule")
                }
                WdlValue::Loss => println!("Position is losing for white"),
            }
        }
        None => {
            println!("WDL probe failed");
        }
    }

    // Probe root to get detailed move information
    let result = tb.probe_root(
        white,
        black,
        kings,
        queens,
        rooks,
        bishops,
        knights,
        pawns,
        0,
        0,
        0,
        Color::White,
        None, // Don't need all moves
    );

    if !result.is_failed() {
        println!("\nRoot probe succeeded");
        if let Some(wdl) = result.wdl() {
            println!("WDL: {:?}", wdl);
        }
        println!(
            "Suggested move: {}{}",
            square_to_string(result.from_square()),
            square_to_string(result.to_square())
        );
        println!("DTZ: {}", result.dtz());
    } else {
        println!("Root probe failed");
    }

    // Demonstrate probe_root_dtz for complete move analysis
    if let Some(root_moves) = tb.probe_root_dtz(
        white,
        black,
        kings,
        queens,
        rooks,
        bishops,
        knights,
        pawns,
        0,
        0,
        0,
        Color::White,
        false,
        true,
    ) {
        println!("\nRoot DTZ analysis:");
        println!("Found {} legal moves", root_moves.len());

        for (i, root_move) in root_moves.iter().enumerate() {
            println!(
                "  {}. Move: {}{} Score: {} Rank: {}",
                i + 1,
                square_to_string(root_move.mv.from_square()),
                square_to_string(root_move.mv.to_square()),
                root_move.tb_score,
                root_move.tb_rank
            );

            if !root_move.pv.is_empty() {
                print!("     PV: ");
                for pv_move in &root_move.pv {
                    print!(
                        "{}{} ",
                        square_to_string(pv_move.from_square()),
                        square_to_string(pv_move.to_square())
                    );
                }
                println!();
            }
        }
    }
}

/// Convert square index to algebraic notation
fn square_to_string(square: u8) -> String {
    let file = square % 8;
    let rank = square / 8;
    format!("{}{}", (b'a' + file) as char, (b'1' + rank) as char)
}
