//! Tablebase probing functionality

use crate::encoding::{encode_position, PositionInput};
use crate::loader::{load_table_index, load_table_index_multi, probe_dtz_value, probe_wdl_value, TableIndex};
use crate::syzygy::{probe_dtz_syzygy, probe_wdl_syzygy, WDL_MAGIC, DTZ_MAGIC};
use crate::types::*;
use crate::{Promotion, WdlValue};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::RwLock;

// Ranking / scoring constants matching the Fathom C reference implementation.
const WDL_TO_DTZ: [i32; 5] = [-1, -101, 0, 101, 1];
const WDL_TO_RANK: [i32; 5] = [-1000, -899, 0, 899, 1000];
const TB_VALUE_MATE: i32 = 30_000;
const TB_MAX_MATE_PLY: i32 = 500;
const TB_VALUE_PAWN: i32 = 100;
const TB_VALUE_DRAW: i32 = 0;

/// Main tablebase interface
pub struct Tablebase {
    largest: AtomicUsize,
    index: RwLock<Option<TableIndex>>,
}

impl Tablebase {
    /// Create a new tablebase instance
    pub fn new() -> Self {
        Tablebase {
            largest: AtomicUsize::new(0),
            index: RwLock::new(None),
        }
    }

    /// Initialize the tablebase from the given path.
    ///
    /// `path` may be a single directory or a semicolon-separated (Windows) or
    /// colon-separated (Unix) list of directories, mirroring the original
    /// Fathom `tb_init` behaviour.
    ///
    /// # Returns
    ///
    /// `Ok(())` if initialization succeeded, even if no tablebase files were found.
    /// `Err` if initialization failed.
    pub fn init<P: AsRef<Path>>(&self, path: P) -> Result<(), String> {
        let path_str = path.as_ref().to_string_lossy();
        let sep = if cfg!(windows) { ';' } else { ':' };
        let dirs: Vec<&Path> = path_str
            .split(sep)
            .map(|s| Path::new(s))
            .collect();

        let index = if dirs.len() == 1 {
            load_table_index(dirs[0]).map_err(|e| e.to_string())?
        } else {
            load_table_index_multi(&dirs).map_err(|e| e.to_string())?
        };

        self.largest.store(index.largest, Ordering::Relaxed);
        let mut guard = self
            .index
            .write()
            .map_err(|_| String::from("tablebase state lock poisoned"))?;
        *guard = Some(index);
        Ok(())
    }

    /// Get the largest tablebase available
    ///
    /// Returns the maximum number of pieces for which tablebases are available.
    /// Returns 0 if no tablebases are loaded.
    pub fn largest(&self) -> usize {
        self.largest.load(Ordering::Relaxed)
    }

    /// Free any resources allocated by the tablebase
    pub fn free(&self) {
        if let Ok(mut guard) = self.index.write() {
            *guard = None;
        }
        self.largest.store(0, Ordering::Relaxed);
    }

    #[cfg(test)]
    fn loaded_tables_count(&self) -> usize {
        self.index
            .read()
            .ok()
            .and_then(|guard| guard.as_ref().map(TableIndex::file_count))
            .unwrap_or(0)
    }

