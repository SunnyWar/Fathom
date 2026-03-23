# RFathom

A Rust implementation of Syzygy tablebase probing, ported from the [Fathom](https://github.com/jdart1/Fathom) C library.

## Overview

RFathom provides access to Syzygy endgame tablebases for chess engines and tools. It supports:

- **Win-Draw-Loss (WDL)** probing - Determine the game-theoretic outcome
- **Distance-To-Zero (DTZ)** probing - Find the distance to a zeroing move (capture or pawn move)
- **Root move analysis** - Rank and score all legal moves with principal variations
- **Thread-safe WDL probing** - Safe for use during parallel search
- **Optional helper API** - Bitboard manipulation and attack generation

## Features

- Pure Rust implementation (no C dependencies)
- Idiomatic Rust API with strong typing
- Memory-safe by design
- Optional helper functions via feature flag
- Comprehensive type safety for chess positions

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
rfathom = "0.1"
```

To disable the helper API (recommended for engines):

```toml
[dependencies]
rfathom = { version = "0.1", default-features = false }
```

## Usage

### Basic Example

```rust
use rfathom::{Tablebase, Color};

fn main() -> Result<(), String> {
    // Create and initialize tablebase
    let tb = Tablebase::new();
    tb.init("path/to/syzygy/tables")?;

    println!("Largest tablebase: {} pieces", tb.largest());

    // Probe a position (bitboards for white, black, and piece types)
    let wdl = tb.probe_wdl(
        0x0000_0000_0000_0010,  // white pieces
        0x0000_0000_0000_1000,  // black pieces
        0x0000_0000_0000_1010,  // kings
        0,                       // queens
        0,                       // rooks
        0,                       // bishops
        0,                       // knights
        0,                       // pawns
        0,                       // rule50 counter
        0,                       // castling rights
        0,                       // en passant square
        Color::White,           // side to move
    );

    if let Some(wdl_value) = wdl {
        println!("WDL result: {:?}", wdl_value);
    }

    Ok(())
}
```

### Root Probing with DTZ

```rust
use rfathom::{Tablebase, Color};

fn probe_root_position(tb: &Tablebase) -> Option<()> {
    let root_moves = tb.probe_root_dtz(
        white, black, kings, queens, rooks, bishops, knights, pawns,
        0,              // rule50
        0,              // castling
        0,              // ep
        Color::White,
        false,          // has_repeated
        true,           // use_rule50
    )?;

    // Iterate through ranked moves
    for root_move in root_moves.iter() {
        println!("Move: {:?}, Score: {}, Rank: {}",
                 root_move.mv, root_move.tb_score, root_move.tb_rank);
        
        // Print principal variation
        for &pv_move in &root_move.pv {
            print!("{:?} ", pv_move);
        }
        println!();
    }

    Some(())
}
```

### Using the Helper API

```rust
use rfathom::helper::*;
use rfathom::Color;

fn example_attacks() {
    let square = 27; // e4
    
    let king_atks = king_attacks(square);
    let knight_atks = knight_attacks(square);
    let pawn_atks = pawn_attacks(square, Color::White);
    
    println!("King attacks: {:064b}", king_atks);
    println!("Knight attacks: {:064b}", knight_atks);
    println!("Pawn attacks: {:064b}", pawn_atks);
}
```

## Types

### Core Types

- `Bitboard` - `u64` representing piece positions on the board
- `Square` - `u8` representing a square (0-63, a1=0, h8=63)
- `Color` - White or Black
- `Move` - Encoded chess move (from, to, promotion)
- `ProbeResult` - Result from tablebase probe with embedded move and DTZ
- `WdlValue` - Win/Draw/Loss values (Loss, BlessedLoss, Draw, CursedWin, Win)
- `Promotion` - Promotion piece type

### Result Types

- `RootMove` - Move with tablebase evaluation and PV
- `RootMoves` - Collection of evaluated root moves

## Constants

```rust
use rfathom::castling::*;

let rights = WHITE_KING_SIDE | WHITE_QUEEN_SIDE;
```

Available castling constants:
- `WHITE_KING_SIDE` (0x1)
- `WHITE_QUEEN_SIDE` (0x2)
- `BLACK_KING_SIDE` (0x4)
- `BLACK_QUEEN_SIDE` (0x8)

## Thread Safety

- `probe_wdl()` - Thread-safe, can be called during parallel search
- `probe_root()`, `probe_root_dtz()`, `probe_root_wdl()` - NOT thread-safe, should only be called at the root

## Performance Notes

- The implementation uses atomic operations for thread-safe access
- WDL probing is optimized for use during search
- DTZ probing generates complete move information and should be used sparingly

## Development Status

This is an initial port of the Fathom library to Rust. The core API is defined and functional,
but the actual tablebase file parsing and probing logic is still being implemented.

Current status:
- ✅ Type definitions and constants
- ✅ Public API design
- ✅ Bitboard utilities
- 🚧 Tablebase file loading
- 🚧 WDL probing implementation
- 🚧 DTZ probing implementation
- 🚧 Root move analysis
- 🚧 Helper API (attack generation)

## License

MIT License - Same as the original Fathom library.

## Credits

- Original Fathom library by basil
- Rust port by RFathom contributors

## Links

- [Original Fathom](https://github.com/jdart1/Fathom)
- [Syzygy Tablebases](https://syzygy-tables.info/)
