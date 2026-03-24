//! Real Syzygy binary format decoder.
//!
//! Implements the Huffman-compressed lookup-table format used by .rtbw (WDL)
//! and .rtbz (DTZ) files produced by Syzygy tablebase generators.
//!
//! Reference: tbprobe.c by Ronald de Man / basil00 / Jon Dart.

use std::sync::OnceLock;

use crate::WdlValue;

// ── Magic bytes ──────────────────────────────────────────────────────────────

pub(crate) const WDL_MAGIC: u32 = 0x5d23e871;
pub(crate) const DTZ_MAGIC: u32 = 0xa50c66d7;

// ── Piece-type constants (matching tbchess.c) ────────────────────────────────
// W_PAWN=1..W_KING=6, B_PAWN=9..B_KING=14
const W_PAWN: u8 = 1;
#[allow(dead_code)]
const W_KNIGHT: u8 = 2;
#[allow(dead_code)]
const W_BISHOP: u8 = 3;
#[allow(dead_code)]
const W_ROOK: u8 = 4;
#[allow(dead_code)]
const W_QUEEN: u8 = 5;
#[allow(dead_code)]
const W_KING: u8 = 6;
const B_PAWN: u8 = 9;
#[allow(dead_code)]
const B_KNIGHT: u8 = 10;
#[allow(dead_code)]
const B_BISHOP: u8 = 11;
#[allow(dead_code)]
const B_ROOK: u8 = 12;
#[allow(dead_code)]
const B_QUEEN: u8 = 13;
#[allow(dead_code)]
const B_KING: u8 = 14;

const TB_PIECES: usize = 7;

// ── Encoding type ─────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Debug)]
enum Enc {
    Piece,
    File, // pawn tables (WDL / DTZ)
}

// ── Static lookup tables (direct translation from tbprobe.c) ─────────────────

#[rustfmt::skip]
static OFF_DIAG: [i8; 64] = [
    0,-1,-1,-1,-1,-1,-1,-1,
    1, 0,-1,-1,-1,-1,-1,-1,
    1, 1, 0,-1,-1,-1,-1,-1,
    1, 1, 1, 0,-1,-1,-1,-1,
    1, 1, 1, 1, 0,-1,-1,-1,
    1, 1, 1, 1, 1, 0,-1,-1,
    1, 1, 1, 1, 1, 1, 0,-1,
    1, 1, 1, 1, 1, 1, 1, 0,
];

#[rustfmt::skip]
static TRIANGLE: [u8; 64] = [
    6, 0, 1, 2, 2, 1, 0, 6,
    0, 7, 3, 4, 4, 3, 7, 0,
    1, 3, 8, 5, 5, 8, 3, 1,
    2, 4, 5, 9, 9, 5, 4, 2,
    2, 4, 5, 9, 9, 5, 4, 2,
    1, 3, 8, 5, 5, 8, 3, 1,
    0, 7, 3, 4, 4, 3, 7, 0,
    6, 0, 1, 2, 2, 1, 0, 6,
];

#[rustfmt::skip]
static FLIP_DIAG: [u8; 64] = [
     0,  8, 16, 24, 32, 40, 48, 56,
     1,  9, 17, 25, 33, 41, 49, 57,
     2, 10, 18, 26, 34, 42, 50, 58,
     3, 11, 19, 27, 35, 43, 51, 59,
     4, 12, 20, 28, 36, 44, 52, 60,
     5, 13, 21, 29, 37, 45, 53, 61,
     6, 14, 22, 30, 38, 46, 54, 62,
     7, 15, 23, 31, 39, 47, 55, 63,
];

#[rustfmt::skip]
static LOWER: [u8; 64] = [
    28,  0,  1,  2,  3,  4,  5,  6,
     0, 29,  7,  8,  9, 10, 11, 12,
     1,  7, 30, 13, 14, 15, 16, 17,
     2,  8, 13, 31, 18, 19, 20, 21,
     3,  9, 14, 18, 32, 22, 23, 24,
     4, 10, 15, 19, 22, 33, 25, 26,
     5, 11, 16, 20, 23, 25, 34, 27,
     6, 12, 17, 21, 24, 26, 27, 35,
];

#[rustfmt::skip]
static DIAG: [u8; 64] = [
     0,  0,  0,  0,  0,  0,  0,  8,
     0,  1,  0,  0,  0,  0,  9,  0,
     0,  0,  2,  0,  0, 10,  0,  0,
     0,  0,  0,  3, 11,  0,  0,  0,
     0,  0,  0, 12,  4,  0,  0,  0,
     0,  0, 13,  0,  0,  5,  0,  0,
     0, 14,  0,  0,  0,  0,  6,  0,
    15,  0,  0,  0,  0,  0,  0,  7,
];

#[rustfmt::skip]
static FLAP: [[u8; 64]; 2] = [
  [
     0,  0,  0,  0,  0,  0,  0,  0,
     0,  6, 12, 18, 18, 12,  6,  0,
     1,  7, 13, 19, 19, 13,  7,  1,
     2,  8, 14, 20, 20, 14,  8,  2,
     3,  9, 15, 21, 21, 15,  9,  3,
     4, 10, 16, 22, 22, 16, 10,  4,
     5, 11, 17, 23, 23, 17, 11,  5,
     0,  0,  0,  0,  0,  0,  0,  0,
  ],
  [
     0,  0,  0,  0,  0,  0,  0,  0,
     0,  1,  2,  3,  3,  2,  1,  0,
     4,  5,  6,  7,  7,  6,  5,  4,
     8,  9, 10, 11, 11, 10,  9,  8,
    12, 13, 14, 15, 15, 14, 13, 12,
    16, 17, 18, 19, 19, 18, 17, 16,
    20, 21, 22, 23, 23, 22, 21, 20,
     0,  0,  0,  0,  0,  0,  0,  0,
  ],
];

#[rustfmt::skip]
static PAWN_TWIST: [[u8; 64]; 2] = [
  [
     0,  0,  0,  0,  0,  0,  0,  0,
    47, 35, 23, 11, 10, 22, 34, 46,
    45, 33, 21,  9,  8, 20, 32, 44,
    43, 31, 19,  7,  6, 18, 30, 42,
    41, 29, 17,  5,  4, 16, 28, 40,
    39, 27, 15,  3,  2, 14, 26, 38,
    37, 25, 13,  1,  0, 12, 24, 36,
     0,  0,  0,  0,  0,  0,  0,  0,
  ],
  [
     0,  0,  0,  0,  0,  0,  0,  0,
    47, 45, 43, 41, 40, 42, 44, 46,
    39, 37, 35, 33, 32, 34, 36, 38,
    31, 29, 27, 25, 24, 26, 28, 30,
    23, 21, 19, 17, 16, 18, 20, 22,
    15, 13, 11,  9,  8, 10, 12, 14,
     7,  5,  3,  1,  0,  2,  4,  6,
     0,  0,  0,  0,  0,  0,  0,  0,
  ],
];