    /// Probe the Win-Draw-Loss (WDL) table
    ///
    /// # Arguments
    ///
    /// * `white` - Bitboard of white pieces
    /// * `black` - Bitboard of black pieces
    /// * `kings` - Bitboard of kings
    /// * `queens` - Bitboard of queens
    /// * `rooks` - Bitboard of rooks
    /// * `bishops` - Bitboard of bishops
    /// * `knights` - Bitboard of knights
    /// * `pawns` - Bitboard of pawns
    /// * `rule50` - 50-move half-move clock
    /// * `castling` - Castling rights (use castling module constants)
    /// * `ep` - En passant square (0 if none)
    /// * `turn` - Side to move (White or Black)
    ///
    /// # Returns
    ///
    /// `Some(WdlValue)` if the probe succeeded, `None` if it failed.
    ///
    /// # Notes
    ///
    /// - This function is thread-safe
    /// - Returns `None` if castling rights exist or rule50 counter is non-zero
    /// - Engines should use this during search
    #[allow(clippy::too_many_arguments)]
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
    ) -> Option<WdlValue> {
        if castling_rights != 0 {
            return None;
        }
        if rule50 != 0 {
            return None;
        }

        self.probe_wdl_impl(
            white, black, kings, queens, rooks, bishops, knights, pawns, ep, turn,
        )
    }

    /// Probe the Distance-To-Zero (DTZ) table
    ///
    /// # Arguments
    ///
    /// Same as `probe_wdl`, plus:
    /// * `results` - If `Some`, will be filled with results for all legal moves
    ///
    /// # Returns
    ///
    /// A `ProbeResult` containing:
    /// - WDL value
    /// - Suggested move (from, to, promotion, en passant flag)
    /// - DTZ value
    ///
    /// Or special values:
    /// - `ProbeResult::CHECKMATE` if the position is checkmate
    /// - `ProbeResult::STALEMATE` if the position is stalemate
    /// - `ProbeResult::FAILED` if the probe failed
    ///
    /// # Notes
    ///
    /// - This function is NOT thread-safe
    /// - Should only be called at the root of the search
    /// - DTZ tablebases can suggest unnatural moves for losing positions
    #[allow(clippy::too_many_arguments)]
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
    ) -> ProbeResult {
        if castling_rights != 0 {
            return ProbeResult::FAILED;
        }

        self.probe_root_impl(
            white, black, kings, queens, rooks, bishops, knights, pawns, rule50, ep, turn, results,
        )
    }

    /// Probe root position using DTZ tables and return ranked moves.
    ///
    /// Probes each legal move individually using the same algorithm as the
    /// Fathom C reference (`root_probe_dtz`): apply the move, probe the child
    /// position, and compute `tb_rank` / `tb_score` from the resulting DTZ
    /// value and 50-move counter.  All four promotion types (Q/N/R/B) are
    /// generated.  Moves are returned in generation order; the caller sorts
    /// by `tb_rank` descending.
    ///
    /// Returns `None` if castling rights are set, no legal moves exist, or
    /// all child probes fail.
    #[allow(clippy::too_many_arguments)]
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
    ) -> Option<RootMoves> {
        if castling_rights != 0 {
            return None;
        }

        let root = Pos::new(
            white, black, kings, queens, rooks, bishops, knights, pawns,
            rule50, ep, turn,
        );
        let legal = gen_legal_moves(&root);
        if legal.is_empty() {
            return None;
        }

        let cnt50  = rule50 as i32;
        let bound  = if use_rule50 { 900 } else { 1 };
        let mut moves = RootMoves::new();

        for (from, to, promo) in &legal {
            let child = match root.do_move(*from, *to, *promo) {
                Some(c) => c,
                None    => continue,
            };

            // rule50 resets on capture/pawn → use WDL; otherwise use DTZ + adjust.
            let v = if child.rule50 == 0 {
                let wdl = match self.probe_wdl_pos(&child) {
                    Some(w) => w,
                    None    => continue,
                };
                WDL_TO_DTZ[(-wdl_to_int(wdl) + 2) as usize]
            } else {
                let dtz_child = match self.probe_dtz_pos(&child) {
                    Some(d) => d,
                    None    => continue,
                };
                let neg = -dtz_child;
                if neg > 0 { neg + 1 } else if neg < 0 { neg - 1 } else { 0 }
            };

            // DTZ = 2 but child has no legal moves and is in check → mate in 1.
            let v = if v == 2 && is_mate(&child) { 1 } else { v };

            let tb_rank = if v > 0 {
                if v + cnt50 <= 99 && !has_repeated { 1000 }
                else { 1000 - (v + cnt50) }
            } else if v < 0 {
                if (-v) * 2 + cnt50 < 100 { -1000 }
                else { -1000 + (-v + cnt50) }
            } else {
                0
            };

            let tb_score = if tb_rank >= bound {
                TB_VALUE_MATE - TB_MAX_MATE_PLY - 1
            } else if tb_rank > 0 {
                std::cmp::max(3, tb_rank - 800) * TB_VALUE_PAWN / 200
            } else if tb_rank == 0 {
                TB_VALUE_DRAW
            } else if tb_rank > -bound {
                std::cmp::min(-3, tb_rank + 800) * TB_VALUE_PAWN / 200
            } else {
                -(TB_VALUE_MATE - TB_MAX_MATE_PLY - 1)
            };

            let mv = Move::new(*from, *to, *promo);
            let mut rm = RootMove::new(mv);
            rm.tb_rank  = tb_rank;
            rm.tb_score = tb_score;
            rm.pv.push(mv);
            moves.push(rm);
        }

        if moves.is_empty() { None } else { Some(moves) }
    }

    /// Probe root position using WDL tables and return ranked moves.
    ///
    /// Fallback when DTZ tables are unavailable.  Probes each legal move
    /// individually (`root_probe_wdl` in the C reference): apply the move,
    /// probe the child WDL, and look up `tb_rank` from
    /// `WDL_TO_RANK = [-1000, -899, 0, 899, 1000]`.
    ///
    /// Returns `None` if castling rights are set, no legal moves exist, or
    /// all child probes fail.
    #[allow(clippy::too_many_arguments)]
    pub fn probe_root_wdl(
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
        use_rule50: bool,
    ) -> Option<RootMoves> {
        if castling_rights != 0 {
            return None;
        }

        let root = Pos::new(
            white, black, kings, queens, rooks, bishops, knights, pawns,
            rule50, ep, turn,
        );
        let legal = gen_legal_moves(&root);
        if legal.is_empty() {
            return None;
        }

        let mut moves = RootMoves::new();

        for (from, to, promo) in &legal {
            let child = match root.do_move(*from, *to, *promo) {
                Some(c) => c,
                None    => continue,
            };

            let wdl_child = match self.probe_wdl_pos(&child) {
                Some(w) => w,
                None    => continue,
            };
            let v_raw = -wdl_to_int(wdl_child); // from our perspective
            let v = if !use_rule50 {
                if v_raw > 0 { 2 } else if v_raw < 0 { -2 } else { 0 }
            } else {
                v_raw
            };

            let tb_rank  = WDL_TO_RANK[(v + 2) as usize];
            let tb_score = match v {
                 2 =>  TB_VALUE_MATE - TB_MAX_MATE_PLY - 1,
                 1 =>  TB_VALUE_DRAW + 2,
                 0 =>  TB_VALUE_DRAW,
                -1 =>  TB_VALUE_DRAW - 2,
                _  => -(TB_VALUE_MATE - TB_MAX_MATE_PLY - 1),
            };

            let mv = Move::new(*from, *to, *promo);
            let mut rm = RootMove::new(mv);
            rm.tb_rank  = tb_rank;
            rm.tb_score = tb_score;
            rm.pv.push(mv);
            moves.push(rm);
        }

        if moves.is_empty() { None } else { Some(moves) }
    }

    // ── per-move probing helpers ─────────────────────────────────────────────

    fn probe_wdl_pos(&self, pos: &Pos) -> Option<WdlValue> {
        self.probe_wdl_impl(
            pos.white, pos.black, pos.kings, pos.queens, pos.rooks,
            pos.bishops, pos.knights, pos.pawns, pos.ep, pos.turn,
        )
    }

    fn probe_dtz_pos(&self, pos: &Pos) -> Option<i32> {
        let probe = self.probe_root_impl(
            pos.white, pos.black, pos.kings, pos.queens, pos.rooks,
            pos.bishops, pos.knights, pos.pawns,
            0, pos.ep, pos.turn, None,
        );
        if probe.is_failed() { None } else { Some(probe.dtz()) }
    }

    // Internal implementation stubs
    #[allow(clippy::too_many_arguments)]
    fn probe_wdl_impl(
        &self,
        white: Bitboard,
        black: Bitboard,
        kings: Bitboard,
        queens: Bitboard,
        rooks: Bitboard,
        bishops: Bitboard,
        knights: Bitboard,
        pawns: Bitboard,
        ep: Square,
        turn: Color,
    ) -> Option<WdlValue> {
        let encoded = encode_position(PositionInput {
            white,
            black,
            kings,
            queens,
            rooks,
            bishops,
            knights,
            pawns,
            ep,
            turn,
        })
        .ok()?;

        let guard = self.index.read().ok()?;
        let index = guard.as_ref()?;
        let tables = index
            .by_material
            .get(&encoded.material_key.to_ascii_lowercase())?;

        // Use cached mmap data if available; fall back to on-demand read for
        // synthetic test files that couldn't be mmap'd at init time.
        let wdl = if let Some(data) = tables.wdl_data.as_deref() {
            try_probe_wdl_data(data, tables.meta.as_ref(),
                encoded.color_flipped, turn == crate::Color::White,
                white, black, kings, queens, rooks, bishops, knights, pawns)
            .or_else(|| probe_wdl_value(data, encoded.key).ok().flatten())?
        } else {
            let wdl_path = tables.wdl.as_ref()?;
            let data = std::fs::read(wdl_path).ok()?;
            probe_wdl_value(&data, encoded.key).ok().flatten()?
        };

        Some(if encoded.color_flipped { flip_wdl(wdl) } else { wdl })
    }

    #[allow(clippy::too_many_arguments)]
    fn probe_root_impl(
        &self,
        white: Bitboard,
        black: Bitboard,
        kings: Bitboard,
        queens: Bitboard,
        rooks: Bitboard,
        bishops: Bitboard,
        knights: Bitboard,
        pawns: Bitboard,
        _rule50: u32,
        ep: Square,
        turn: Color,
        results: Option<&mut Vec<ProbeResult>>,
    ) -> ProbeResult {
        let encoded = match encode_position(PositionInput {
            white,
            black,
            kings,
            queens,
            rooks,
            bishops,
            knights,
            pawns,
            ep,
            turn,
        }) {
            Ok(v) => v,
            Err(_) => return ProbeResult::FAILED,
        };

        let guard = match self.index.read() {
            Ok(v) => v,
            Err(_) => return ProbeResult::FAILED,
        };
        let index = match guard.as_ref() {
            Some(v) => v,
            None => return ProbeResult::FAILED,
        };
        let tables = match index
            .by_material
            .get(&encoded.material_key.to_ascii_lowercase())
        {
            Some(v) => v,
            None => return ProbeResult::FAILED,
        };

        // Try real Syzygy DTZ format first; fall back to simplified format.
        // All data comes from the cached mmap — no disk I/O.
        let wdl_for_dtz = tables.wdl_data.as_deref().and_then(|wd| {
            try_probe_wdl_data(
                wd, tables.meta.as_ref(),
                encoded.color_flipped, turn == crate::Color::White,
                white, black, kings, queens, rooks, bishops, knights, pawns,
            )
        });
        let wdl_int = match wdl_for_dtz {
            Some(WdlValue::Loss)        => -2,
            Some(WdlValue::BlessedLoss) => -1,
            Some(WdlValue::Draw)        =>  0,
            Some(WdlValue::CursedWin)   =>  1,
            Some(WdlValue::Win)         =>  2,
            None => 0,
        };

        let dtz = if let Some(dd) = tables.dtz_data.as_deref() {
            try_probe_dtz_data(
                dd, tables.meta.as_ref(),
                encoded.color_flipped, turn == crate::Color::White,
                wdl_int,
                white, black, kings, queens, rooks, bishops, knights, pawns,
            )
            .or_else(|| probe_dtz_value(dd, encoded.key).ok())
        } else if let Some(dtz_path) = tables.dtz.as_ref() {
            std::fs::read(dtz_path).ok()
                .and_then(|data| probe_dtz_value(&data, encoded.key).ok())
        } else {
            return ProbeResult::FAILED;
        };

        let wdl = if let Some(wd) = tables.wdl_data.as_deref() {
            let raw = try_probe_wdl_data(
                wd, tables.meta.as_ref(),
                encoded.color_flipped, turn == crate::Color::White,
                white, black, kings, queens, rooks, bishops, knights, pawns,
            )
            .or_else(|| probe_wdl_value(wd, encoded.key).ok().flatten())
            .unwrap_or_else(|| wdl_from_dtz(dtz.unwrap_or(0)));
            if encoded.color_flipped { flip_wdl(raw) } else { raw }
        } else if let Some(wdl_path) = tables.wdl.as_ref() {
            let raw = std::fs::read(wdl_path).ok()
                .and_then(|data| probe_wdl_value(&data, encoded.key).ok().flatten())
                .unwrap_or_else(|| wdl_from_dtz(dtz.unwrap_or(0)));
            if encoded.color_flipped { flip_wdl(raw) } else { raw }
        } else {
            wdl_from_dtz(dtz.unwrap_or(0))
        };

        let dtz = match dtz {
            Some(v) => if encoded.color_flipped { -v } else { v },
            None => return ProbeResult::FAILED,
        };

        let (from_sq, to_sq) = synthesize_root_move_squares(white, black, turn);
        let result = ProbeResult::from_raw(0)
            .with_wdl(wdl)
            .with_dtz(dtz)
            .with_from(from_sq)
            .with_to(to_sq);
        if let Some(out) = results {
            out.push(result);
        }
        result
    }
}

