# VeriFast lane

| Field | Value |
| --- | --- |
| Lane | 7 |
| Status | Implemented program plus detached verification model |
| Pin | VeriFast `26.01`, Rust `nightly-2025-11-25` |
| Verification | Symbolic execution with the built-in Redux prover |
| Artifacts | None; every run re-verifies the detached model |
| Last reviewed | 2026-07-19 |

[VeriFast](https://github.com/verifast/verifast) verifies C, Java, and experimental Rust with
separation logic and symbolic execution. This lane tests a deliberate target-fit risk: the workshop's
verified cores are safe, pure, and heap-free, so they do not exercise pointer ownership and
separation.

## Integration

The lane has two related but distinct artifacts.

`ticketing-verifast` is a complete Pinocchio program over the shared interface. It contains the same
instruction shells, runtime decisions, and SBF entrypoint as the other program crates, but no verifier
annotations or dependencies.

`crates/ticketing/verifast/verification/model.rs` is a standalone VeriFast target outside the Cargo
module tree. It transcribes `create_event`, `purchase`, and `close_event` into a self-contained scalar
model with `//@ req` and `//@ ens` contracts plus `fix` predicates for invariants and frames.

The split is required by the frontend. A compatibility spike compiled the full Cargo dependency graph
with VeriFast's Rust toolchain but then failed with `No such struct: "ticketing_interface::Event"`:
linked dependency metadata does not expose external struct definitions to the verifier. Re-declaring
the complete wire graph as trusted `.rsspec` files would create a larger and less reviewable trusted
surface than a detached decision model.

## Verification coverage

The model proves the behavioral reference properties on the three top-level cores:

- event invariant, no overselling, and bounded economics;
- exact event-creation overflow discrimination and initial state;
- exact purchase clamp, sold delta, payment, and ticket consistency;
- legal close transition and exact frames;
- on-error unchanged state.

The proof is unbounded. Nonlinear multiplication monotonicity is established manually with two
`mul_mono_l` applications, so purchase needs no overflow branch after the event invariant is assumed.

The model does not separately define or verify the shipping leaf operations `payment_for`,
`record_sale`, and `close_sales`; their behavior is represented directly in the top-level cores. The
current verifier result is 143 statements verified with zero errors.

## Trusted surface

The model has one explicit `assume_correct` function: `checked_mul_u64`, specified to return `None`
exactly on `u64` overflow and otherwise the mathematical product. The recipe's `-allow_assume` flag
is present solely for that axiom.

Representational choices are also trusted correspondence decisions:

- addresses and event names are scalar equality tags;
- `NonZeroU64` values are positive `u64` values;
- event status is a `u8` tag;
- headers and reserved padding are omitted;
- mutable cores return updated values rather than borrowing the shipping structs.

Most importantly, the model is not mechanically linked to the shipping source. The shared SBF tests
exercise both against the same protocol but cannot prove transcription fidelity.

## Key findings

- No separation logic is needed: the target decision layer has no heap or pointer ownership.
- Marking model cores `unsafe fn` avoids RustBelt ownership-token ceremony for returned composite
  values. The bodies contain no unsafe operation; this is a verifier modeling choice.
- Nonlinear arithmetic is manual but fully proved and unbounded.
- Enum ownership is immature in the Rust frontend. Modeling status as `u8` avoids unsupported shared
  enum ownership and partial-move behavior.
- The detached proof is compact and complete at the top-level contract boundary, but that convenience
  introduces a substantial correspondence assumption.

## Limitations

- The proof does not certify the shipping core source directly.
- Separate domain leaf functions are not verified as independent units.
- Wire representation, account parsing, instruction shells, and model fidelity remain trusted.
- The transition text guard covers only Creusot.
- The lane exercises symbolic execution and arithmetic, not VeriFast's defining separation-logic
  strengths.

## Reproduce

```sh
just verifast version
just verifast verify
just solana test-client-rust ticketing verifast
```
