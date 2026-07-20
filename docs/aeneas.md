# Aeneas lane

| Field | Value |
| --- | --- |
| Lane | 5 |
| Status | Implemented |
| Pin | Aeneas `d71d2e3f`, Charon `5a501733`, Lean `v4.31.0` |
| Verification | Rust extraction followed by hand-written Lean proofs |
| Artifacts | Committed generated model and proof source |
| Last reviewed | 2026-07-19 |

[Aeneas](https://github.com/AeneasVerif/aeneas) verifies Rust by translation. Charon extracts the
crate to LLBC, Aeneas emits a pure functional Lean model, and separate hand-written Lean contracts are
proved against that model.

## Integration

`ticketing-aeneas` is a complete program crate with no in-Rust verification surface. It has no
`cfg(aeneas)`, verifier dependency, or proof attribute; ordinary Rust source is the extraction input.

The extracted `flows/` and `domain/` code uses explicit `if` and `match` rather than `?`. The `?`
operator expands through `Try` and `ControlFlow`, adding irrelevant branches and glue to every Lean
proof. Trusted instruction shells are not extracted and retain idiomatic `?` usage.

The pipeline is:

```text
Charon -> LLBC -> Aeneas -> TicketingAeneas.lean -> TicketingAeneasProofs.lean
```

`TicketingAeneas.lean` is generated and committed but never hand-edited.
`TicketingAeneasProofs.lean` contains the maintained contracts and proofs. `verify` regenerates and
checks both; `replay` checks the committed Lean without extraction.

## Verification coverage

The Lean proof covers the three cores and three domain operations at the Creusot target level:

- exact, positive, overflow-free payment;
- exact create-time overflow discrimination and initial event invariant;
- exact positive clamp, no overselling, full frames, and on-error unchanged state;
- legal close transition and organizer authentication;
- end-to-end purchase composition, ticket invariant, ticket/event consistency, and exact economics.

The nonlinear capacity-times-price implication is proved with mathlib monotonicity rather than
trusted as an arithmetic lemma. `payment_for` is total because the invariant proves its error branch
dead and a fallible Rust shape would add extraction scaffolding without strengthening the Lean theorem.

## Trusted surface

Wire types are opaque in the generated model. Their recursive account header, padding, and foreign
`NonZeroU64` representation are unsuitable for direct extraction, so `domain::glue` is specified by
a trusted Lean interface:

- logical projections and invariants for `Event` and `Ticket`;
- accessor and setter contracts with full modeled frames;
- constructor contracts pinning every modeled field;
- `NonZeroU64` to `u64` bridges;
- address equality and set-ness.

Account headers and reserved bytes are not modeled. Arithmetic, clamp behavior, status transitions,
and invariant preservation are proved above that interface.

## Key findings

- Separate Lean provides a reviewable generated model and hand-written proof artifact, at the cost
  of maintaining proofs outside the Rust source.
- Arithmetic that Flux leaves trusted is proved directly in mathlib.
- Charon's default MIR-only sysroot cannot link this `cdylib`; extraction uses `--sysroot default` and
  a dedicated target directory.
- `lean` is invoked directly with a baked `LEAN_PATH`. `lake build` would re-resolve and clone mathlib,
  violating the runtime hermeticity goal.
- Keeping proof source separate makes ordinary Rust tooling work everywhere, but contracts can drift
  from Rust names until regeneration exposes the mismatch.

## Limitations

- Correctness of the opaque wire/glue specifications is trusted.
- The generated model is reviewable but is not the shipping SBF code.
- Full verification includes Charon extraction and Lean generation; replay runs only the Lean check.
- The transition text guard and dispatcher byte-front proof do not cover this lane.

## Reproduce

```sh
just aeneas version
just aeneas verify
just aeneas replay
just solana test-client-rust ticketing aeneas
```