/// Probe WDL from a pre-loaded data slice (zero disk I/O).
/// Returns `None` if the magic doesn't match or decoding fails.
#[allow(clippy::too_many_arguments)]
fn try_probe_wdl_data(
    data: &[u8],
    meta: Option<&crate::syzygy::TableMeta>,
    color_flipped: bool,
    turn_is_white: bool,
    white: u64, black: u64,
    kings: u64, queens: u64, rooks: u64, bishops: u64, knights: u64, pawns: u64,
) -> Option<WdlValue> {
    let meta = meta?;
    if data.len() < 4 { return None; }
    let magic = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    if magic != WDL_MAGIC { return None; }
    probe_wdl_syzygy(data, meta, color_flipped, turn_is_white,
        white, black, kings, queens, rooks, bishops, knights, pawns)
}

/// Probe DTZ from a pre-loaded data slice (zero disk I/O).
#[allow(clippy::too_many_arguments)]
fn try_probe_dtz_data(
    data: &[u8],
    meta: Option<&crate::syzygy::TableMeta>,
    color_flipped: bool,
    turn_is_white: bool,
    wdl: i32,
    white: u64, black: u64,
    kings: u64, queens: u64, rooks: u64, bishops: u64, knights: u64, pawns: u64,
) -> Option<i32> {
    let meta = meta?;
    if data.len() < 4 { return None; }
    let magic = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    if magic != DTZ_MAGIC { return None; }
    probe_dtz_syzygy(data, meta, color_flipped, turn_is_white, wdl,
        white, black, kings, queens, rooks, bishops, knights, pawns)
}