#[rustfmt::skip]
static KK_IDX: [[i16; 64]; 10] = [
  [-1,-1,-1, 0, 1, 2, 3, 4,
   -1,-1,-1, 5, 6, 7, 8, 9,
   10,11,12,13,14,15,16,17,
   18,19,20,21,22,23,24,25,
   26,27,28,29,30,31,32,33,
   34,35,36,37,38,39,40,41,
   42,43,44,45,46,47,48,49,
   50,51,52,53,54,55,56,57],
  [58,-1,-1,-1,59,60,61,62,
   63,-1,-1,-1,64,65,66,67,
   68,69,70,71,72,73,74,75,
   76,77,78,79,80,81,82,83,
   84,85,86,87,88,89,90,91,
   92,93,94,95,96,97,98,99,
  100,101,102,103,104,105,106,107,
  108,109,110,111,112,113,114,115],
  [116,117,-1,-1,-1,118,119,120,
   121,122,-1,-1,-1,123,124,125,
   126,127,128,129,130,131,132,133,
   134,135,136,137,138,139,140,141,
   142,143,144,145,146,147,148,149,
   150,151,152,153,154,155,156,157,
   158,159,160,161,162,163,164,165,
   166,167,168,169,170,171,172,173],
  [174,-1,-1,-1,175,176,177,178,
   179,-1,-1,-1,180,181,182,183,
   184,-1,-1,-1,185,186,187,188,
   189,190,191,192,193,194,195,196,
   197,198,199,200,201,202,203,204,
   205,206,207,208,209,210,211,212,
   213,214,215,216,217,218,219,220,
   221,222,223,224,225,226,227,228],
  [229,230,-1,-1,-1,231,232,233,
   234,235,-1,-1,-1,236,237,238,
   239,240,-1,-1,-1,241,242,243,
   244,245,246,247,248,249,250,251,
   252,253,254,255,256,257,258,259,
   260,261,262,263,264,265,266,267,
   268,269,270,271,272,273,274,275,
   276,277,278,279,280,281,282,283],
  [284,285,286,287,288,289,290,291,
   292,293,-1,-1,-1,294,295,296,
   297,298,-1,-1,-1,299,300,301,
   302,303,-1,-1,-1,304,305,306,
   307,308,309,310,311,312,313,314,
   315,316,317,318,319,320,321,322,
   323,324,325,326,327,328,329,330,
   331,332,333,334,335,336,337,338],
  [-1,-1,339,340,341,342,343,344,
   -1,-1,345,346,347,348,349,350,
   -1,-1,441,351,352,353,354,355,
   -1,-1,-1,442,356,357,358,359,
   -1,-1,-1,-1,443,360,361,362,
   -1,-1,-1,-1,-1,444,363,364,
   -1,-1,-1,-1,-1,-1,445,365,
   -1,-1,-1,-1,-1,-1,-1,446],
  [-1,-1,-1,366,367,368,369,370,
   -1,-1,-1,371,372,373,374,375,
   -1,-1,-1,376,377,378,379,380,
   -1,-1,-1,447,381,382,383,384,
   -1,-1,-1,-1,448,385,386,387,
   -1,-1,-1,-1,-1,449,388,389,
   -1,-1,-1,-1,-1,-1,450,390,
   -1,-1,-1,-1,-1,-1,-1,451],
  [452,391,392,393,394,395,396,397,
   -1,-1,-1,-1,398,399,400,401,
   -1,-1,-1,-1,402,403,404,405,
   -1,-1,-1,-1,406,407,408,409,
   -1,-1,-1,-1,453,410,411,412,
   -1,-1,-1,-1,-1,454,413,414,
   -1,-1,-1,-1,-1,-1,455,415,
   -1,-1,-1,-1,-1,-1,-1,456],
  [457,416,417,418,419,420,421,422,
   -1,458,423,424,425,426,427,428,
   -1,-1,-1,-1,-1,429,430,431,
   -1,-1,-1,-1,-1,432,433,434,
   -1,-1,-1,-1,-1,435,436,437,
   -1,-1,-1,-1,-1,459,438,439,
   -1,-1,-1,-1,-1,-1,460,440,
   -1,-1,-1,-1,-1,-1,-1,461],
];

static FILE_TO_FILE: [usize; 8] = [0, 1, 2, 3, 3, 2, 1, 0];

// WdlToMap and PAFlags for DTZ value post-processing
static WDL_TO_MAP: [usize; 5] = [1, 3, 0, 2, 0];
static PA_FLAGS: [u8; 5] = [8, 0, 0, 0, 4];

// ── Dynamically-computed tables (init_indices equivalent) ────────────────────

struct GlobalTables {
    binomial: [[usize; 64]; 7],
    pawn_idx: [[[usize; 24]; 6]; 2],
    pawn_factor_file: [[usize; 4]; 6],
    pawn_factor_rank: [[usize; 6]; 6],
}

static GLOBAL_TABLES: OnceLock<GlobalTables> = OnceLock::new();

fn tables() -> &'static GlobalTables {
    GLOBAL_TABLES.get_or_init(|| {
        let mut t = GlobalTables {
            binomial: [[0; 64]; 7],
            pawn_idx: [[[0; 24]; 6]; 2],
            pawn_factor_file: [[0; 4]; 6],
            pawn_factor_rank: [[0; 6]; 6],
        };

        // Binomial[k][n] = C(n, k)
        for i in 0..7usize {
            for j in 0..64usize {
                let (mut f, mut l) = (1usize, 1usize);
                for k in 0..i {
                    f = f.saturating_mul(j.saturating_sub(k));
                    l *= k + 1;
                }
                t.binomial[i][j] = if l == 0 { 0 } else { f / l };
            }
        }

        // PawnIdx[0] – file encoding
        for i in 0..6usize {
            let mut s = 0usize;
            for j in 0..24usize {
                t.pawn_idx[0][i][j] = s;
                let sq = (1 + (j % 6)) * 8 + (j / 6);
                s = s.saturating_add(t.binomial[i][PAWN_TWIST[0][sq] as usize]);
                if (j + 1) % 6 == 0 {
                    t.pawn_factor_file[i][j / 6] = s;
                    s = 0;
                }
            }
        }

        // PawnIdx[1] – rank encoding
        for i in 0..6usize {
            let mut s = 0usize;
            for j in 0..24usize {
                t.pawn_idx[1][i][j] = s;
                let sq = (1 + (j / 4)) * 8 + (j % 4);
                s = s.saturating_add(t.binomial[i][PAWN_TWIST[1][sq] as usize]);
                if (j + 1) % 4 == 0 {
                    t.pawn_factor_rank[i][j / 4] = s;
                    s = 0;
                }
            }
        }

        t
    })
}

