//! Position canonicalization and deterministic encoding.

use crate::types::{Bitboard, Color, Square};

const MAX_TB_PIECES: u32 = 7;

#[derive(Debug, Clone, Copy)]
pub(crate) struct PositionInput {
    pub(crate) white: Bitboard,
    pub(crate) black: Bitboard,
    pub(crate) kings: Bitboard,
    pub(crate) queens: Bitboard,
    pub(crate) rooks: Bitboard,
    pub(crate) bishops: Bitboard,
    pub(crate) knights: Bitboard,
    pub(crate) pawns: Bitboard,
    pub(crate) ep: Square,
    pub(crate) turn: Color,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EncodedPosition {
    pub(crate) material_key: String,
    pub(crate) key: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EncodeError {
    SideOverlap,
    KingsNotTwo,
    InvalidPieceUnion,
    TooManyPieces,
}

pub(crate) fn encode_position(input: PositionInput) -> Result<EncodedPosition, EncodeError> {
    validate_position(input)?;

    let material_key = material_key(input);
    let key = hash_position(input);

    Ok(EncodedPosition { material_key, key })
}

fn validate_position(input: PositionInput) -> Result<(), EncodeError> {
    if (input.white & input.black) != 0 {
        return Err(EncodeError::SideOverlap);
    }

    if input.kings.count_ones() != 2 {
        return Err(EncodeError::KingsNotTwo);
    }

    let pieces =
        input.kings | input.queens | input.rooks | input.bishops | input.knights | input.pawns;

    if pieces != (input.white | input.black) {
        return Err(EncodeError::InvalidPieceUnion);
    }

    if pieces.count_ones() > MAX_TB_PIECES {
        return Err(EncodeError::TooManyPieces);
    }

    Ok(())
}

fn material_key(input: PositionInput) -> String {
    let w = side_material(input, input.white);
    let b = side_material(input, input.black);
    format!("{}v{}", w, b)
}

fn side_material(input: PositionInput, side: Bitboard) -> String {
    let mut s = String::from("K");
    append_repeated(&mut s, 'Q', (input.queens & side).count_ones());
    append_repeated(&mut s, 'R', (input.rooks & side).count_ones());
    append_repeated(&mut s, 'B', (input.bishops & side).count_ones());
    append_repeated(&mut s, 'N', (input.knights & side).count_ones());
    append_repeated(&mut s, 'P', (input.pawns & side).count_ones());
    s
}

fn append_repeated(out: &mut String, ch: char, count: u32) {
    for _ in 0..count {
        out.push(ch);
    }
}

fn hash_position(input: PositionInput) -> u64 {
    // Stable FNV-1a style hash over the raw position representation.
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    fn mix(h: &mut u64, v: u64) {
        *h ^= v;
        *h = h.wrapping_mul(0x0000_0001_0000_01b3);
    }

    mix(&mut h, input.white);
    mix(&mut h, input.black);
    mix(&mut h, input.kings);
    mix(&mut h, input.queens);
    mix(&mut h, input.rooks);
    mix(&mut h, input.bishops);
    mix(&mut h, input.knights);
    mix(&mut h, input.pawns);
    mix(&mut h, input.ep as u64);
    mix(&mut h, input.turn as u64);

    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_encoding() {
        let pos = PositionInput {
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
        };
        let a = encode_position(pos).expect("position should encode");
        let b = encode_position(pos).expect("position should encode");
        assert_eq!(a, b);
        assert_eq!(a.material_key, "KQvK");
    }

    #[test]
    fn rejects_overlapping_sides() {
        let pos = PositionInput {
            white: 0x1,
            black: 0x1,
            kings: 0x1,
            queens: 0,
            rooks: 0,
            bishops: 0,
            knights: 0,
            pawns: 0,
            ep: 0,
            turn: Color::White,
        };
        let err = encode_position(pos).expect_err("position should be invalid");
        assert_eq!(err, EncodeError::SideOverlap);
    }

    #[test]
    fn rejects_missing_or_extra_kings() {
        let pos = PositionInput {
            white: 0x11,
            black: 0x80,
            kings: 0x01,
            queens: 0x10,
            rooks: 0,
            bishops: 0,
            knights: 0,
            pawns: 0,
            ep: 0,
            turn: Color::White,
        };
        let err = encode_position(pos).expect_err("position should be invalid");
        assert_eq!(err, EncodeError::KingsNotTwo);
    }
}