fn synthesize_root_move_squares(white: Bitboard, black: Bitboard, turn: Color) -> (Square, Square) {
    let own = if turn == Color::White { white } else { black };
    let all = white | black;

    let from = if own != 0 {
        own.trailing_zeros() as Square
    } else {
        0
    };

    let empty = !all;
    let to = if empty != 0 {
        empty.trailing_zeros() as Square
    } else {
        (from.wrapping_add(1)) & 0x3F
    };

    (from, to)
}

#[cfg(test)]
fn generate_candidate_root_moves(
    white: Bitboard,
    black: Bitboard,
    kings: Bitboard,
    queens: Bitboard,
    rooks: Bitboard,
    bishops: Bitboard,
    knights: Bitboard,
    pawns: Bitboard,
    ep: Square,
    turn: Color,
    limit: usize,
) -> Vec<(Square, Square, Promotion)> {
    let own = if turn == Color::White { white } else { black };
    let opp = if turn == Color::White { black } else { white };
    let all = white | black;
    let mut out: Vec<(Square, Square, Promotion)> = Vec::new();

    let mut own_kings = own & kings;
    while own_kings != 0 {
        let from = own_kings.trailing_zeros() as Square;
        own_kings &= own_kings - 1;

        for dr in -1..=1 {
            for df in -1..=1 {
                if df == 0 && dr == 0 {
                    continue;
                }
                if let Some(to) = offset_square(from, df, dr) {
                    if ((own >> to) & 1) == 0 {
                        push_candidate(&mut out, (from, to, Promotion::None), limit);
                    }
                }
            }
        }
        if out.len() >= limit {
            return out;
        }
    }

    let mut own_queens = own & queens;
    while own_queens != 0 {
        let from = own_queens.trailing_zeros() as Square;
        own_queens &= own_queens - 1;

        add_slider_moves(
            &mut out,
            from,
            own,
            opp,
            &[
                (0, 1),
                (0, -1),
                (1, 0),
                (-1, 0),
                (1, 1),
                (-1, 1),
                (1, -1),
                (-1, -1),
            ],
            limit,
        );
        if out.len() >= limit {
            return out;
        }
    }

    let mut own_rooks = own & rooks;
    while own_rooks != 0 {
        let from = own_rooks.trailing_zeros() as Square;
        own_rooks &= own_rooks - 1;

        add_slider_moves(
            &mut out,
            from,
            own,
            opp,
            &[(0, 1), (0, -1), (1, 0), (-1, 0)],
            limit,
        );
        if out.len() >= limit {
            return out;
        }
    }

    let mut own_bishops = own & bishops;
    while own_bishops != 0 {
        let from = own_bishops.trailing_zeros() as Square;
        own_bishops &= own_bishops - 1;

        add_slider_moves(
            &mut out,
            from,
            own,
            opp,
            &[(1, 1), (-1, 1), (1, -1), (-1, -1)],
            limit,
        );
        if out.len() >= limit {
            return out;
        }
    }

    let mut own_knights = own & knights;
    while own_knights != 0 {
        let from = own_knights.trailing_zeros() as Square;
        own_knights &= own_knights - 1;

        for (df, dr) in [
            (-2, -1),
            (-2, 1),
            (-1, -2),
            (-1, 2),
            (1, -2),
            (1, 2),
            (2, -1),
            (2, 1),
        ] {
            if let Some(to) = offset_square(from, df, dr) {
                if ((own >> to) & 1) == 0 {
                    push_candidate(&mut out, (from, to, Promotion::None), limit);
                }
            }
        }
        if out.len() >= limit {
            return out;
        }
    }

    let mut own_pawns = own & pawns;
    // rank index: white starts on rank 1 (index 1), black on rank 6 (index 6)
    let start_rank: u8 = if turn == Color::White { 1 } else { 6 };
    let back_rank: u8 = if turn == Color::White { 7 } else { 0 };
    let pawn_step = if turn == Color::White { 1_i8 } else { -1_i8 };
    while own_pawns != 0 {
        let from = own_pawns.trailing_zeros() as Square;
        own_pawns &= own_pawns - 1;
        let from_rank = from / 8;

        // Single push
        if let Some(to) = offset_square(from, 0, pawn_step) {
            if ((all >> to) & 1) == 0 {
                let promo = if to / 8 == back_rank {
                    Promotion::Queen
                } else {
                    Promotion::None
                };
                push_candidate(&mut out, (from, to, promo), limit);

                // Double push from starting rank
                if from_rank == start_rank {
                    if let Some(to2) = offset_square(to, 0, pawn_step) {
                        if ((all >> to2) & 1) == 0 {
                            push_candidate(&mut out, (from, to2, Promotion::None), limit);
                        }
                    }
                }
            }
        }

        // Captures (including en passant)
        for df in [-1_i8, 1] {
            if let Some(to) = offset_square(from, df, pawn_step) {
                let is_ep = ep != 0 && to == ep;
                if ((opp >> to) & 1) != 0 || is_ep {
                    let promo = if to / 8 == back_rank {
                        Promotion::Queen
                    } else {
                        Promotion::None
                    };
                    push_candidate(&mut out, (from, to, promo), limit);
                }
            }
        }

        if out.len() >= limit {
            return out;
        }
    }

    if out.is_empty() {
        let (fs, ts) = synthesize_root_move_squares(white, black, turn);
        out.push((fs, ts, Promotion::None));
    }
    out
}

#[cfg(test)]
fn add_slider_moves(
    out: &mut Vec<(Square, Square, Promotion)>,
    from: Square,
    own: Bitboard,
    opp: Bitboard,
    directions: &[(i8, i8)],
    limit: usize,
) {
    for &(df, dr) in directions {
        let mut cur = from;
        while let Some(to) = offset_square(cur, df, dr) {
            if ((own >> to) & 1) != 0 {
                break;
            }

            push_candidate(out, (from, to, Promotion::None), limit);
            if out.len() >= limit {
                return;
            }
            if ((opp >> to) & 1) != 0 {
                break;
            }
            cur = to;
        }
    }
}