fn subfactor(k: usize, n: usize) -> usize {
    let (mut f, mut l) = (n, 1usize);
    for i in 1..k {
        f = f.saturating_mul(n.saturating_sub(i));
        l *= i + 1;
    }
    if l == 0 {
        0
    } else {
        f / l
    }
}

// ── TableMeta ─────────────────────────────────────────────────────────────────

/// Structural metadata derived from the material key (e.g. "kqvk").
#[derive(Debug, Clone)]
pub(crate) struct TableMeta {
    pub(crate) num: usize,
    pub(crate) has_pawns: bool,
    pub(crate) symmetric: bool,
    pub(crate) kk_enc: bool,
    pub(crate) pawns: [usize; 2],
}

/// Parse a lower-case material key (e.g. "kqvkr") into [`TableMeta`].
pub(crate) fn parse_material_key(key: &str) -> Option<TableMeta> {
    // Count pieces on each side and by type
    let mut pcs = [0u8; 16]; // indexed by piece code (same as tbchess.c)
    let mut color_offset = 0u8; // 0 = white side, 8 = black side
    for ch in key.chars() {
        if ch == 'v' {
            color_offset = 8;
            continue;
        }
        let pt: u8 = match ch {
            'p' => 1,
            'n' => 2,
            'b' => 3,
            'r' => 4,
            'q' => 5,
            'k' => 6,
            _ => return None,
        };
        let idx = (pt | color_offset) as usize;
        if idx >= 16 {
            return None;
        }
        pcs[idx] += 1;
    }

    let num: usize = pcs.iter().map(|&x| x as usize).sum();
    if num < 2 || num > TB_PIECES {
        return None;
    }

    let has_pawns = pcs[W_PAWN as usize] > 0 || pcs[B_PAWN as usize] > 0;

    // symmetric: same pieces on both sides
    // Compute a simple symmetry check: white-count array == black-count array
    let symmetric = {
        let mut sym = true;
        for pt in 1usize..=6 {
            if pcs[pt] != pcs[pt + 8] {
                sym = false;
                break;
            }
        }
        sym
    };

    let kk_enc;
    let mut pawns = [0usize; 2];

    if !has_pawns {
        // kk_enc: true only when all piece types appear exactly once (only KvK)
        let unique = pcs.iter().filter(|&&x| x == 1).count();
        kk_enc = unique == 2;
    } else {
        kk_enc = false;
        pawns[0] = pcs[W_PAWN as usize] as usize;
        pawns[1] = pcs[B_PAWN as usize] as usize;
        // If black has more pawns (or white has none), swap so pawns[0] >= pawns[1]
        if pawns[1] > 0 && (pawns[0] == 0 || pawns[0] > pawns[1]) {
            pawns.swap(0, 1);
        }
    }

    Some(TableMeta {
        num,
        has_pawns,
        symmetric,
        kk_enc,
        pawns,
    })
}

// ── EncInfo ───────────────────────────────────────────────────────────────────

#[derive(Clone, Default, Debug)]
struct EncInfo {
    pieces: [u8; TB_PIECES],
    norm: [usize; TB_PIECES],
    factor: [usize; TB_PIECES],
}

// ── PairsData ─────────────────────────────────────────────────────────────────

#[derive(Debug)]
struct PairsData {
    is_const: bool,
    const_value: [u8; 2],

    block_size: usize,
    idx_bits: usize,
    min_len: usize,

    sym_len: Vec<u8>,
    base: Vec<u64>,

    // Byte offsets into the mapped file data slice
    sym_pat_off: usize,
    offset_arr_off: usize, // raw u16 array at data[10]; length = h = max_len-min_len+1
    index_table_off: usize,
    size_table_off: usize,
    data_off: usize,

    // Sizes (for post-parse offset assignment)
    idx_table_size: usize,  // bytes
    size_table_size: usize, // bytes
    data_size: usize,       // bytes
}

impl Default for PairsData {
    fn default() -> Self {
        PairsData {
            is_const: false,
            const_value: [0; 2],
            block_size: 0,
            idx_bits: 0,
            min_len: 0,
            sym_len: Vec::new(),
            base: Vec::new(),
            sym_pat_off: 0,
            offset_arr_off: 0,
            index_table_off: 0,
            size_table_off: 0,
            data_off: 0,
            idx_table_size: 0,
            size_table_size: 0,
            data_size: 0,
        }
    }
}

