//! # RFathom - Rust Syzygy Tablebase Probing Library
//!
//! RFathom is a Rust port of the Fathom tablebase probing library.
//! It provides access to Syzygy endgame tablebases for chess engines.
//!
//! ## Features
//!
//! - Win-Draw-Loss (WDL) probing
//! - Distance-To-Zero (DTZ) probing
//! - Root move analysis with PV generation
//! - Thread-safe WDL probing
//! - Optional helper API for bitboard operations
//!
//! ## Example
//!
//! ```no_run
//! use rfathom::Tablebase;
//!
//! let tb = Tablebase::new();
//! if tb.init("path/to/syzygy").is_ok() {
//!     // Probe tablebase
//! }
//! ```

pub mod bitboard;
pub mod constants;
mod encoding;
mod loader;
pub mod probe;
pub mod types;

pub use constants::*;
pub use probe::Tablebase;
pub use types::*;

pub(crate) mod syzygy;

#[cfg(feature = "helper-api")]
pub mod helper;

#[cfg(feature = "helper-api")]
pub use helper::*;