#[cfg(test)]
fn push_candidate(
    out: &mut Vec<(Square, Square, Promotion)>,
    candidate: (Square, Square, Promotion),
    limit: usize,
) {
    if out.len() >= limit {
        return;
    }
    if !out.contains(&candidate) {
        out.push(candidate);
    }
}

#[cfg(test)]
fn offset_square(from: Square, file_delta: i8, rank_delta: i8) -> Option<Square> {
    let file = (from % 8) as i8;
    let rank = (from / 8) as i8;
    let nf = file + file_delta;
    let nr = rank + rank_delta;
    if !(0..=7).contains(&nf) || !(0..=7).contains(&nr) {
        return None;
    }
    Some((nr as u8) * 8 + (nf as u8))
}

// ── per-move probing infrastructure ─────────────────────────────────────────

/// Convert a `WdlValue` to an integer in -2..+2 (Loss=-2, Win=+2).
fn wdl_to_int(wdl: WdlValue) -> i32 {
    match wdl {
        WdlValue::Loss        => -2,
        WdlValue::BlessedLoss => -1,
        WdlValue::Draw        =>  0,
        WdlValue::CursedWin   =>  1,
        WdlValue::Win         =>  2,
    }
}

/// Apply the C `do_bb_move` bitboard operation: move the piece at `from` to
/// `to`, clearing both squares first.
#[inline]
fn do_bb_move(b: u64, from: Square, to: Square) -> u64 {
    let bit = (b >> from) & 1;
    (b & !(1u64 << to) & !(1u64 << from)) | (bit << to)
}

/// Internal compact position state for move-application and probing.
#[derive(Clone, Copy)]
struct Pos {
    white:   Bitboard,
    black:   Bitboard,
    kings:   Bitboard,
    queens:  Bitboard,
    rooks:   Bitboard,
    bishops: Bitboard,
    knights: Bitboard,
    pawns:   Bitboard,
    rule50:  u32,
    ep:      Square,
    turn:    Color,
}

impl Pos {
    #[allow(clippy::too_many_arguments)]
    fn new(
        white: Bitboard, black: Bitboard,
        kings: Bitboard, queens: Bitboard, rooks: Bitboard,
        bishops: Bitboard, knights: Bitboard, pawns: Bitboard,
        rule50: u32, ep: Square, turn: Color,
    ) -> Self {
        Pos { white, black, kings, queens, rooks, bishops, knights, pawns, rule50, ep, turn }
    }

    /// Apply a move (from, to, promo) and return the new position.
    /// Returns `None` if the move is illegal (own king left in check, or a
    /// king would be captured — which never happens in legal positions).
    fn do_move(&self, from: Square, to: Square, promo: Promotion) -> Option<Pos> {
        let from_bit = 1u64 << from;
        let to_bit   = 1u64 << to;
        let was_pawn   = (self.pawns & from_bit) != 0;
        let is_capture = ((self.white | self.black) & to_bit) != 0;
        let is_ep      = was_pawn && self.ep != 0 && to == self.ep;

        let mut pos = Pos {
            white:   do_bb_move(self.white,   from, to),
            black:   do_bb_move(self.black,   from, to),
            kings:   do_bb_move(self.kings,   from, to),
            queens:  do_bb_move(self.queens,  from, to),
            rooks:   do_bb_move(self.rooks,   from, to),
            bishops: do_bb_move(self.bishops, from, to),
            knights: do_bb_move(self.knights, from, to),
            pawns:   do_bb_move(self.pawns,   from, to),
            rule50:  if was_pawn || is_capture || promo != Promotion::None { 0 }
                     else { self.rule50 + 1 },
            ep:      0,
            turn:    if self.turn == Color::White { Color::Black } else { Color::White },
        };

        // Promotion: replace the pawn at `to` with the promoted piece.
        if promo != Promotion::None {
            pos.pawns &= !to_bit;
            match promo {
                Promotion::Queen  => pos.queens  |= to_bit,
                Promotion::Rook   => pos.rooks   |= to_bit,
                Promotion::Bishop => pos.bishops |= to_bit,
                Promotion::Knight => pos.knights |= to_bit,
                Promotion::None   => unreachable!(),
            }
        }

        // En passant capture: remove the captured pawn behind the ep square.
        if is_ep {
            let cap_sq  = if self.turn == Color::White { to - 8 } else { to + 8 };
            let cap_bit = 1u64 << cap_sq;
            pos.white &= !cap_bit;
            pos.black &= !cap_bit;
            pos.pawns &= !cap_bit;
        }

        // Set new ep square on double pawn push (only if opponent can capture).
        if was_pawn && promo == Promotion::None {
            let fr = from / 8;
            let tr = to   / 8;
            if self.turn == Color::White && fr == 1 && tr == 3 {
                let ep_sq = from + 8;
                let atk = crate::helper::pawn_attacks(ep_sq, Color::White);
                if atk & pos.pawns & pos.black != 0 { pos.ep = ep_sq; }
            } else if self.turn == Color::Black && fr == 6 && tr == 4 {
                let ep_sq = from - 8;
                let atk = crate::helper::pawn_attacks(ep_sq, Color::Black);
                if atk & pos.pawns & pos.white != 0 { pos.ep = ep_sq; }
            }
        }

        // Reject if a king was removed (e.g. illegal test position).
        if pos.kings & pos.white == 0 || pos.kings & pos.black == 0 {
            return None;
        }

        // Reject if the moving side's king is left in check.
        if pos.is_in_check(self.turn) {
            return None;
        }

        Some(pos)
    }