/// Parse a PairsData header from `data[off..]`.
/// Returns (parsed PairsData, new offset after the sym_pat section, flags byte).
fn setup_pairs(
    data: &[u8],
    off: usize,
    tb_size: usize,
    is_wdl: bool,
) -> Result<(PairsData, usize, u8), &'static str> {
    if off >= data.len() {
        return Err("setup_pairs: offset out of range");
    }
    let flags = data[off];

    if flags & 0x80 != 0 {
        // Constant block
        let const_val = if is_wdl { data[off + 1] } else { 0 };
        let pd = PairsData {
            is_const: true,
            const_value: [const_val, 0],
            ..Default::default()
        };
        return Ok((pd, off + 2, flags));
    }

    let block_size = data[off + 1] as usize;
    let idx_bits = data[off + 2] as usize;
    let real_num_blocks =
        u32::from_le_bytes([data[off + 4], data[off + 5], data[off + 6], data[off + 7]]) as usize;
    let num_blocks = real_num_blocks + data[off + 3] as usize;
    let max_len = data[off + 8] as usize;
    let min_len = data[off + 9] as usize;
    let h = max_len - min_len + 1;
    let offset_arr_off = off + 10; // u16 array, h entries

    if off + 10 + 2 * h + 2 > data.len() {
        return Err("setup_pairs: file truncated at offset array");
    }
    let num_syms =
        u16::from_le_bytes([data[off + 10 + 2 * h], data[off + 10 + 2 * h + 1]]) as usize;

    let sym_pat_off = off + 12 + 2 * h;
    // Each symbol is 3 bytes; alignment pad if num_syms is odd
    let next_off = sym_pat_off + 3 * num_syms + (num_syms & 1);

    // Compute sym_len via recursive calc_sym_len
    let mut sym_len = vec![0u8; num_syms];
    let mut visited = vec![false; num_syms];
    for s in 0..num_syms {
        if !visited[s] {
            calc_sym_len(data, sym_pat_off, s, &mut sym_len, &mut visited);
        }
    }

    // Compute base array (Huffman decode table, 64-bit version)
    let mut base = vec![0u64; h];
    base[h - 1] = 0;
    for i in (0..h - 1).rev() {
        let cnt_i = u16::from_le_bytes([
            data[offset_arr_off + 2 * i],
            data[offset_arr_off + 2 * i + 1],
        ]) as u64;
        let cnt_ip1 = u16::from_le_bytes([
            data[offset_arr_off + 2 * (i + 1)],
            data[offset_arr_off + 2 * (i + 1) + 1],
        ]) as u64;
        base[i] = (base[i + 1] + cnt_i - cnt_ip1) / 2;
    }
    // Left-align for 64-bit comparison
    for (i, b) in base.iter_mut().enumerate() {
        *b <<= 64u32.saturating_sub((min_len + i) as u32);
    }

    // Compute sizes for later pointer assignment
    let num_indices = (tb_size + (1 << idx_bits) - 1) >> idx_bits;
    let idx_table_size = 6 * num_indices;
    let size_table_size = 2 * num_blocks;
    let data_size = real_num_blocks << block_size;

    let pd = PairsData {
        is_const: false,
        const_value: [0; 2],
        block_size,
        idx_bits,
        min_len,
        sym_len,
        base,
        sym_pat_off,
        offset_arr_off,
        index_table_off: 0, // filled in later
        size_table_off: 0,  // filled in later
        data_off: 0,        // filled in later
        idx_table_size,
        size_table_size,
        data_size,
    };

    Ok((pd, next_off, flags))
}

fn calc_sym_len(
    data: &[u8],
    sym_pat_off: usize,
    s: usize,
    sym_len: &mut Vec<u8>,
    visited: &mut Vec<bool>,
) {
    visited[s] = true;
    let w = sym_pat_off + 3 * s;
    if w + 2 >= data.len() {
        sym_len[s] = 0;
        return;
    }
    let s2 = ((data[w + 2] as usize) << 4) | ((data[w + 1] >> 4) as usize);
    if s2 == 0x0fff {
        sym_len[s] = 0;
    } else {
        let s1 = (((data[w + 1] & 0x0f) as usize) << 8) | (data[w] as usize);
        if s1 < visited.len() && !visited[s1] {
            calc_sym_len(data, sym_pat_off, s1, sym_len, visited);
        }
        if s2 < visited.len() && !visited[s2] {
            calc_sym_len(data, sym_pat_off, s2, sym_len, visited);
        }
        let l1 = if s1 < sym_len.len() { sym_len[s1] } else { 0 };
        let l2 = if s2 < sym_len.len() { sym_len[s2] } else { 0 };
        sym_len[s] = l1.saturating_add(l2).saturating_add(1);
    }
}

// ── Huffman decoder ───────────────────────────────────────────────────────────

fn decompress_pairs(data: &[u8], pd: &PairsData, idx: usize) -> [u8; 2] {
    if pd.is_const {
        return pd.const_value;
    }
    if pd.idx_bits == 0 {
        return pd.const_value;
    }

    let idx_bits = pd.idx_bits;
    let main_idx = idx >> idx_bits;
    let lit_offset = (idx & ((1 << idx_bits) - 1)) as i64 - (1i64 << (idx_bits - 1));

    // Read 6-byte index-table entry
    let ie = pd.index_table_off + 6 * main_idx;
    if ie + 6 > data.len() {
        return [0; 2];
    }
    let mut block =
        u32::from_le_bytes([data[ie], data[ie + 1], data[ie + 2], data[ie + 3]]) as usize;
    let idx_off = u16::from_le_bytes([data[ie + 4], data[ie + 5]]) as i64;
    let mut lit_idx = lit_offset + idx_off;

    // Navigate to the right block
    if lit_idx < 0 {
        while lit_idx < 0 {
            if block == 0 {
                return [0; 2];
            }
            block -= 1;
            let se = pd.size_table_off + 2 * block;
            if se + 2 > data.len() {
                return [0; 2];
            }
            let sz = u16::from_le_bytes([data[se], data[se + 1]]) as i64;
            lit_idx += sz + 1;
        }
    } else {
        loop {
            let se = pd.size_table_off + 2 * block;
            if se + 2 > data.len() {
                return [0; 2];
            }
            let sz = u16::from_le_bytes([data[se], data[se + 1]]) as i64;
            if lit_idx <= sz {
                break;
            }
            lit_idx -= sz + 1;
            block += 1;
        }
    }

    // Load 8 bytes big-endian to start Huffman decode
    let blk_ptr = pd.data_off + (block << pd.block_size);
    if blk_ptr + 8 > data.len() {
        return [0; 2];
    }
    let mut code = u64::from_be_bytes([
        data[blk_ptr],
        data[blk_ptr + 1],
        data[blk_ptr + 2],
        data[blk_ptr + 3],
        data[blk_ptr + 4],
        data[blk_ptr + 5],
        data[blk_ptr + 6],
        data[blk_ptr + 7],
    ]);
    let mut ptr = blk_ptr + 8;
    let mut bit_cnt = 0u32;
    let m = pd.min_len;
    let sym;

    // Huffman search (64-bit version, matching DECOMP64 path)
    loop {
        let mut l = m;
        while l < pd.base.len() + m && code < pd.base[l - m] {
            l += 1;
        }
        if l - m >= pd.base.len() {
            return [0; 2];
        }

        // sym_start = offset array entry at code-length l (adjusted by -min_len in C)
        let arr_idx = (l - m) * 2;
        if pd.offset_arr_off + arr_idx + 2 > data.len() {
            return [0; 2];
        }
        let sym_start = u16::from_le_bytes([
            data[pd.offset_arr_off + arr_idx],
            data[pd.offset_arr_off + arr_idx + 1],
        ]) as u64;
        let s = (sym_start + ((code.wrapping_sub(pd.base[l - m])) >> (64 - l as u32))) as usize;

        if lit_idx < pd.sym_len.get(s).copied().unwrap_or(0) as i64 + 1 {
            sym = s;
            break;
        }
        lit_idx -= pd.sym_len.get(s).copied().unwrap_or(0) as i64 + 1;
        code = code.wrapping_shl(l as u32);
        bit_cnt += l as u32;
        if bit_cnt >= 32 {
            bit_cnt -= 32;
            if ptr + 4 <= data.len() {
                let tmp =
                    u32::from_be_bytes([data[ptr], data[ptr + 1], data[ptr + 2], data[ptr + 3]]);
                ptr += 4;
                code |= (tmp as u64) << bit_cnt;
            }
        }
    }

    // Walk symbol tree to leaf
    let mut cur = sym;
    while pd.sym_len.get(cur).copied().unwrap_or(0) != 0 {
        let w = pd.sym_pat_off + 3 * cur;
        if w + 2 >= data.len() {
            break;
        }
        let s1 = (((data[w + 1] & 0x0f) as usize) << 8) | (data[w] as usize);
        let left_len = pd.sym_len.get(s1).copied().unwrap_or(0) as i64;
        if lit_idx < left_len + 1 {
            cur = s1;
        } else {
            lit_idx -= left_len + 1;
            cur = ((data[w + 2] as usize) << 4) | ((data[w + 1] >> 4) as usize);
        }
    }

    let leaf = pd.sym_pat_off + 3 * cur;
    if leaf + 2 <= data.len() {
        [data[leaf], data[leaf + 1]]
    } else {
        [0; 2]
    }
}

