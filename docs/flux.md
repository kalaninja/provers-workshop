# Flux lane

| Field | Value |
| --- | --- |
| Lane | 4 |
| Status | Implemented |
| Pin | Flux `1d986760`, Rust `nightly-2025-11-25`, Z3 `4.16.0` |
| Verification | Refinement checking through liquid-fixpoint and Z3 |
| Artifacts | None; every run re-verifies from source |
| Last reviewed | 2026-07-19 |

[Flux](https://github.com/flux-rs/flux) expresses program properties as refined types and relies on
liquid inference rather than explicit proof blocks. This lane tests the ergonomic benefit of carrying
the event invariant on the type itself.

## Integration

`ticketing-flux` is a complete program crate with trusted instruction shells, refined decision cores,
and a domain layer containing verified operations plus the foreign-type adapters required by Flux.

`cargo flux` sets `cfg(flux)`, which gates logic definitions and external specifications. The
`flux_rs` attribute dependency is unconditional because Cargo resolves target dependency tables before
the per-package Flux cfg is active; its attributes become no-ops in ordinary and SBF builds.

The driver is built from the pinned Git revision. Z3 and liquid-fixpoint are separate mise-managed
tools; liquid-fixpoint's prebuilt binary is available only from its mutable `nightly` release tag, the
lane's one intentionally non-immutable external asset.

## Verification coverage

Flux checks eight functions, recognizes 18 trusted items, and solves 58 constraints. It establishes:

- the event invariant as a type invariant;
- no overselling and monotonic `sold` updates;
- exact state frames and legal status transitions;
- bounded, positive, exact payment products;
- the load-bearing event-creation economics guard.

`payment_for` is total and returns a refined `u64`: the event invariant makes its overflow branch dead,
while a `Result` discriminant cannot be refined through the downstream standard-library type at this
pin.

The lane does not express the exact sale clamp, exact result discriminants, on-error unchanged state,
or full ticket consistency. Those are disclosed coverage gaps rather than silently weakened claims.

## Trusted surface

Two foreign-type limits shape the lane.

First, `Event` and `Ticket` cannot be registered transparently. Every external field needs a refinement
index, while the wire structs contain reserved padding and a self-referential `AccountHeader<Event>`.
The resulting recursive sort forces opaque models and a trusted accessor, setter, and constructor
layer.

Second, `NonZeroU64` cannot carry an integer refinement through its sealed generic standard-library
type, and Flux's own `core` refinements are not visible to this downstream crate. Accounting therefore
crosses trusted `nz_get` and `nz_from` bridges. The following arithmetic facts are also trusted:

- `mul_fits` relates checked multiplication to the mathematical product bound;
- `mul_le_mono` carries the event bound to a smaller purchased quantity;
- `mul_pos` establishes payment positivity.

The exact product itself is still stated and checked in the refined `payment_for` signature.

## Key findings

- The event invariant genuinely rides the type: every construction and mutation must re-establish it
  without repeating a function precondition.
- Core annotations are terse, but foreign-type opacity moves substantial behavior into trusted
  helpers.
- Built-in integer refinements, strong mutable updates, and frame predicates work cleanly once values
  have crossed the foreign-type boundary.
- Proof failures are source-spanned compiler diagnostics, and the full check is fast enough for an
  editor loop.
- Removing the sale clamp makes the trusted setter's `v <= capacity` precondition unprovable, providing
  a direct non-vacuity mutation test.

## Limitations

- Exact clamp equality, result-discriminant equivalence, on-error frames, and ticket consistency are
  not modeled.
- Arithmetic-overflow checking is not enabled as a Flux compiler option; explicit bounds carry the
  relevant safety properties instead.
- The dispatcher byte front and transition text guard remain outside this lane.
- The mutable liquid-fixpoint `nightly` asset weakens otherwise commit-pinned tooling reproducibility.

## Reproduce

```sh
just flux version
just flux verify
just solana test-client-rust ticketing flux
```
