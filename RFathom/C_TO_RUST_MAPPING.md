# RFathom C to Rust Mapping Guide

This document shows how the original C API from Fathom maps to the idiomatic Rust API in RFathom.

## Type Mappings

| C Type | Rust Type | Notes |
|--------|-----------|-------|
| `uint64_t` (bitboard) | `Bitboard` (alias for `u64`) | Type alias for clarity |
| `unsigned` (square/result) | `u8`, `u32`, or specific types | Context-dependent |
| `bool` | `bool` | Direct mapping |
| `TbMove` (`uint16_t`) | `Move` (newtype wrapper) | Type-safe wrapper |
| `struct TbRootMove` | `RootMove` | Idiomatic Rust struct |
| `struct TbRootMoves` | `RootMoves` | Uses `Vec` instead of fixed array |

## Constant Mappings

### C Constants → Rust Constants/Enums

```rust
// C: #define TB_LOSS 0
// Rust:
WdlValue::Loss  // Enum variant

// C: #define TB_CASTLING_K 0x1
// Rust:
castling::WHITE_KING_SIDE  // Module constant
```

All WDL values are now type-safe enum variants:
- `WdlValue::Loss`
- `WdlValue::BlessedLoss`
- `WdlValue::Draw`
- `WdlValue::CursedWin`
- `WdlValue::Win`

Promotion types are also enums:
- `Promotion::None`
- `Promotion::Queen`
- `Promotion::Rook`
- `Promotion::Bishop`
- `Promotion::Knight`

## API Function Mappings

### Initialization

**C:**
```c
bool tb_init(const char *path);
void tb_free(void);
extern unsigned TB_LARGEST;
```

**Rust:**
```rust
impl Tablebase {
    pub fn new() -> Self;
    pub fn init<P: AsRef<Path>>(&self, path: P) -> Result<(), String>;
    pub fn free(&self);
    pub fn largest(&self) -> usize;
}
```

**Key differences:**
- `TB_LARGEST` is now a method on the `Tablebase` instance
- `init` returns `Result` for error handling
- `free` is automatically called when `Tablebase` is dropped (RAII)

### WDL Probing

**C:**
```c
unsigned tb_probe_wdl(
    uint64_t white,
    uint64_t black,
    uint64_t kings,
    uint64_t queens,
    uint64_t rooks,
    uint64_t bishops,
    uint64_t knights,
    uint64_t pawns,
    unsigned rule50,
    unsigned castling,
    unsigned ep,
    bool turn);
```

**Rust:**
```rust
pub fn probe_wdl(
    &self,
    white: Bitboard,
    black: Bitboard,
    kings: Bitboard,
    queens: Bitboard,
    rooks: Bitboard,
    bishops: Bitboard,
    knights: Bitboard,
    pawns: Bitboard,
    rule50: u32,
    castling_rights: u32,
    ep: Square,
    turn: Color,
) -> Option<WdlValue>
```

**Key differences:**
- Returns `Option<WdlValue>` instead of raw `unsigned`
- `None` represents failure instead of `TB_RESULT_FAILED`
- `turn` is a `Color` enum instead of `bool`
- `ep` is typed as `Square` (u8) for type safety

### Root Probing

**C:**
```c
unsigned tb_probe_root(
    uint64_t white,
    uint64_t black,
    uint64_t kings,
    uint64_t queens,
    uint64_t rooks,
    uint64_t bishops,
    uint64_t knights,
    uint64_t pawns,
    unsigned rule50,
    unsigned castling,
    unsigned ep,
    bool turn,
    unsigned *results);
```

**Rust:**
```rust
pub fn probe_root(
    &self,
    white: Bitboard,
    black: Bitboard,
    kings: Bitboard,
    queens: Bitboard,
    rooks: Bitboard,
    bishops: Bitboard,
    knights: Bitboard,
    pawns: Bitboard,
    rule50: u32,
    castling_rights: u32,
    ep: Square,
    turn: Color,
    results: Option<&mut Vec<ProbeResult>>,
) -> ProbeResult
```

**Key differences:**
- Returns `ProbeResult` type with methods instead of bit-packed integer
- `results` parameter is `Option<&mut Vec<ProbeResult>>` instead of raw pointer
- Memory-safe - no manual array size management

### Root DTZ/WDL with Move Ranking

**C:**
```c
int tb_probe_root_dtz(
    uint64_t white, uint64_t black,
    uint64_t kings, uint64_t queens,
    uint64_t rooks, uint64_t bishops,
    uint64_t knights, uint64_t pawns,
    unsigned rule50, unsigned castling,
    unsigned ep, bool turn,
    bool hasRepeated, bool useRule50,
    struct TbRootMoves *results);
```

**Rust:**
```rust
pub fn probe_root_dtz(
    &self,
    white: Bitboard,
    black: Bitboard,
    kings: Bitboard,
    queens: Bitboard,
    rooks: Bitboard,
    bishops: Bitboard,
    knights: Bitboard,
    pawns: Bitboard,
    rule50: u32,
    castling_rights: u32,
    ep: Square,
    turn: Color,
    has_repeated: bool,
    use_rule50: bool,
) -> Option<RootMoves>
```

**Key differences:**
- Returns `Option<RootMoves>` instead of mutating output parameter
- Rust naming convention: `has_repeated`, `use_rule50` (snake_case)
- Returns `None` on failure instead of returning 0

## Extracting Data from Results

### C Macros → Rust Methods

