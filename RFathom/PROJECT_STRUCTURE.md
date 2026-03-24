# RFathom Project Structure

This document provides an overview of the RFathom project structure and what each file contains.

## Project Root

```
RFathom/
├── Cargo.toml              # Project manifest and dependencies
├── README.md               # Main documentation
├── C_TO_RUST_MAPPING.md   # Mapping guide from C API to Rust
├── .gitignore              # Git ignore patterns
├── src/                    # Source code
│   ├── lib.rs             # Library entry point and public API
│   ├── constants.rs       # Constants and enums
│   ├── types.rs           # Type definitions
│   ├── bitboard.rs        # Bitboard utilities
│   ├── probe.rs           # Tablebase probing implementation
│   └── helper.rs          # Optional helper API
└── examples/               # Example programs
    ├── basic.rs           # Basic usage example
    └── helper_api.rs      # Helper API demonstration
```

## Module Overview

### `src/lib.rs`
- Main library entry point
- Re-exports public types and functions
- Conditional compilation for the `helper-api` feature
- Top-level documentation

### `src/constants.rs`
- Constant values (MAX_MOVES, MAX_PLY, etc.)
- Castling rights constants
- WDL value enum (`WdlValue`)
- Promotion piece enum (`Promotion`)
- Internal bit masks and shifts for result encoding

### `src/types.rs`
- `Bitboard` type alias
- `Square` type alias
- `Move` struct with constructor and field extractors
- `ProbeResult` struct for tablebase probe results
- `RootMove` and `RootMoves` for root analysis
- `Color` enum for side to move

### `src/bitboard.rs`
- Low-level bitboard manipulation functions
- `pop_count()` - count set bits
- `lsb()` - find least significant bit
- `pop_lsb()` - pop and return LSB
- Unit tests

### `src/probe.rs`
- `Tablebase` struct - main API
- `init()` - initialize tablebase from path
- `probe_wdl()` - Win-Draw-Loss probing
- `probe_root()` - DTZ probing at root
- `probe_root_dtz()` - Root move ranking with DTZ
- `probe_root_wdl()` - Root move ranking with WDL
- Thread safety notes in documentation

### `src/helper.rs` (optional, feature-gated)
- Attack generation functions
- Piece movement utilities
- Re-exports of bitboard functions
- Only compiled when `helper-api` feature is enabled

## Examples

### `examples/basic.rs`
Demonstrates:
- Creating and initializing a `Tablebase`
- Probing WDL values
- Root probing for move suggestions
- DTZ-based root move analysis

### `examples/helper_api.rs`
Demonstrates:
- Bitboard manipulation functions
- Attack generation for different piece types
- Pretty-printing bitboards
- Requires `--features helper-api` to compile

## Feature Flags

### `default`
Includes: `helper-api`

### `helper-api`
Enables the helper API module with attack generation and bitboard utilities.
Can be disabled for chess engines that have their own implementations.

## Building

```bash
# Standard build with all features
cargo build

# Build without helper API
cargo build --no-default-features

# Release build
cargo build --release

# Run tests
cargo test

# Run examples
cargo run --example basic
cargo run --example helper_api --features helper-api
```

## API Design Principles

1. **Type Safety**: Use enums and newtypes instead of raw integers
2. **Memory Safety**: No unsafe code in public API, proper RAII
3. **Rust Idioms**: Use `Option` and `Result` for error handling
4. **Zero Cost**: Newtype wrappers compile to same representation
5. **Clear Intent**: Descriptive names and comprehensive documentation

## Implementation Status

### ✅ Completed
- Type system design
- Public API definition
- Bitboard utilities
- Project structure
- Documentation
- Build system
- Unit tests

### 🚧 To Be Implemented
- Tablebase file loading and parsing
- Actual WDL probing logic
- Actual DTZ probing logic
- Root move ranking algorithms
- Attack generation (helper API)
- Performance benchmarks

## Testing Strategy

- Unit tests for each module
- Doc tests for public API examples
- Integration tests for complete workflows (when implementation is done)
Benchmark suite for performance-critical paths (benches/ directory, Criterion.rs)
Performance benchmarks (benches/ directory, Criterion.rs)

## Contributing

3. Update documentation
4. Ensure `cargo test` passes
5. Run `cargo clippy` for linting
6. Format code with `cargo fmt`

## Next Steps

To complete the implementation, the following needs to be done:

1. **File I/O**: Implement tablebase file loading
   - Parse .rtbw (WDL) files
   - Parse .rtbz (DTZ) files
   - Memory-mapped file access

2. **Probe Implementation**: Port the actual probing logic
   - WDL lookup algorithms
   - DTZ lookup algorithms
   - Move generation
   - Position encoding/decoding

3. **Helper API**: Implement attack generation
   - Pre-computed lookup tables
   - Magic bitboard implementation
   - Or use existing Rust chess library

4. **Performance**: Optimize hot paths
   - Benchmark critical functions
   - Consider SIMD where applicable
   - Profile and optimize

5. **Integration**: Test with real chess engines
   - Verify correctness against original Fathom
   - Performance comparison
   - Real-world usage patterns

## References

- [Original Fathom](https://github.com/jdart1/Fathom)
- [Syzygy Format Documentation](https://syzygy-tables.info/)
- [Chess Programming Wiki - Endgame Tablebases](https://www.chessprogramming.org/Endgame_Tablebases)