// ── init_enc_info ──────────────────────────────────────────────────────────────

/// Parse the per-table encoding header into an [`EncInfo`] and return
/// the total table size (number of position entries).
fn init_enc_info(
    meta: &TableMeta,
    tb: &[u8],  // slice starting at the encoding-header bytes for this table
    shift: u32, // 0 for main (white-side), 4 for split (black-side)
    t: usize,   // sub-table index (file index for pawn tables)
    enc: Enc,
) -> Option<(EncInfo, usize)> {
    let tabs = tables();
    let more_pawns = enc != Enc::Piece && meta.pawns[1] > 0;

    let mut ei = EncInfo::default();

    for i in 0..meta.num {
        let byte_idx = i + 1 + (more_pawns as usize);
        if byte_idx >= tb.len() {
            return None;
        }
        ei.pieces[i] = (tb[byte_idx] >> shift) & 0x0f;
    }

    let order = (tb[0] >> shift) & 0x0f;
    let order2 = if more_pawns {
        (tb[1] >> shift) & 0x0f
    } else {
        0x0f
    };

    let mut k;
    ei.norm[0] = if enc != Enc::Piece {
        meta.pawns[0]
    } else if meta.kk_enc {
        2
    } else {
        3
    };
    k = ei.norm[0];

    if more_pawns {
        ei.norm[k] = meta.pawns[1];
        k += meta.pawns[1];
    }

    // Compute norms for remaining groups (pieces of the same type)
    let mut i = k;
    while i < meta.num {
        let mut j = i;
        while j < meta.num && ei.pieces[j] == ei.pieces[i] {
            j += 1;
        }
        ei.norm[i] = j - i;
        i = j;
    }

    let mut n = 64usize.saturating_sub(k);
    let mut f = 1usize;
    let mut kk = k;

    let mut step = 0usize;
    loop {
        let done = kk >= meta.num;
        if done && step != order as usize && step != order2 as usize {
            break;
        }

        if step == order as usize {
            ei.factor[0] = f;
            let mult = match enc {
                Enc::File => {
                    if t < 4 {
                        tabs.pawn_factor_file[ei.norm[0] - 1][t]
                    } else {
                        1
                    }
                }
                Enc::Piece => {
                    if meta.kk_enc {
                        462
                    } else {
                        31332
                    }
                }
            };
            f = f.saturating_mul(mult);
        } else if step == order2 as usize {
            ei.factor[ei.norm[0]] = f;
            f = f.saturating_mul(subfactor(
                ei.norm[ei.norm[0]],
                48usize.saturating_sub(ei.norm[0]),
            ));
        } else {
            ei.factor[kk] = f;
            f = f.saturating_mul(subfactor(ei.norm[kk], n));
            n = n.saturating_sub(ei.norm[kk]);
            kk += ei.norm[kk];
        }

        step += 1;
        if step > 64 {
            break;
        } // safety
    }

    Some((ei, f))
}

// ── Position encoding ─────────────────────────────────────────────────────────

