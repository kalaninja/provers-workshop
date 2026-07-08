//! Codama IDL-only scaffolding.
//!
//! Declarations that exist solely so codama emits a complete, byte-accurate
//! IDL. None of these types are instantiated at runtime — codama reads them
//! from the source AST during `just solana gen-idl`:
//!
//! - `types` — mirror structs for generic wire types codama can't reflect over
//!   (e.g. `EventName` behind `BoundedString<64>`), linked by name.
//! - `pdas` — `CodamaPda` definitions for every program PDA; their seeds mirror
//!   the `CanonicalPda` impls in [`crate::state`].

mod pdas;
mod types;