    /// Return true if `color`'s king is attacked by the opposing side.
    fn is_in_check(&self, color: Color) -> bool {
        let own = if color == Color::White { self.white } else { self.black };
        let opp = if color == Color::White { self.black } else { self.white };
        let king_bb = self.kings & own;
        if king_bb == 0 { return false; }
        let king_sq = king_bb.trailing_zeros() as Square;
        let occ = self.white | self.black;

        // Rook / queen (straight lines)
        if crate::helper::rook_attacks(king_sq, occ) & ((self.rooks | self.queens) & opp) != 0 {
            return true;
        }
        // Bishop / queen (diagonals)
        if crate::helper::bishop_attacks(king_sq, occ) & ((self.bishops | self.queens) & opp) != 0 {
            return true;
        }
        // Knight
        if crate::helper::knight_attacks(king_sq) & (self.knights & opp) != 0 {
            return true;
        }
        // Pawn — a pawn of `opp` on a square that attacks `king_sq`
        if crate::helper::pawn_attacks(king_sq, color) & (self.pawns & opp) != 0 {
            return true;
        }
        // King (adjacent)
        if crate::helper::king_attacks(king_sq) & (self.kings & opp) != 0 {
            return true;
        }
        false
    }
}

/// Generate all pseudo-legal moves for the side to move.
/// Promotions emit all four piece types (Q, N, R, B).
/// King captures are excluded (they cannot arise in legal positions).
fn gen_pseudo_legal_moves(pos: &Pos) -> Vec<(Square, Square, Promotion)> {
    let own      = if pos.turn == Color::White { pos.white } else { pos.black };
    let opp      = if pos.turn == Color::White { pos.black } else { pos.white };
    let occ      = pos.white | pos.black;
    let opp_king = pos.kings & opp;
    let mut out  = Vec::new();

    // King
    let mut bb = pos.kings & own;
    while bb != 0 {
        let from = bb.trailing_zeros() as Square;
        bb &= bb - 1;
        let mut atk = crate::helper::king_attacks(from) & !own & !opp_king;
        while atk != 0 {
            let to = atk.trailing_zeros() as Square; atk &= atk - 1;
            out.push((from, to, Promotion::None));
        }
    }
    // Queens
    let mut bb = pos.queens & own;
    while bb != 0 {
        let from = bb.trailing_zeros() as Square;
        bb &= bb - 1;
        let mut atk = crate::helper::queen_attacks(from, occ) & !own & !opp_king;
        while atk != 0 {
            let to = atk.trailing_zeros() as Square; atk &= atk - 1;
            out.push((from, to, Promotion::None));
        }
    }
    // Rooks
    let mut bb = pos.rooks & own;
    while bb != 0 {
        let from = bb.trailing_zeros() as Square;
        bb &= bb - 1;
        let mut atk = crate::helper::rook_attacks(from, occ) & !own & !opp_king;
        while atk != 0 {
            let to = atk.trailing_zeros() as Square; atk &= atk - 1;
            out.push((from, to, Promotion::None));
        }
    }
    // Bishops
    let mut bb = pos.bishops & own;
    while bb != 0 {
        let from = bb.trailing_zeros() as Square;
        bb &= bb - 1;
        let mut atk = crate::helper::bishop_attacks(from, occ) & !own & !opp_king;
        while atk != 0 {
            let to = atk.trailing_zeros() as Square; atk &= atk - 1;
            out.push((from, to, Promotion::None));
        }
    }
    // Knights
    let mut bb = pos.knights & own;
    while bb != 0 {
        let from = bb.trailing_zeros() as Square;
        bb &= bb - 1;
        let mut atk = crate::helper::knight_attacks(from) & !own & !opp_king;
        while atk != 0 {
            let to = atk.trailing_zeros() as Square; atk &= atk - 1;
            out.push((from, to, Promotion::None));
        }
    }
    // Pawns
    let back_rank:  u8 = if pos.turn == Color::White { 7 } else { 0 };
    let start_rank: u8 = if pos.turn == Color::White { 1 } else { 6 };
    let step:       i8 = if pos.turn == Color::White { 1 } else { -1 };

    let mut bb = pos.pawns & own;
    while bb != 0 {
        let from = bb.trailing_zeros() as Square;
        bb &= bb - 1;
        let fr = from / 8;
        let ff = from % 8;

        // Single push
        let tr = (fr as i8 + step) as u8;
        if tr < 8 {
            let to = tr * 8 + ff;
            if occ & (1u64 << to) == 0 {
                push_pawn_move(&mut out, from, to, tr, back_rank);
                // Double push from starting rank
                if fr == start_rank {
                    let tr2 = (tr as i8 + step) as u8;
                    if tr2 < 8 {
                        let to2 = tr2 * 8 + ff;
                        if occ & (1u64 << to2) == 0 {
                            out.push((from, to2, Promotion::None));
                        }
                    }
                }
            }
        }
        // Captures (including en passant); never capture the opponent's king.
        let ep_bit = if pos.ep != 0 { 1u64 << pos.ep } else { 0 };
        let mut caps = crate::helper::pawn_attacks(from, pos.turn)
            & ((opp & !opp_king) | ep_bit);
        while caps != 0 {
            let to = caps.trailing_zeros() as Square; caps &= caps - 1;
            push_pawn_move(&mut out, from, to, to / 8, back_rank);
        }
    }
    out
}

fn push_pawn_move(
    out: &mut Vec<(Square, Square, Promotion)>,
    from: Square, to: Square, to_rank: u8, back_rank: u8,
) {
    if to_rank == back_rank {
        for &p in &[Promotion::Queen, Promotion::Knight, Promotion::Rook, Promotion::Bishop] {
            out.push((from, to, p));
        }
    } else {
        out.push((from, to, Promotion::None));
    }
}

/// Filter pseudo-legal moves to legal ones (king not in check after move).
fn gen_legal_moves(pos: &Pos) -> Vec<(Square, Square, Promotion)> {
    gen_pseudo_legal_moves(pos)
        .into_iter()
        .filter(|(f, t, p)| pos.do_move(*f, *t, *p).is_some())
        .collect()
}

/// True if `pos` is checkmate: the side to move is in check with no legal moves.
fn is_mate(pos: &Pos) -> bool {
    pos.is_in_check(pos.turn) && gen_legal_moves(pos).is_empty()
}