fn encode_position(p: &mut [i32; TB_PIECES], ei: &EncInfo, meta: &TableMeta, enc: Enc) -> usize {
    let tabs = tables();
    let n = meta.num;

    // Fold file symmetry: if p[0] is on files e-h, flip file axis
    if p[0] & 0x04 != 0 {
        for sq in p[..n].iter_mut() {
            *sq ^= 0x07;
        }
    }

    let idx;
    let k;

    if enc == Enc::Piece {
        // Fold rank symmetry: if p[0] is on ranks 5-8, flip rank axis
        if p[0] & 0x20 != 0 {
            for sq in p[..n].iter_mut() {
                *sq ^= 0x38;
            }
        }

        // Diagonal flip
        for i in 0..n {
            let od = OFF_DIAG[p[i] as usize];
            if od != 0 {
                if od > 0 && i < if meta.kk_enc { 2 } else { 3 } {
                    for sq in p[..n].iter_mut() {
                        *sq = FLIP_DIAG[*sq as usize] as i32;
                    }
                }
                break;
            }
        }

        if meta.kk_enc {
            idx = KK_IDX[TRIANGLE[p[0] as usize] as usize][p[1] as usize] as usize;
            k = 2;
        } else {
            let s1 = (p[1] > p[0]) as i32;
            let s2 = (p[2] > p[0]) as i32 + (p[2] > p[1]) as i32;
            let od0 = OFF_DIAG[p[0] as usize];
            let od1 = OFF_DIAG[p[1] as usize];
            let od2 = OFF_DIAG[p[2] as usize];
            idx = if od0 != 0 {
                TRIANGLE[p[0] as usize] as usize * 63 * 62
                    + (p[1] - s1) as usize * 62
                    + (p[2] - s2) as usize
            } else if od1 != 0 {
                6 * 63 * 62
                    + DIAG[p[0] as usize] as usize * 28 * 62
                    + LOWER[p[1] as usize] as usize * 62
                    + (p[2] - s2) as usize
            } else if od2 != 0 {
                6 * 63 * 62
                    + 4 * 28 * 62
                    + DIAG[p[0] as usize] as usize * 7 * 28
                    + (DIAG[p[1] as usize] as i32 - s1) as usize * 28
                    + LOWER[p[2] as usize] as usize
            } else {
                6 * 63 * 62
                    + 4 * 28 * 62
                    + 4 * 7 * 28
                    + DIAG[p[0] as usize] as usize * 7 * 6
                    + (DIAG[p[1] as usize] as i32 - s1) as usize * 6
                    + (DIAG[p[2] as usize] as i32 - s2) as usize
            };
            k = 3;
        }
    } else {
        // Pawn encoding
        // Sort leading pawns by Flap descending
        let enc_idx = 0usize; // FILE_ENC uses index 0
        let num_pawns0 = meta.pawns[0];
        for i in 1..num_pawns0 {
            for j in i + 1..num_pawns0 {
                if FLAP[enc_idx][p[i] as usize] < FLAP[enc_idx][p[j] as usize] {
                    p.swap(i, j);
                }
            }
        }

        let leading = FILE_TO_FILE[p[0] as usize & 7];
        let k0 = num_pawns0;
        let mut base_idx = tabs.pawn_idx[enc_idx][k0 - 1][FLAP[enc_idx][p[0] as usize] as usize];
        for i in 1..k0 {
            base_idx += tabs.binomial[k0 - i][PAWN_TWIST[enc_idx][p[i] as usize] as usize];
        }
        idx = base_idx;
        k = k0;
        let _ = leading; // used below via factor[0]
    }

    let mut result = idx * ei.factor[0];

    // Handle opponent's pawns for pawn tables
    let mut kk = k;
    if enc != Enc::Piece && meta.pawns[1] > 0 {
        let t2 = kk + meta.pawns[1];
        // Sort by square
        for i in kk..t2 {
            for j in i + 1..t2 {
                if p[i] > p[j] {
                    p.swap(i, j);
                }
            }
        }
        let mut s = 0usize;
        for i in kk..t2 {
            let sq = p[i] as usize;
            let skips: usize = (0..kk).map(|j| (sq > p[j] as usize) as usize).sum();
            s += tabs.binomial[i - kk + 1][sq.saturating_sub(skips + 8)];
        }
        result += s * ei.factor[kk];
        kk = t2;
    }

    // Remaining non-pawn pieces
    while kk < n {
        let t = kk + ei.norm[kk];
        // Sort group
        for i in kk..t {
            for j in i + 1..t {
                if p[i] > p[j] {
                    p.swap(i, j);
                }
            }
        }
        let mut s = 0usize;
        for i in kk..t {
            let sq = p[i] as usize;
            let skips: usize = (0..kk).map(|j| (sq > p[j] as usize) as usize).sum();
            s += tabs.binomial[i - kk + 1][sq.saturating_sub(skips)];
        }
        result += s * ei.factor[kk];
        kk = t;
    }

    result
}

// ── fill_squares ──────────────────────────────────────────────────────────────

/// Fill `p[start..]` with squares for the piece type `piece_code`.
/// Returns the new index (start + number of squares filled).
fn fill_squares_for_piece(
    piece_code: u8,
    flip: bool,
    mirror: i32,
    white: u64,
    black: u64,
    kings: u64,
    queens: u64,
    rooks: u64,
    bishops: u64,
    knights: u64,
    pawns: u64,
    p: &mut [i32; TB_PIECES],
    start: usize,
) -> usize {
    // Color: W_PAWN..W_KING are 1-6 (< 8), B_PAWN..B_KING are 9-14 (>= 8)
    let is_white_piece = piece_code < 8;
    let color_is_white = if flip {
        !is_white_piece
    } else {
        is_white_piece
    };

    let piece_type = piece_code & 0x07; // 1=pawn..6=king (same for both colors)
    let color_bb = if color_is_white { white } else { black };
    let type_bb = match piece_type {
        1 => pawns,
        2 => knights,
        3 => bishops,
        4 => rooks,
        5 => queens,
        6 => kings,
        _ => 0,
    };
    let bb = color_bb & type_bb;

    let mut idx = start;
    let mut b = bb;
    while b != 0 {
        let sq = b.trailing_zeros() as i32;
        if idx < TB_PIECES {
            p[idx] = sq ^ mirror;
        }
        b &= b - 1;
        idx += 1;
    }
    idx
}

fn fill_all_squares(
    ei: &EncInfo,
    meta: &TableMeta,
    flip: bool,
    mirror: i32,
    white: u64,
    black: u64,
    kings: u64,
    queens: u64,
    rooks: u64,
    bishops: u64,
    knights: u64,
    pawns: u64,
    p: &mut [i32; TB_PIECES],
) {
    let mut i = 0;
    while i < meta.num {
        i = fill_squares_for_piece(
            ei.pieces[i],
            flip,
            mirror,
            white,
            black,
            kings,
            queens,
            rooks,
            bishops,
            knights,
            pawns,
            p,
            i,
        );
    }
}

#[allow(dead_code)]
fn leading_pawn(p: &mut [i32; TB_PIECES], meta: &TableMeta) -> usize {
    let n0 = meta.pawns[0];
    for i in 1..n0 {
        if FLAP[0][p[0] as usize] > FLAP[0][p[i] as usize] {
            p.swap(0, i);
        }
    }
    FILE_TO_FILE[p[0] as usize & 7]
}

// ── Top-level probers ─────────────────────────────────────────────────────────

