# Flux lane

| Field | Value |
| --- | --- |
| Lane | 4 |
| Status | Implemented |
| Pin | Flux `1d986760`, Rust `nightly-2025-11-25`, Z3 `4.16.0` |
| Verification | Refinement checking through liquid-fixpoint and Z3 |
| Artifacts | None; every run re-verifies from source |
| Last reviewed | 2026-10-13 |

[Flux](https://github.com/flux-rs/flux) expresses program properties as refined types and relies on
liquid inference rather than explicit proof blocks. This lane tests the ergonomic benefit of carrying
the event invariant on the type itself.

## Integration

`ticketing-flux` is a complete program crate with trusted instruction shells, refined decision cores,
and a domain layer containing verified operations plus the foreign-type adapters required by Flux.

`cargo flux` sets `cfg(flux)`, which gates logic definitions and external specifications. The
`flux_rs` attribute dependency is unconditional because Cargo resolves target dependency tables before
the per-package Flux cfg is active; its attributes become no-ops in ordinary and SBF builds.

`just flux verify` enables `-Fstd-extern-specs=on` inside the container. Flux's standard-library
specifications are opt-in at this pin; without the flag, `Result`/`Option` refinements and checked
arithmetic specifications are not loaded automatically.
The command also adds `/opt/flux` to the dependency search path for those precompiled libraries
and enables `-Fcheck-overflow=strict` to check integer overflow and underflow.
At this pin, `?` loses the `Result` discriminant refinement because `Try::branch` has no matching
specification. The checked sale and close functions use explicit `Result` branches instead.

The driver is built from the pinned Git revision. Z3 and liquid-fixpoint are separate mise-managed
tools; liquid-fixpoint's prebuilt binary is available only from its mutable `nightly` release tag, the
lane's one intentionally non-immutable external asset.

## Verification coverage

Flux checks nine functions, recognizes 15 trusted items, and solves 123 constraints. It establishes:

- the event invariant as a type invariant;
- event creation succeeds exactly when `capacity * price` fits in `u64`, and pins the initial
  organizer, capacity, price, name, counter, status, bump, and slots;
- no overselling and exact sale quantity and `sold` increment:
  `min(requested, capacity - old_sold)`;
- exact frames over every modeled event field, including identities for the header, name, and
  reserved bytes;
- exact success conditions for sale and close operations, including their organizer checks;
- a closed status after successful `close_sales` or `close_event`;
- unchanged modeled event state on errors from sale and close operations;
- exact positive payment products without overflow;
- purchase consistency: the returned payment and ticket's payment both equal quantity times price,
  ticket quantity equals the counter increment, and the ticket pins its event, buyer, bump, and slots.

These cover the reference flow properties at the model level. They do not give the same proof
boundary as Creusot: the Flux wire constructors and field access remain trusted.

`payment_for` is total under `0 < quantity <= capacity`, using the event invariant to prove the
product fits. The reference helper accepts arbitrary positive quantities and reports overflow through
`Result`; the Flux helper's narrower precondition covers every quantity reachable from `purchase`.
Neither this lane's flow contracts nor the reference flow contracts specify concrete `ProgramError`
variants; they specify success and failure conditions.

## Trusted surface

`Event` and `Ticket` use opaque models to avoid expanding the wire types' recursive account headers
and padding into refinement types. Flux checks the cores against these models but trusts the bodies
of their constructors, accessors, and setters. Creusot checks those field constructions and updates.
This is the remaining integration boundary, not evidence that Flux cannot express ticket consistency.

The event model now includes every field. Header and reserved bytes have opaque identities, and an
opaque `BoundedString` model supplies the event-name identity. Equality in the frame predicates means
those identities stay unchanged. Their correspondence to the complete Rust field values is assumed;
adding them strengthens the trusted helpers' specifications rather than proving their bodies.
The same distinction applies to the expanded ticket-constructor specification.

The pinned standard-library specifications supply no numeric model for `NonZero<T>`.
The lane registers an opaque model indexed by an integer value. Its trusted `nz_get` and `nz_from`
bridges preserve that value, allowing callers to prove exact quantities and payments.
The generic type's sealed bound requires `nonzero_internals`, enabled only under `cfg(flux)`.
Creusot also trusts its `NonZero` wrappers; this is not a Flux-only gap.

There are no lane-owned trusted arithmetic lemmas. Flux proves the smaller-purchase bound and
positivity directly in `payment_for`, with strict overflow checking enabled. The `mul_fits` adapter
is checked against the standard-library models of `checked_mul` and `Option::is_some`; the library
implementations themselves remain outside this crate's proof.

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
- On 2026-10-13, enabling standard-library specifications and replacing `?` with explicit branches
  made the error frames provable without changing the pinned Flux revision. A temporary mutation
  that incremented `sold` before returning an error preserved the old bounds but failed the new
  unchanged-state clause. The original source was restored and the full check passed.
- The same day's `NonZero` model check proved the exact sale result and counter increment in
  `record_sale` and `purchase`. Returning one ticket instead of the clamped request preserved the
  earlier safety bounds but failed the new exact-quantity clauses. The correct source passed again.
- The coverage comparison also exposed omitted clauses rather than prover failures: creation fields
  and success equivalence, returned ticket/payment consistency, and the successful close status all
  checked after strengthening their specifications. Temporary mutations to the ticket quantity,
  returned payment, creation fields, close status, and overflow guard were rejected. The correct
  source passed again.
- The same audit removed `trusted` from `mul_fits`, deleted the unnecessary `mul_le_mono` and `mul_pos`
  helpers, and enabled strict overflow checking. The arithmetic is proved by the existing pinned
  toolchain; no Flux upgrade or new trusted function was needed.

## Limitations

- Wire constructors, accessors, setters, and numeric bridges remain trusted, including their
  correspondence to the field models. Contract coverage does not imply equality of trusted surfaces.
- The total `payment_for` helper is specified only for quantities within capacity.
- Concrete `ProgramError` variants are not specified.
- The dispatcher byte front and transition text guard remain outside this lane.
- The mutable liquid-fixpoint `nightly` asset weakens otherwise commit-pinned tooling reproducibility.

## Reproduce

```sh
just flux version
just flux verify
just solana test-client-rust ticketing flux
```
