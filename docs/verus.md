# Verus lane

| Field | Value |
| --- | --- |
| Lane | 3 |
| Status | Implemented |
| Pin | Verus `0.2026.07.12.0b42f4c`, Rust `1.96.0`, verusfmt `0.7.2` |
| Verification | Unbounded SMT proof with bundled Z3 |
| Artifacts | Incremental Cargo-Verus output only; no committed proof object |
| Last reviewed | 2026-07-19 |

[Verus](https://github.com/verus-lang/verus) verifies a Rust dialect with specifications and proofs
written in the language itself. The lane tests both proof expressiveness and whether Verus source can
remain a real SBF program rather than a detached proof crate.

## Integration

`ticketing-verus` preserves the standard program split:

- trusted plain-Rust instruction shells and dispatcher;
- decision cores inside `verus!` blocks with inline `requires` and `ensures`;
- domain predicates and typed mutators as Verus `spec fn` and executable functions.

The `verus!` proc-macro erases ghost items unless the Verus driver sets `verus_keep_ghost`. Ordinary
host and SBF builds therefore expand the same source to executable Rust without linking `vstd`. A
`verus` Cargo feature activates only the optional `vstd` dependency and foreign-type registrations
during verification.

Rustfmt cannot format inside `verus!` blocks, so the lane adds pinned verusfmt commands and a dedicated
pre-push format check.

## Verification coverage

Verus proves the three cores and their domain operations over the full integer domain. The contracts
cover the Creusot reference properties: invariant preservation, exact frames, exact clamp and payment,
legal transitions, exact errors, and on-error unchanged state.

The nonlinear `capacity * price <= u64::MAX` implication is unbounded. One explicit
`by (nonlinear_arith)` assertion proves multiplication monotonicity in `record_sale`; the rest of the
purchase composition follows from function contracts without additional proof blocks.

The current result is 11 verified functions and zero errors.

## Trusted surface

Verus verifies one crate, so foreign interface types must be registered before specifications can
inspect them. Transparent registrations mirror `Event`, `Ticket`, `EventStatus`, and `ProgramError`;
wire padding types are opaque.

The remaining assumptions are:

- the opaque logical model for `Address` set-ness and equality;
- `nz_checked_mul` and `nz_min`, because the pinned `vstd` does not specify those `NonZeroU64`
  operations;
- constructors for opaque account header and reserved padding values;
- the imperative instruction shells.

The canonical `NonZero` view, construction, access, and positivity model come from `vstd`, not from
lane-owned axioms.

## Key findings

- A `verus!` source file can still be the shipping program: macro erasure keeps ghost state and `vstd`
  out of the SBF graph.
- The pinned `vstd` contains a `NonZero<T>` model behind an undeclared
  `feature="nonzero_internals"` cfg. The verify recipe supplies that cfg through `RUSTFLAGS`; a normal
  Cargo feature cannot activate it.
- Verus diagnostics point to source spans and usually keep proof debugging inside the editor, unlike
  the translated WhyML diagnosis required by Creusot.
- Foreign-function specifications can attach directly through verifier registrations, avoiding the
  extension-trait call-site bending used by Creusot.
- The practical proof delta from Creusot is arithmetic guidance rather than theorem strength: both
  prove the core contracts unbounded, but Verus needs one explicit nonlinear hint.

## Limitations

- The dispatcher byte front remains a trusted shell. Proving it would require specifications for raw
  pointer parsing and loop invariants for bounded wire validation.
- Rust-analyzer and rustfmt have partial visibility inside `verus!`; verusfmt is an additional tool.
- The transition text guard is hard-coded to Creusot and does not scan this lane.
- Verification produces no committed proof object or replay command. Cargo-Verus may reuse unchanged
  verification output through normal Cargo fingerprinting.

## Reproduce

```sh
just verus version
just verus fmt-check
just verus verify
just solana test-client-rust ticketing verus
```