**C:**
```c
#define TB_GET_WDL(res) (((res) & TB_RESULT_WDL_MASK) >> TB_RESULT_WDL_SHIFT)
#define TB_GET_FROM(res) (((res) & TB_RESULT_FROM_MASK) >> TB_RESULT_FROM_SHIFT)
// etc.
```

**Rust:**
```rust
impl ProbeResult {
    pub fn wdl(&self) -> Option<WdlValue>;
    pub fn from_square(&self) -> Square;
    pub fn to_square(&self) -> Square;
    pub fn promotion(&self) -> Promotion;
    pub fn is_en_passant(&self) -> bool;
    pub fn dtz(&self) -> i32;
}
```

**Key differences:**
- Type-safe methods instead of bit manipulation macros
- `wdl()` returns `Option<WdlValue>` - proper enum instead of integer
- `promotion()` returns `Promotion` enum
- Clearer names: `is_en_passant()` instead of `TB_GET_EP()`

### Setting Result Fields

**C:**
```c
#define TB_SET_WDL(res, wdl) (((res) & ~TB_RESULT_WDL_MASK) | \
    (((wdl) << TB_RESULT_WDL_SHIFT) & TB_RESULT_WDL_MASK))
```

**Rust:**
```rust
impl ProbeResult {
    pub fn with_wdl(self, wdl: WdlValue) -> Self;
    pub fn with_from(self, from: Square) -> Self;
    pub fn with_to(self, to: Square) -> Self;
    // etc.
}

// Usage with builder pattern:
let result = ProbeResult::from_raw(0)
    .with_wdl(WdlValue::Win)
    .with_from(12)
    .with_to(20);
```

## Move Handling

### C Move Extraction

**C:**
```c
#define TB_MOVE_FROM(move) (((move) >> 6) & 0x3F)
#define TB_MOVE_TO(move) ((move) & 0x3F)
#define TB_MOVE_PROMOTES(move) (((move) >> 12) & 0x7)
```

**Rust:**
```rust
impl Move {
    pub fn from_square(&self) -> Square;
    pub fn to_square(&self) -> Square;
    pub fn promotion(&self) -> Promotion;
    
    // Constructor
    pub fn new(from: Square, to: Square, promotion: Promotion) -> Self;
}
```

## Helper API

### C Functions → Rust Functions

**C:**
```c
unsigned tb_pop_count(uint64_t bb);
unsigned tb_lsb(uint64_t bb);
uint64_t tb_pop_lsb(uint64_t bb);
uint64_t tb_king_attacks(unsigned square);
// etc.
```

**Rust:**
```rust
#[cfg(feature = "helper-api")]
pub mod helper {
    pub fn pop_count(bb: Bitboard) -> u32;
    pub fn lsb(bb: Bitboard) -> Square;
    pub fn pop_lsb(bb: Bitboard) -> (Bitboard, Square);  // Returns tuple!
    pub fn king_attacks(square: Square) -> Bitboard;
    // etc.
}
```

**Key differences:**
- Gated behind `helper-api` feature flag
- `pop_lsb` returns a tuple instead of modifying the input
- All functions are pure (no side effects)

## Error Handling Comparison

### C

```c
unsigned result = tb_probe_wdl(...);
if (result == TB_RESULT_FAILED) {
    // Handle error
}

int success = tb_probe_root_dtz(...);
if (success == 0) {
    // Handle error
}
```

### Rust

```rust
// Option for WDL
match tb.probe_wdl(...) {
    Some(wdl) => { /* Use wdl */ },
    None => { /* Handle error */ },
}

// ProbeResult for root
let result = tb.probe_root(...);
if result.is_failed() {
    // Handle error
}

// Option for ranked moves
match tb.probe_root_dtz(...) {
    Some(moves) => { /* Process moves */ },
    None => { /* Handle error */ },
}
```

## Usage Example Comparison

### C Code

```c
bool success = tb_init("/path/to/syzygy");
if (!success) {
    fprintf(stderr, "Failed to init\n");
    return 1;
}

unsigned result = tb_probe_wdl(white, black, kings, queens, rooks,
                               bishops, knights, pawns, 0, 0, 0, true);

if (result != TB_RESULT_FAILED) {
    unsigned wdl = TB_GET_WDL(result);
    printf("WDL: %u\n", wdl);
}

tb_free();
```

### Rust Code

```rust
let tb = Tablebase::new();
tb.init("/path/to/syzygy")?;

if let Some(wdl) = tb.probe_wdl(
    white, black, kings, queens, rooks, bishops, knights, pawns,
    0, 0, 0, Color::White
) {
    println!("WDL: {:?}", wdl);
}

// tb.free() called automatically when tb goes out of scope
```

## Memory Safety Improvements

1. **RAII**: Resources are automatically freed when `Tablebase` is dropped
2. **No null pointers**: `Option` type instead of nullable pointers
3. **Bounds checking**: Vector access is bounds-checked
4. **Type safety**: Enums prevent invalid WDL/promotion values
5. **Immutability by default**: Prevents accidental modifications

## Thread Safety

Both implementations support thread-safe WDL probing. In Rust, this is enforced at compile-time:

```rust
// Tablebase can be safely shared across threads
let tb = Arc::new(Tablebase::new());
tb.init("./syzygy")?;

// Clone the Arc and move to another thread
let tb_clone = Arc::clone(&tb);
thread::spawn(move || {
    tb_clone.probe_wdl(...);  // Safe!
});
```

Root probing functions explicitly note they are NOT thread-safe in both versions.
