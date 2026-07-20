# Creusot lane

| Field | Value |
| --- | --- |
| Lane | 1, reference baseline |
| Status | Implemented |
| Pin | Creusot `v0.12.0`, Rust `nightly-2026-04-21` |
| Verification | Deductive proof through Why3 and SMT |
| Artifacts | 23 committed `proof.json` sessions |
| Last reviewed | 2026-07-19 |

[Creusot](https://github.com/creusot-rs/creusot) translates Rust with pearlite contracts into Why3.
`why3find` dispatches the resulting verification conditions to the Alt-Ergo, Z3, CVC4, and CVC5
configuration in `why3find.json`.

## Integration

`ticketing-creusot` is the source baseline for the other program crates. Its structure makes the
verification boundary explicit:

- `instructions/` contains trusted Solana shells: account validation, PDA creation, Clock access,
  lamport-transfer CPI, and persistence.
- `flows/` contains the three pure decision cores.
- `domain/` contains predicates, verified state operations, and trusted foreign-type axioms.

Contracts use `cfg_attr(creusot, requires(...))` and `ensures(...)`; predicates and verifier-only
dependencies are gated by `cfg(creusot)`. Ordinary host and SBF builds contain none of the proof
surface.

The lane owns `just creusot verify`, deterministic `replay`, the Why3 IDE workflow, and a pre-push
replay gate. The transition text guard also covers this lane's mutable flow cores.

## Verification coverage

Creusot defines the target coverage for later lanes:

- `payment_for` returns the exact positive `quantity * price` product and cannot overflow.
- `create_event` succeeds with a fully initial event and fails exactly when
  `capacity * price_lamports` overflows.
- `record_sale` preserves the invariant, changes only `sold`, returns the exact positive clamp, and
  leaves the event unchanged on error.
- `close_sales` and `close_event` prove the legal open-to-closed transition, exact frames,
  organizer authentication, and on-error unchanged state.
- `purchase` composes the domain operations end to end: no overselling, exact sold delta, consistent
  ticket construction, exact payment, invariant preservation, and on-error unchanged state.

The proof is unbounded. The nonlinear multiplication bound closes through the solver ensemble without
an in-source proof hint.

## Trusted surface

The imperative instruction shells are outside the proof. The domain layer also trusts explicit models
for foreign types:

- `NonZeroU64` operations are tied to an opaque logical integer view through `ax_*` shims.
- `Address` set-ness and equality are modeled through trusted predicates and wrappers.
- Extension traits carry foreign-function contracts because Rust's orphan rule prevents the desired
  direct registrations.

The trusted surface is visible in source and is part of the comparison; solver success does not prove
the axioms themselves.

## Key findings

- The executable cores stay plain Rust, so rustfmt, Clippy, and rust-analyzer see the complete runtime
  code.
- Committed Why3 sessions provide reviewable and deterministic replay rather than fresh solver search
  in every gate.
- Proof failures are diagnosed in translated WhyML goals through the Why3 IDE. That makes the success
  path highly automated but adds a second language and mental model when a goal fails.
- The solver ensemble proves the nonlinear economics without the explicit arithmetic hint required by
  Verus or the domain bound required by Kani.

## Limitations

- Account parsing, CPI, clock reads, and persistence remain trusted.
- Foreign-type correctness depends on lane-owned axioms.
- The byte-parser panic-freedom property covered by Kani is not proved here.
- The transition guard is a syntax-level backstop for Creusot contracts, not a substitute for the
  verifier.

## Reproduce

```sh
just creusot version
just creusot verify
just creusot replay
just solana test-client-rust ticketing creusot
```

Use `just creusot verify-ide` only for interactive diagnosis of an unproved goal.