/// Probe a .rtbw file (WDL) using the real Syzygy binary format.
///
/// `data` is the memory-mapped (or buffered) file bytes.
/// `flip` = true if piece colors were swapped for canonical ordering.
/// `turn_is_white` = true if the side to move is white (in the original, pre-flip orientation).
pub(crate) fn probe_wdl_syzygy(
    data: &[u8],
    meta: &TableMeta,
    flip: bool,
    turn_is_white: bool,
    white: u64,
    black: u64,
    kings: u64,
    queens: u64,
    rooks: u64,
    bishops: u64,
    knights: u64,
    pawns: u64,
) -> Option<WdlValue> {
    // Validate magic
    if data.len() < 5 {
        return None;
    }
    let magic = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    if magic != WDL_MAGIC {
        return None;
    }

    let split = data[4] & 0x01 != 0;
    let enc = if meta.has_pawns {
        Enc::File
    } else {
        Enc::Piece
    };
    let num_tables = if meta.has_pawns { 4 } else { 1 };

    // bside: selects which of the two EI halves to use
    // In C: bside = (turn == WHITE) == flip
    let bside = turn_is_white == flip;

    // ── Phase 1: parse encoding headers ──────────────────────────────────────
    let mut data_off = 5usize;
    let mut ei_main: Vec<(EncInfo, usize)> = Vec::new();
    let mut ei_split: Vec<(EncInfo, usize)> = Vec::new();

    let header_step = meta.num + 1 + (meta.has_pawns && meta.pawns[1] > 0) as usize;

    for t in 0..num_tables {
        let tb = &data[data_off..];
        let (ei, tb_size) = init_enc_info(meta, tb, 0, t, enc)?;
        ei_main.push((ei, tb_size));
        if split {
            let (ei_s, tb_sz_s) = init_enc_info(meta, tb, 4, t, enc)?;
            ei_split.push((ei_s, tb_sz_s));
        }
        data_off += header_step;
    }

    // 2-byte alignment
    if data_off & 1 != 0 {
        data_off += 1;
    }

    // ── Phase 2: parse PairsData headers ──────────────────────────────────────
    let mut pd_main: Vec<PairsData> = Vec::new();
    let mut pd_split: Vec<PairsData> = Vec::new();

    for t in 0..num_tables {
        let (pd, next, _flags) = setup_pairs(data, data_off, ei_main[t].1, true).ok()?;
        data_off = next;
        pd_main.push(pd);

        if split {
            let (pd_s, next_s, _) = setup_pairs(data, data_off, ei_split[t].1, true).ok()?;
            data_off = next_s;
            pd_split.push(pd_s);
        }
    }

    // ── Phase 3: assign index / size / data section byte-offsets ─────────────
    // Index tables
    for t in 0..num_tables {
        pd_main[t].index_table_off = data_off;
        data_off += pd_main[t].idx_table_size;
        if split {
            pd_split[t].index_table_off = data_off;
            data_off += pd_split[t].idx_table_size;
        }
    }
    // Size tables
    for t in 0..num_tables {
        pd_main[t].size_table_off = data_off;
        data_off += pd_main[t].size_table_size;
        if split {
            pd_split[t].size_table_off = data_off;
            data_off += pd_split[t].size_table_size;
        }
    }
    // Compressed data (64-byte aligned)
    for t in 0..num_tables {
        data_off = (data_off + 0x3f) & !0x3f;
        pd_main[t].data_off = data_off;
        data_off += pd_main[t].data_size;
        if split {
            data_off = (data_off + 0x3f) & !0x3f;
            pd_split[t].data_off = data_off;
            data_off += pd_split[t].data_size;
        }
    }

    // ── Phase 4: probe ────────────────────────────────────────────────────────
    let (_t_idx, ei_ref, pd_ref) = if meta.has_pawns {
        // Need to determine leading-pawn file index t
        let mirror = if flip { 0x38i32 } else { 0i32 };
        let mut p = [0i32; TB_PIECES];
        // Fill just the first group (pawns) to find leading pawn
        let mut i = 0;
        while i < meta.num {
            i = fill_squares_for_piece(
                ei_main[0].0.pieces[i],
                flip,
                mirror,
                white,
                black,
                kings,
                queens,
                rooks,
                bishops,
                knights,
                pawns,
                &mut p,
                i,
            );
            if i >= meta.pawns[0] {
                break;
            }
        }
        let t = leading_pawn(&mut p, meta).min(num_tables - 1);

        let (ei, pd) = if split && bside {
            (&ei_split[t].0, &pd_split[t])
        } else {
            (&ei_main[t].0, &pd_main[t])
        };
        (t, ei, pd)
    } else {
        // Piece table: bside selects EI
        let (ei, pd) = if split && bside {
            (&ei_split[0].0, &pd_split[0])
        } else {
            (&ei_main[0].0, &pd_main[0])
        };
        (0, ei, pd)
    };

    // Fill all squares
    let mirror = if meta.has_pawns && flip {
        0x38i32
    } else {
        0i32
    };
    let mut p = [0i32; TB_PIECES];
    fill_all_squares(
        ei_ref, meta, flip, mirror, white, black, kings, queens, rooks, bishops, knights, pawns,
        &mut p,
    );

    let idx = encode_position(&mut p, ei_ref, meta, enc);
    let w = decompress_pairs(data, pd_ref, idx);

    // WDL: w[0] encodes 0(loss)..4(win), mapped to -2..2
    let raw = (w[0] as i32) - 2;
    Some(match raw {
        -2 => WdlValue::Loss,
        -1 => WdlValue::BlessedLoss,
        0 => WdlValue::Draw,
        1 => WdlValue::CursedWin,
        _ => WdlValue::Win,
    })
}

