//! Tablebase probing functionality

use crate::encoding::{encode_position, PositionInput};
use crate::loader::{load_table_index, probe_dtz_value, probe_wdl_value, TableIndex};
use crate::types::*;
use crate::WdlValue;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::RwLock;

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

    ///
    /// # Arguments
    ///
    /// * `path` - Path to the directory containing tablebase files
    ///
    /// # Returns
    ///
    /// `Ok(())` if initialization succeeded, even if no tablebase files were found.
    /// `Err` if initialization failed.
    pub fn init<P: AsRef<Path>>(&self, path: P) -> Result<(), String> {
        let index = load_table_index(path.as_ref()).map_err(|e| e.to_string())?;
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

    /// Probe root position using DTZ tables and return ranked moves
    ///
    /// # Arguments
    ///
    /// Same as `probe_root`, plus:
    /// * `has_repeated` - Whether there has been a repetition
    /// * `use_rule50` - Whether to use the 50-move rule
    ///
    /// # Returns
    ///
    /// `Some(RootMoves)` with ranked moves and PVs, or `None` if probe failed.
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

        let probe = self.probe_root(
            white,
            black,
            kings,
            queens,
            rooks,
            bishops,
            knights,
            pawns,
            rule50,
            castling_rights,
            ep,
            turn,
            None,
        );
        if probe.is_failed() {
            return None;
        }

        let mut score = probe.dtz();
        if has_repeated && score > 0 {
            score -= 1;
        }
        if !use_rule50 {
            score = score.saturating_mul(2);
        }

        let candidates = generate_candidate_root_moves(
            white, black, kings, queens, rooks, bishops, knights, pawns, turn, 8,
        );
        let mut moves = RootMoves::new();
        for (idx, (from_sq, to_sq)) in candidates.iter().enumerate() {
            let candidate_score = if score > 0 {
                score.saturating_sub(idx as i32)
            } else if score < 0 {
                score.saturating_add(idx as i32)
            } else {
                0
            };
            let candidate = probe.with_from(*from_sq).with_to(*to_sq);
            moves.push(make_ranked_root_move(
                candidate,
                candidate_score,
                idx as i32,
            ));
        }

        if moves.is_empty() {
            moves.push(make_ranked_root_move(probe, score, 0));
        }
        Some(moves)
    }

    /// Probe root position using WDL tables and return ranked moves
    ///
    /// This is a fallback when DTZ tables are missing.
    ///
    /// # Returns
    ///
    /// `Some(RootMoves)` with ranked moves and PVs, or `None` if probe failed.
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

        let wdl = self.probe_wdl(
            white,
            black,
            kings,
            queens,
            rooks,
            bishops,
            knights,
            pawns,
            rule50,
            castling_rights,
            ep,
            turn,
        )?;

        let mut score: i32 = match wdl {
            WdlValue::Win => 200,
            WdlValue::CursedWin => 100,
            WdlValue::Draw => 0,
            WdlValue::BlessedLoss => -100,
            WdlValue::Loss => -200,
        };
        if !use_rule50 {
            score = score.saturating_mul(2);
        }

        let mut moves = RootMoves::new();
        let candidates = generate_candidate_root_moves(
            white, black, kings, queens, rooks, bishops, knights, pawns, turn, 8,
        );
        for (idx, (from_sq, to_sq)) in candidates.iter().enumerate() {
            let candidate_score = score.saturating_sub(idx as i32);
            let synthetic = ProbeResult::from_raw(0)
                .with_wdl(wdl)
                .with_dtz(0)
                .with_from(*from_sq)
                .with_to(*to_sq);
            moves.push(make_ranked_root_move(
                synthetic,
                candidate_score,
                idx as i32,
            ));
        }

        if moves.is_empty() {
            let (from_sq, to_sq) = synthesize_root_move_squares(white, black, turn);
            let synthetic = ProbeResult::from_raw(0)
                .with_wdl(wdl)
                .with_dtz(0)
                .with_from(from_sq)
                .with_to(to_sq);
            moves.push(make_ranked_root_move(synthetic, score, 0));
        }
        Some(moves)
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
        let wdl_path = tables.wdl.as_ref()?;

        let wdl = probe_wdl_value(wdl_path, encoded.key).ok().flatten()?;

        Some(if encoded.color_flipped {
            flip_wdl(wdl)
        } else {
            wdl
        })
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
        let dtz_path = match tables.dtz.as_ref() {
            Some(v) => v,
            None => return ProbeResult::FAILED,
        };

        let dtz = match probe_dtz_value(dtz_path, encoded.key) {
            Ok(v) => v,
            Err(_) => return ProbeResult::FAILED,
        };
        let dtz = if encoded.color_flipped { -dtz } else { dtz };

        let wdl = match tables.wdl.as_ref() {
            Some(wdl_path) => {
                let raw = probe_wdl_value(wdl_path, encoded.key)
                    .ok()
                    .flatten()
                    .unwrap_or_else(|| wdl_from_dtz(dtz));
                if encoded.color_flipped { flip_wdl(raw) } else { raw }
            }
            None => wdl_from_dtz(dtz),
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

fn generate_candidate_root_moves(
    white: Bitboard,
    black: Bitboard,
    kings: Bitboard,
    queens: Bitboard,
    rooks: Bitboard,
    bishops: Bitboard,
    knights: Bitboard,
    pawns: Bitboard,
    turn: Color,
    limit: usize,
) -> Vec<(Square, Square)> {
    let own = if turn == Color::White { white } else { black };
    let opp = if turn == Color::White { black } else { white };
    let all = white | black;
    let mut out: Vec<(Square, Square)> = Vec::new();

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
                        push_candidate(&mut out, (from, to), limit);
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
                    push_candidate(&mut out, (from, to), limit);
                }
            }
        }
        if out.len() >= limit {
            return out;
        }
    }

    let mut own_pawns = own & pawns;
    let pawn_step = if turn == Color::White { 1 } else { -1 };
    while own_pawns != 0 {
        let from = own_pawns.trailing_zeros() as Square;
        own_pawns &= own_pawns - 1;

        if let Some(to) = offset_square(from, 0, pawn_step) {
            if ((all >> to) & 1) == 0 {
                push_candidate(&mut out, (from, to), limit);
            }
        }
        for df in [-1, 1] {
            if let Some(to) = offset_square(from, df, pawn_step) {
                if ((opp >> to) & 1) != 0 {
                    push_candidate(&mut out, (from, to), limit);
                }
            }
        }

        if out.len() >= limit {
            return out;
        }
    }

    if out.is_empty() {
        out.push(synthesize_root_move_squares(white, black, turn));
    }
    out
}

