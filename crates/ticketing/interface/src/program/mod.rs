//! Program-side helpers for Pinocchio instruction handlers.

/// Account-validation and PDA-creation helpers used by instruction handlers.
pub mod accounts;
/// Instruction-related abstractions: the wire-format trait for
/// fixed-layout instruction-argument payloads.
pub mod instruction;
/// Account-state abstractions: the wire-format trait for fixed-layout
/// account payloads.
pub mod state;