/// Probe a .rtbz file (DTZ) using the real Syzygy binary format.
///
/// `wdl` is the WDL result for this position (-2..2), needed for DTZ map lookup.
pub(crate) fn probe_dtz_syzygy(
    data: &[u8],
    meta: &TableMeta,
    flip: bool,
    turn_is_white: bool,
    wdl: i32,
    white: u64,
    black: u64,
    kings: u64,
    queens: u64,
    rooks: u64,
    bishops: u64,
    knights: u64,
    pawns: u64,
) -> Option<i32> {
    if data.len() < 5 {
        return None;
    }
    let magic = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    if magic != DTZ_MAGIC {
        return None;
    }

    // DTZ tables are never split (one table per leading-pawn file)
    let enc = if meta.has_pawns {
        Enc::File
    } else {
        Enc::Piece
    };
    let num_tables = if meta.has_pawns { 4 } else { 1 };
    let bside = turn_is_white == flip;

    let mut data_off = 5usize;

    // ── Phase 1: parse encoding headers (DTZ: no split, shift=0 only) ─────────
    let mut ei_vec: Vec<(EncInfo, usize)> = Vec::new();
    let header_step = meta.num + 1 + (meta.has_pawns && meta.pawns[1] > 0) as usize;
    for t in 0..num_tables {
        let tb = &data[data_off..];
        let (ei, tb_size) = init_enc_info(meta, tb, 0, t, enc)?;
        ei_vec.push((ei, tb_size));
        data_off += header_step;
    }
    if data_off & 1 != 0 {
        data_off += 1;
    }

    // ── Phase 2: parse PairsData headers ──────────────────────────────────────
    let mut pd_vec: Vec<PairsData> = Vec::new();
    let mut flags_vec: Vec<u8> = Vec::new();
    for t in 0..num_tables {
        let (pd, next, flags) = setup_pairs(data, data_off, ei_vec[t].1, false).ok()?;
        data_off = next;
        pd_vec.push(pd);
        flags_vec.push(flags);
    }

    // ── Phase 3: parse DTZ map ────────────────────────────────────────────────
    // The dtz map follows immediately after the sym_pat sections.
    let dtz_map_off = data_off; // base for all map lookups
    let mut map_idx = vec![[0u16; 4]; num_tables];

    for t in 0..num_tables {
        let fl = flags_vec[t];
        if fl & 2 != 0 {
            if fl & 16 == 0 {
                // byte-sized map entries
                for i in 0..4usize {
                    if data_off >= data.len() {
                        return None;
                    }
                    let map_start = data_off as u16;
                    map_idx[t][i] = map_start + 1;
                    data_off += 1 + data[data_off] as usize;
                }
            } else {
                // u16-sized map entries
                if data_off & 1 != 0 {
                    data_off += 1;
                }
                for i in 0..4usize {
                    if data_off + 2 > data.len() {
                        return None;
                    }
                    map_idx[t][i] = (data_off / 2 + 1) as u16;
                    let cnt = u16::from_le_bytes([data[data_off], data[data_off + 1]]) as usize;
                    data_off += 2 + 2 * cnt;
                }
            }
        }
    }
    if data_off & 1 != 0 {
        data_off += 1;
    }

    // ── Phase 4: assign index / size / data byte-offsets ──────────────────────
    for t in 0..num_tables {
        pd_vec[t].index_table_off = data_off;
        data_off += pd_vec[t].idx_table_size;
    }
    for t in 0..num_tables {
        pd_vec[t].size_table_off = data_off;
        data_off += pd_vec[t].size_table_size;
    }
    for t in 0..num_tables {
        data_off = (data_off + 0x3f) & !0x3f;
        pd_vec[t].data_off = data_off;
        data_off += pd_vec[t].data_size;
    }

    // ── Phase 5: probe ────────────────────────────────────────────────────────
    let t_idx = if meta.has_pawns {
        // Determine leading-pawn file index
        let mirror = if flip { 0x38i32 } else { 0i32 };
        let mut p = [0i32; TB_PIECES];
        let mut i = 0;
        while i < meta.pawns[0] {
            i = fill_squares_for_piece(
                ei_vec[0].0.pieces[i],
                flip,
                mirror,
                white,
                black,
                kings,
                queens,
                rooks,
                bishops,
                knights,
                pawns,
                &mut p,
                i,
            );
        }
        leading_pawn(&mut p, meta).min(num_tables - 1)
    } else {
        0
    };

    // Check bside vs flags
    let fl = flags_vec[t_idx];
    if (fl & 1) as usize != bside as usize && !meta.symmetric {
        return None; // table doesn't cover this side-to-move
    }

    let ei_ref = &ei_vec[t_idx].0;
    let pd_ref = &pd_vec[t_idx];

    let mirror = if meta.has_pawns && flip {
        0x38i32
    } else {
        0i32
    };
    let mut p = [0i32; TB_PIECES];
    fill_all_squares(
        ei_ref, meta, flip, mirror, white, black, kings, queens, rooks, bishops, knights, pawns,
        &mut p,
    );

    let idx = encode_position(&mut p, ei_ref, meta, enc);
    let w = decompress_pairs(data, pd_ref, idx);

    let mut v = w[0] as i32 + ((w[1] as i32 & 0x0f) << 8);

    // Apply DTZ map if present
    if fl & 2 != 0 {
        let m = WDL_TO_MAP[(wdl + 2) as usize];
        let map_base = dtz_map_off;
        let midx = map_idx[t_idx][m] as usize;
        if fl & 16 == 0 {
            // byte map
            let byte_off = map_base + midx.saturating_sub(1) + v as usize;
            if byte_off < data.len() {
                v = data[byte_off] as i32;
            }
        } else {
            // u16 map
            let u16_off = (midx.saturating_sub(1)) * 2 + v as usize * 2;
            let byte_off = map_base + u16_off;
            if byte_off + 2 <= data.len() {
                v = u16::from_le_bytes([data[byte_off], data[byte_off + 1]]) as i32;
            }
        }
    }
    if PA_FLAGS[(wdl + 2) as usize] & fl == 0 || (wdl & 1) != 0 {
        v *= 2;
    }

    Some(v)
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_material_key_kqvk() {
        let m = parse_material_key("kqvk").unwrap();
        assert_eq!(m.num, 3);
        assert!(!m.has_pawns);
        assert!(!m.kk_enc);
        assert!(!m.symmetric);
    }

    #[test]
    fn test_parse_material_key_kvk() {
        let m = parse_material_key("kvk").unwrap();
        assert_eq!(m.num, 2);
        assert!(!m.has_pawns);
        assert!(m.kk_enc);
        assert!(m.symmetric);
    }

    #[test]
    fn test_parse_material_key_kqpvkr() {
        let m = parse_material_key("kqpvkr").unwrap();
        assert_eq!(m.num, 5);
        assert!(m.has_pawns);
        assert!(!m.kk_enc);
    }

    #[test]
    fn test_tables_binomial_c2() {
        // C(2,1)=1, C(2,2)=1, C(2,3)=3... wait C(n,k): C(3,2)=3
        let t = tables();
        assert_eq!(t.binomial[0][5], 1); // C(5,0)=1
        assert_eq!(t.binomial[1][5], 5); // C(5,1)=5
        assert_eq!(t.binomial[2][5], 10); // C(5,2)=10
        assert_eq!(t.binomial[3][5], 10); // C(5,3)=10
    }

    #[test]
    fn test_magic_constants() {
        assert_eq!(WDL_MAGIC, 0x5d23e871);
        assert_eq!(DTZ_MAGIC, 0xa50c66d7);
    }

    #[test]
    fn test_wdl_rejects_wrong_magic() {
        let data = [0u8; 64];
        let meta = parse_material_key("kvk").unwrap();
        let result = probe_wdl_syzygy(&data, &meta, false, true, 0, 0, 0, 0, 0, 0, 0, 0);
        assert!(result.is_none());
    }
}