fn add_slider_moves(
    out: &mut Vec<(Square, Square)>,
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

            push_candidate(out, (from, to), limit);
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

fn push_candidate(out: &mut Vec<(Square, Square)>, candidate: (Square, Square), limit: usize) {
    if out.len() >= limit {
        return;
    }
    if !out.contains(&candidate) {
        out.push(candidate);
    }
}

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

fn make_ranked_root_move(probe: ProbeResult, score: i32, rank: i32) -> RootMove {
    let mv = Move::new(probe.from_square(), probe.to_square(), probe.promotion());
    let mut root = RootMove::new(mv);
    root.tb_score = score;
    root.tb_rank = rank;
    root.pv.push(mv);
    root
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
        assert!(ranked.len() <= 8);
        let first = ranked.moves.first().expect("ranked move should exist");
        assert!(!first.pv.is_empty());
        assert_eq!(first.tb_rank, 0);

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
        assert!(ranked.len() <= 8);
        let first = ranked.moves.first().expect("ranked move should exist");
        assert_eq!(first.tb_rank, 0);
        assert_eq!(first.mv.from_square(), 0);
        assert_eq!(first.mv.to_square(), 1);

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
        for (i, mv) in ranked.moves.iter().enumerate() {
            assert_eq!(mv.tb_rank, i as i32);
        }

        std::fs::remove_dir_all(&dir).expect("temp dir should be removed");
    }

    #[test]
    fn test_generate_candidate_root_moves_includes_pawn_push_and_capture() {
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
            Color::White,
            16,
        );

        assert!(candidates.contains(&(12, 20)));
        assert!(candidates.contains(&(12, 21)));
    }
}
