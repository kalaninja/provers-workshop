# Kani lane

| Field | Value |
| --- | --- |
| Lane | 2 |
| Status | Implemented |
| Pin | Kani `0.67.0` |
| Verification | Bounded model checking through CBMC and Kissat |
| Artifacts | None; every run re-verifies from source |
| Last reviewed | 2026-07-19 |

[Kani](https://github.com/model-checking/kani) compiles Rust MIR to a CBMC goto program and asks a
SAT solver to find an execution that violates a property. An `UNSAT` result establishes the property
only within the modeled domain and unwinding bounds.

## Integration

`ticketing-kani` is a complete program crate over the shared interface. The runtime code calls standard
library operations directly; Kani-specific material is gated by `cfg(kani)`:

- function contracts use `kani::requires`, `kani::ensures`, and `kani::modifies`;
- a `proofs.rs` module contains three `proof_for_contract` harnesses and one dispatcher harness;
- each contract harness includes `kani::cover!` checks for its interesting result branches;
- the recipe enables the unstable `-Z function-contracts` feature and runs harnesses in parallel.

The fourth harness feeds the dispatcher every instruction payload length through one byte past the
largest layout with no accounts. It proves that the discriminator and argument front always reject
the input without panicking.

## Verification coverage

The three flow-core contracts mirror the Creusot clauses for event creation, purchase, and closure.
The domain operations are reached through those cores and carry their own function contracts.

The important qualification is arithmetic width: `capacity` is modeled as `NonZeroU8`, while price
remains a full `u64`. This keeps overflowing creation inputs reachable but bounds every symbolic
capacity-times-price multiplication. The cores are loop-free; only the `BoundedString` parser needs
an explicit unwind bound in the dispatcher harness.

Within that domain Kani checks:

- invariant preservation and no overselling;
- exact state frames and error behavior;
- exact clamping and payment;
- the load-bearing create-time overflow guard;
- dispatcher panic freedom for the enumerated byte lengths.

## Trusted surface

Kani needs no foreign-type axioms in this lane. `NonZeroU64`, `min`, checked arithmetic, and address
equality execute as ordinary Rust inside the model checker.

Its trust boundary is instead reachability-based: only code reachable from a proof harness is checked.
Instruction interiors after parsing—PDA creation, runtime clock access, CPI, and persistence—remain
outside the harness graph. Shrinking that graph would not leave a marker in production code, which is
why the harness set and cover assertions are review-critical.

## Key findings

- Parser code is a better BMC target than a deductive target: one byte-oriented harness covers a useful
  part of the otherwise trusted shells.
- Nonlinear 64-bit multiplication, not control flow, dominates verification cost. Bit-blasting a
  symbolic multiplier is the reason for the `u8` capacity bound.
- `kani::cover!` prevents an accidentally contradictory precondition from turning verification green
  vacuously.
- A mutation that removes the event-creation overflow check fails both the success contract and the
  cover requiring an overflow rejection.
- Counterexamples can be rendered as concrete playback tests, providing a direct failing-input
  workflow.

## Limitations

- The proof does not cover capacities above `u8::MAX`.
- A future caller using `stub_verified` must not treat the contracts as established outside the harness
  bound.
- Verification leaves no replayable proof object and must run from source.
- The shared transition text guard does not scan this lane.

## Reproduce

```sh
just kani version
just kani verify
just solana test-client-rust ticketing kani
```