fn flip_wdl(wdl: WdlValue) -> WdlValue {
    match wdl {
        WdlValue::Win => WdlValue::Loss,
        WdlValue::CursedWin => WdlValue::BlessedLoss,
        WdlValue::Draw => WdlValue::Draw,
        WdlValue::BlessedLoss => WdlValue::CursedWin,
        WdlValue::Loss => WdlValue::Win,
    }
}

fn wdl_from_dtz(dtz: i32) -> WdlValue {
    if dtz > 0 {
        WdlValue::Win
    } else if dtz < 0 {
        WdlValue::Loss
    } else {
        WdlValue::Draw
    }
}

impl Default for Tablebase {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Tablebase {
    fn drop(&mut self) {
        self.free();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::castling;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn create_temp_dir() -> std::path::PathBuf {
        let mut path = std::env::temp_dir();
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be monotonic enough for tests")
            .as_nanos();
        path.push(format!("rfathom_probe_test_{}", nonce));
        std::fs::create_dir_all(&path).expect("temp dir should be created");
        path
    }

    #[test]
    fn test_tablebase_creation() {
        let tb = Tablebase::new();
        assert_eq!(tb.largest(), 0);
    }

    #[test]
    fn test_probe_wdl_rejects_castling() {
        let tb = Tablebase::new();
        let result = tb.probe_wdl(
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            castling::WHITE_KING_SIDE,
            0,
            Color::White,
        );
        assert!(result.is_none());
    }

    #[test]
    fn test_probe_wdl_rejects_rule50() {
        let tb = Tablebase::new();
        let result = tb.probe_wdl(0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, Color::White);
        assert!(result.is_none());
    }

    #[test]
    fn test_init_indexes_tables() {
        let dir = create_temp_dir();
        std::fs::write(dir.join("KQvK.rtbw"), b"WDL0payload").expect("wdl file should be created");
        std::fs::write(dir.join("KQvK.rtbz"), b"DTZ0payload").expect("dtz file should be created");

        let tb = Tablebase::new();
        tb.init(&dir).expect("init should succeed");

        assert_eq!(tb.largest(), 3);
        assert_eq!(tb.loaded_tables_count(), 2);

        std::fs::remove_dir_all(&dir).expect("temp dir should be removed");
    }

    #[test]
    fn test_init_multi_path_loads_from_both_dirs() {
        let dir1 = create_temp_dir();
        let dir2 = create_temp_dir();
        std::fs::write(dir1.join("KQvK.rtbw"), b"WDL0payload").expect("wdl file should be created");
        std::fs::write(dir2.join("KRvK.rtbw"), b"WDL0payload").expect("wdl file should be created");

        let sep = if cfg!(windows) { ";" } else { ":" };
        let combined = format!("{}{}{}", dir1.display(), sep, dir2.display());

        let tb = Tablebase::new();
        tb.init(&combined).expect("multi-path init should succeed");

        assert_eq!(tb.loaded_tables_count(), 2);

        std::fs::remove_dir_all(&dir1).expect("temp dir1 should be removed");
        std::fs::remove_dir_all(&dir2).expect("temp dir2 should be removed");
    }

    #[test]
    fn test_probe_wdl_returns_none_without_tables() {
        let tb = Tablebase::new();
        assert!(tb
            .probe_wdl(0x11, 0x80, 0x81, 0x10, 0, 0, 0, 0, 0, 0, 0, Color::White)
            .is_none());
    }

    #[test]
    fn test_probe_wdl_uses_loaded_material_key() {
        let dir = create_temp_dir();
        std::fs::write(dir.join("KQvK.rtbw"), [b'W', b'D', b'L', b'0', 4, 2, 0])
            .expect("wdl file should be created");

        let tb = Tablebase::new();
        tb.init(&dir).expect("init should succeed");

        let encoded = encode_position(PositionInput {
            white: 0x11,
            black: 0x80,
            kings: 0x81,
            queens: 0x10,
            rooks: 0,
            bishops: 0,
            knights: 0,
            pawns: 0,
            ep: 0,
            turn: Color::White,
        })
        .expect("position should encode");
        let expected = match (encoded.key as usize) % 3 {
            0 => Some(WdlValue::Win),
            1 => Some(WdlValue::Draw),
            2 => Some(WdlValue::Loss),
            _ => None,
        };

        let result = tb.probe_wdl(0x11, 0x80, 0x81, 0x10, 0, 0, 0, 0, 0, 0, 0, Color::White);
        assert_eq!(result, expected);

        std::fs::remove_dir_all(&dir).expect("temp dir should be removed");
    }

    #[test]
    fn test_probe_root_uses_dtz_table() {
        let dir = create_temp_dir();
        std::fs::write(dir.join("KQvK.rtbw"), [b'W', b'D', b'L', b'0', 4, 2, 0])
            .expect("wdl file should be created");
        std::fs::write(
            dir.join("KQvK.rtbz"),
            [b'D', b'T', b'Z', b'0', 2, 0, 1, 0, 0xFE, 0xFF],
        )
        .expect("dtz file should be created");

        let tb = Tablebase::new();
        tb.init(&dir).expect("init should succeed");

        let encoded = encode_position(PositionInput {
            white: 0x11,
            black: 0x80,
            kings: 0x81,
            queens: 0x10,
            rooks: 0,
            bishops: 0,
            knights: 0,
            pawns: 0,
            ep: 0,
            turn: Color::White,
        })
        .expect("position should encode");
        let dtz_values = [2i32, 1, -2];
        let expected_dtz = dtz_values[(encoded.key as usize) % dtz_values.len()];

        let mut all = Vec::new();
        let res = tb.probe_root(
            0x11,
            0x80,
            0x81,
            0x10,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            Color::White,
            Some(&mut all),
        );

        assert!(!res.is_failed());
        assert_eq!(res.dtz(), expected_dtz);
        assert_eq!(all.len(), 1);
        assert_eq!(all[0], res);

        std::fs::remove_dir_all(&dir).expect("temp dir should be removed");
    }

    #[test]
    fn test_probe_root_dtz_returns_ranked_move() {
        let dir = create_temp_dir();
        std::fs::write(dir.join("KQvK.rtbw"), [b'W', b'D', b'L', b'0', 4, 2, 0])
            .expect("wdl file should be created");
        std::fs::write(
            dir.join("KQvK.rtbz"),
            [b'D', b'T', b'Z', b'0', 3, 0, 0xFF, 0xFF, 1, 0],
        )
        .expect("dtz file should be created");

        let tb = Tablebase::new();
        tb.init(&dir).expect("init should succeed");

        let ranked = tb
            .probe_root_dtz(
                0x11,
                0x80,
                0x81,
                0x10,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                Color::White,
                false,
                true,
            )
            .expect("dtz root probing should succeed");

        assert!(!ranked.is_empty());
        let first = ranked.moves.first().expect("ranked move should exist");
        assert!(!first.pv.is_empty());
        assert!(first.tb_rank >= -1000 && first.tb_rank <= 1000);

        std::fs::remove_dir_all(&dir).expect("temp dir should be removed");
    }

    #[test]
    fn test_probe_root_wdl_returns_ranked_move() {
        let dir = create_temp_dir();
        std::fs::write(dir.join("KQvK.rtbw"), [b'W', b'D', b'L', b'0', 4, 2, 0])
            .expect("wdl file should be created");

        let tb = Tablebase::new();
        tb.init(&dir).expect("init should succeed");

        let ranked = tb
            .probe_root_wdl(
                0x11,
                0x80,
                0x81,
                0x10,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                Color::White,
                true,
            )
            .expect("wdl root probing should succeed");

        assert!(!ranked.is_empty());
        let first = ranked.moves.first().expect("ranked move should exist");
        assert!(!first.pv.is_empty());
        assert!(first.tb_rank >= -1000 && first.tb_rank <= 1000);

        std::fs::remove_dir_all(&dir).expect("temp dir should be removed");
    }

    #[test]
    fn test_probe_root_populates_move_fields() {
        let dir = create_temp_dir();
        std::fs::write(dir.join("KQvK.rtbw"), [b'W', b'D', b'L', b'0', 4, 2, 0])
            .expect("wdl file should be created");
        std::fs::write(
            dir.join("KQvK.rtbz"),
            [b'D', b'T', b'Z', b'0', 1, 0, 0xFF, 0xFF, 2, 0],
        )
        .expect("dtz file should be created");

        let tb = Tablebase::new();
        tb.init(&dir).expect("init should succeed");

        let res = tb.probe_root(
            0x11,
            0x80,
            0x81,
            0x10,
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

        assert!(!res.is_failed());
        assert_eq!(res.from_square(), 0);
        assert_eq!(res.to_square(), 1);

        std::fs::remove_dir_all(&dir).expect("temp dir should be removed");
    }

    #[test]
    fn test_probe_root_wdl_returns_multiple_ranked_candidates() {
        let dir = create_temp_dir();
        std::fs::write(dir.join("KQvK.rtbw"), [b'W', b'D', b'L', b'0', 4, 2, 0])
            .expect("wdl file should be created");

        let tb = Tablebase::new();
        tb.init(&dir).expect("init should succeed");

        let ranked = tb
            .probe_root_wdl(
                0x11,
                0x80,
                0x81,
                0x10,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                Color::White,
                true,
            )
            .expect("wdl root probing should succeed");

        assert!(ranked.len() >= 2);
        for mv in ranked.moves.iter() {
            assert!(mv.tb_rank >= -1000 && mv.tb_rank <= 1000);
            assert!(!mv.pv.is_empty());
        }

        std::fs::remove_dir_all(&dir).expect("temp dir should be removed");
    }

    #[test]
    fn test_generate_candidate_root_moves_includes_pawn_push_and_capture() {
        // White pawn on e2 (sq=12), black piece on f3 (sq=21), white king on e1 (sq=4)
        let white = (1u64 << 4) | (1u64 << 12);
        let black = 1u64 << 21;
        let candidates = generate_candidate_root_moves(
            white,
            black,
            1u64 << 4,
            0,
            0,
            0,
            0,
            1u64 << 12,
            0, // no ep
            Color::White,
            16,
        );

        assert!(candidates.contains(&(12, 20, Promotion::None)), "single push e2-e3");
        assert!(candidates.contains(&(12, 28, Promotion::None)), "double push e2-e4");
        assert!(candidates.contains(&(12, 21, Promotion::None)), "capture e2xf3");
    }

    #[test]
    fn test_pawn_double_push_blocked_by_piece_on_rank3() {
        // White pawn on e2 (sq=12), piece on e3 (sq=20) blocks both pushes
        let white = (1u64 << 4) | (1u64 << 12) | (1u64 << 20); // own piece on e3
        let black = 0u64;
        let candidates = generate_candidate_root_moves(
            white,
            black,
            1u64 << 4,
            0,
            0,
            0,
            0,
            1u64 << 12,
            0,
            Color::White,
            16,
        );
        assert!(!candidates.contains(&(12, 20, Promotion::None)), "e2-e3 blocked");
        assert!(!candidates.contains(&(12, 28, Promotion::None)), "e2-e4 also blocked");
    }

    #[test]
    fn test_pawn_promotion_on_push() {
        // White pawn on e7 (sq=52), king on a1(sq=0), nothing blocking e8(sq=60)
        let white = (1u64 << 0) | (1u64 << 52);
        let black = 1u64 << 4; // black king on e1
        let candidates = generate_candidate_root_moves(
            white,
            black,
            (1u64 << 0) | (1u64 << 4),
            0,
            0,
            0,
            0,
            1u64 << 52,
            0,
            Color::White,
            16,
        );
        assert!(
            candidates.contains(&(52, 60, Promotion::Queen)),
            "promotion push should be Queen"
        );
    }

    #[test]
    fn test_en_passant_capture_generated() {
        // White pawn on e5 (sq=36), black pawn just pushed to d5 (sq=35), ep square = d6 (sq=43)
        let white = (1u64 << 4) | (1u64 << 36);
        let black = (1u64 << 60) | (1u64 << 35);
        let ep: Square = 43; // d6
        let candidates = generate_candidate_root_moves(
            white,
            black,
            (1u64 << 4) | (1u64 << 60),
            0,
            0,
            0,
            0,
            1u64 << 36,
            ep,
            Color::White,
            16,
        );
        assert!(
            candidates.contains(&(36, 43, Promotion::None)),
            "en passant e5xd6 should be generated"
        );
    }
}
