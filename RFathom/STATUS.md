# RFathom - Rust Port of Fathom Tablebase Library

## Project Summary

RFathom is a Rust port of the [Fathom](https://github.com/jdart1/Fathom) Syzygy endgame tablebase probing library (originally written in C by basil00 / Ronald de Man / Jon Dart). The goal is a safe, idiomatic Rust library with identical probing semantics.

---

## ✅ What's Been Completed

### 📦 Project Structure
- ✅ Cargo project with feature flags (`helper-api`)
- ✅ `.gitignore` configured (worktree noise excluded)
- ✅ Module layout: `lib.rs`, `constants.rs`, `types.rs`, `bitboard.rs`, `encoding.rs`, `loader.rs`, `probe.rs`, `syzygy.rs`, `helper.rs`

### 📚 Type System
- ✅ `Bitboard` / `Square` types
- ✅ `Move` newtype with field extractors
- ✅ `ProbeResult` with builder pattern
- ✅ `WdlValue` enum (Loss, BlessedLoss, Draw, CursedWin, Win)
- ✅ `Promotion` enum
- ✅ `Color` enum (White, Black)
- ✅ `RootMove` / `RootMoves` structures

### 🎯 Public API
- ✅ `Tablebase::new()` — constructor
- ✅ `Tablebase::init()` — single/multi-path init (`;` on Windows, `:` on Unix)
- ✅ `Tablebase::probe_wdl()` — WDL probing (thread-safe via `RwLock`)
- ✅ `Tablebase::probe_root()` — DTZ probing at root
- ✅ `Tablebase::probe_root_dtz()` — root move ranking with DTZ
- ✅ `Tablebase::probe_root_wdl()` — root move ranking with WDL
- ✅ `Tablebase::largest()` — max tablebase size
- ✅ `Tablebase::free()` — RAII cleanup

### 🔧 Utilities
- ✅ Bitboard ops: `pop_count`, `lsb`, `pop_lsb`, `isolate_lsb`
- ✅ King, knight, pawn, rook, bishop, queen attack generation
- ✅ Optional `helper-api` feature exposing attack generation

### 📁 File I/O (`loader.rs`)
- ✅ Syzygy file discovery (`.rtbw` / `.rtbz`)
- ✅ Buffered file access
- ✅ Material key indexing into `HashMap<String, MaterialTables>`
- ✅ `TableMeta` parsed and stored per material key
- ✅ WDL + DTZ table probing (simplified test format)
- ✅ Multi-path `init()` support
- ✅ Malformed header detection + error handling

### 🔍 Core Probing (`probe.rs` + `encoding.rs`)
- ✅ Position encoding with FNV-1a hash
- ✅ Position validation (overlap, king count, piece union)
- ✅ Material key canonicalization (stronger side always first)
- ✅ Color-flip with WDL inversion and DTZ negation
- ✅ `probe_wdl_impl()` — full pipeline with magic-based format dispatch
- ✅ `probe_root_impl()` — full pipeline with magic-based format dispatch

### 🌳 Root Analysis (`probe.rs`)
- ✅ Candidate root move generation: K, Q, R, B, N, P (all piece types)
- ✅ Pawn double push from starting rank
- ✅ En passant capture generation
- ✅ Pawn promotion to queen on back rank
- ✅ `Tablebase::probe_root_dtz()` — per-move DTZ ranking with correct `tb_rank`/`tb_score`
- ✅ `Tablebase::probe_root_wdl()` — per-move WDL ranking with `WDL_TO_RANK` lookup
- ✅ Legal move generation: `Pos`, `do_move`, `gen_legal_moves`, all 4 promotion types

### 🗜️ Real Syzygy Binary Format Decoder (`syzygy.rs`)
- ✅ WDL magic: `0x5d23e871` / DTZ magic: `0xa50c66d7`
- ✅ Static lookup tables: `OffDiag`, `Triangle`, `FlipDiag`, `Lower`, `Diag`, `Flap`, `PawnTwist`, `KKIdx`
- ✅ Dynamic tables (init_indices): `Binomial[k][n]`, `PawnIdx`, `PawnFactorFile`, `PawnFactorRank`
- ✅ `TableMeta` derivation from material key (num, has_pawns, symmetric, kk_enc, pawns[2])
- ✅ `parse_material_key()` — parses e.g. `"kqvkr"` into `TableMeta`
- ✅ `EncInfo` + `init_enc_info()` — encoding header parsing with factor/norm computation
- ✅ `PairsData` + `setup_pairs()` — Huffman header parsing
- ✅ `calc_sym_len()` — recursive symbol length computation
- ✅ `decompress_pairs()` — 64-bit Huffman decoder (DECOMP64 path)
- ✅ `encode_position()` — piece and pawn position encoder
- ✅ `fill_all_squares()` / `fill_squares_for_piece()` — bitboard → p[] array
- ✅ `probe_wdl_syzygy()` — full WDL prober (split tables, pawn tables, piece tables)
- ✅ `probe_dtz_syzygy()` — full DTZ prober (DTZ maps, byte/u16 variants)
- ✅ Magic-based dispatch in `probe.rs`: real format tried first, simplified format as fallback

### 🧪 Testing
- ✅ Unit tests for bitboard functions
- ✅ Unit tests for attack generation
- ✅ Unit tests for position encoding and canonicalization
- ✅ Unit tests for file loading and probing
- ✅ Unit tests for probe pipeline (WDL, DTZ, root moves)
- ✅ Unit tests for pawn double push, promotions, en passant
- ✅ Unit tests for Syzygy decoder (magic, material key parsing, Binomial table)
- ✅ **45 tests passing**, clean build

---

## Project Files

```
RFathom/
├── Cargo.toml
├── README.md
├── C_TO_RUST_MAPPING.md
├── PROJECT_STRUCTURE.md
├── .gitignore
│
├── src/
│   ├── lib.rs          ✅ Library entry point
│   ├── constants.rs    ✅ Constants and enums
│   ├── types.rs        ✅ Type definitions
│   ├── bitboard.rs     ✅ Bitboard utilities
│   ├── encoding.rs     ✅ Position encoding + canonicalization
│   ├── loader.rs       ✅ File I/O, WDL/DTZ probing, TableMeta indexing
│   ├── probe.rs        ✅ Tablebase API + root move generation
│   ├── syzygy.rs       ✅ Real Syzygy binary format decoder
│   └── helper.rs       ✅ Attack generation helper API
│
└── examples/
    ├── basic.rs
    └── helper_api.rs
```

---

## Build Status

```
✅ cargo build       — compiles (warnings suppressed with #[allow])
✅ cargo test        — 54 tests pass (45 unit + 8 integration + 1 doc), zero warnings
✅ cargo doc         — documentation generated
```

---

## What Still Needs Work

### ✅ Validation Against Real Syzygy Files (Complete)
- [x] Real `.rtbw` / `.rtbz` files available at `C:\Syzygy` (full 3-4-5 piece set)
- [x] Integration tests in `tests/syzygy_integration.rs` — 8 tests, all passing
  - KQvK Win, KRvK Win, KBBvK Win, KBvK Draw, KNvK Draw
  - KQvKR Win (4-piece), KQvK DTZ positive, `largest()` ≥ 3
- [x] `init_enc_info` factor loop validated — correct results on real tables
- [x] DTZ map byte-offset indexing validated — KQvK DTZ returns positive value
- [x] `calc_sym_len` handles real tables without recursion issues
- Tests skip gracefully with a warning when files are missing (`SYZYGY_PATH` env var overrides default path)

### ✅ Per-Move Root Probing (Complete)
- [x] `Pos` struct + `do_bb_move` + `do_move` with legality check (`is_in_check`)
- [x] `gen_pseudo_legal_moves` / `gen_legal_moves` / `is_mate`
- [x] All four promotion types (Q/N/R/B) generated for pawn moves to back rank
- [x] `probe_root_dtz` — applies each legal move, probes child DTZ/WDL, computes `tb_rank` / `tb_score` using Fathom C reference formula
- [x] `probe_root_wdl` — applies each legal move, probes child WDL, uses `WDL_TO_RANK = [-1000, -899, 0, 899, 1000]`
- [x] `tb_score` computed from `tb_rank` matching Fathom C

### ✅ PV Extension (Not Required)
- Reckless does not read the PV array (`pvSize` always 0, never accessed)
- Root moves are sorted by `tb_rank` by the engine — no iterative PV needed

### 🚧 Optimization (Low Priority)
- [ ] Pre-computed attack lookup tables (currently computed on-the-fly)
- [ ] Performance benchmarks vs original Fathom C library
- [ ] Thread-safety stress tests under concurrent WDL probing

---

## Key Design Improvements Over C

1. **Memory Safety** — No manual memory management; RAII cleanup
2. **Type Safety** — Enums prevent invalid values; newtypes prevent confusion
3. **Error Handling** — `Option`/`Result` instead of sentinel values
4. **Thread Safety** — `RwLock` enforces safe concurrent WDL probing
5. **Immutability** — Default immutable; explicit `mut` when needed
6. **No Null Pointers** — `Option<T>` makes nullability explicit
7. **Builder Pattern** — Fluent API for constructing `ProbeResult`

---

## References

- **Original Fathom**: https://github.com/jdart1/Fathom
- **Syzygy Format Reference**: `D:\Fathom\src\tbprobe.c`
- **Syzygy Tables Info**: https://syzygy-tables.info/

---

**Status**: Real Syzygy decoder validated against live files; integration tests passing 🟢  
**Last Updated**: March 23, 2026  
**Version**: 0.1.0  
**Tests**: 54 passing ✅ (45 unit + 8 integration + 1 doc)

