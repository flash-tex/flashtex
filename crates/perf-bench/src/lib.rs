//! The harness's reusable parts, so a second measuring binary (the FT-070
//! memory profile) loads the same corpus through the same font gate as
//! `main.rs` instead of a near-copy that could drift from it.
//!
//! `main.rs` keeps its own `mod` declarations: the timing binary is
//! deliberately not refactored onto this library, because every committed
//! baseline was recorded by it exactly as it stands.

pub mod corpus;
pub mod fontgate;
pub mod sys;
