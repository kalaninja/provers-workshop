//! IDL-only mirror structs for the named wire types codama links by name.
//!
//! codama cannot reflect over the generic type behind the
//! [`crate::state::event::EventName`] alias (`BoundedString<64>`), so the
//! mirror below supplies the byte layout codama links to by the (camelCased)
//! type name.
//!
//! A mirror is used rather than an inline
//! `struct(field("len", ...), field("bytes", ...))` override:
//! `@codama/renderers-rust` mis-renders inline anonymous struct types on
//! *instruction arguments* (it links the field to the parent
//! `*InstructionData` struct), so `event_create` could not be built correctly
//! through the generated client. A named defined type sidesteps the renderer
//! bug. The mirror is never instantiated at runtime — only codama's AST
//! walker reads it.

// codama reads these from the source AST, not the compiled crate, so they have
// no Rust callers and need no visibility. Silence the resulting dead-code lint
// rather than leak IDL-only scaffolding into the crate's public API.
#![allow(dead_code)]

use ticketing_macros::CodamaType;

/// Fixed-capacity event name: a `u16` length prefix followed by 64 bytes of
/// storage, of which the first `len` are significant.
///
/// IDL view of [`crate::state::event::EventName`] (= `BoundedString<64>`),
/// whose generic layout codama cannot reflect over. Shares its identifier with
/// the alias so codama links an `EventName` argument to this defined type.
/// Only the first doc paragraph is client-facing (the `docs` fixup copies it
/// into the IDL), so it stays plain prose with no rustdoc paths.
#[derive(Clone, Copy, CodamaType)]
struct EventName {
    /// Number of significant bytes in `bytes`.
    len: u16,
    /// Fixed-capacity byte storage (not validated as UTF-8).
    #[codama(type = fixed_size(bytes, 64))]
    bytes: [u8; 64],
}
