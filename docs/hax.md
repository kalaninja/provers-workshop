# hax lane

| Field | Value |
| --- | --- |
| Lane | 6 |
| Status | Implemented |
| Pin | hax `0.3.7`, Rust `nightly-2025-11-08`, Lean `v4.29.0-rc1` |
| Verification | Rust extraction to inline Lean Hoare proofs |
| Artifacts | Inline proofs and committed model preamble; generated Lean is ignored |
| Last reviewed | 2026-10-13 |

[hax](https://github.com/cryspen/hax) translates Rust into a backend prover. This lane fixes Lean 4
as the target so it can be compared directly with Aeneas while exercising a different extraction and
proof model: hax's Rust engine, `RustM`, and `mvcgen`.

## Integration

`ticketing-hax` is a complete program crate. Contracts and model replacements are inline Rust
attributes gated by `cfg(hax)`; the `hax-lib` dependency is absent from ordinary and SBF target graphs.

As in the Aeneas lane, extracted cores use explicit control flow instead of `?` to avoid lowering
through `Try` and `ControlFlow`. The include clause selects only `flows/` and `domain/`, leaving the
imperative instruction shells outside extraction.

`scripts/hax/verify.sh` performs extraction, inserts the committed
`verif/ticketing_hax/model_preamble.lean`, and checks the generated module directly with `lean` against
the image's prebuilt Hax and Qq libraries. Generated Lean is deleted and regenerated on every run and
remains git-ignored.

## Verification coverage

The three cores and three domain operations carry inline `lean::after` Hoare triples proved with
`hax_mvcgen`, `grind`, and `omega`.

The lane proves:

- exact positive payment and the nonlinear no-overflow bound;
- the event-creation overflow discriminant and initial invariant;
- exact positive purchase quantity, no overselling, exact sold delta, full modeled frames, and exact
  errors;
- legal closure and organizer authentication;
- modular end-to-end purchase composition with a consistent ticket.

`record_sale` proves that the result equals `min(requested, capacity - sold)`. The composed
`purchase` proof carries that equality through to the ticket's quantity, the sold delta, and payment.
These properties hold over the full allowed input range and the modeled account fields.
An intentional mutation returning one ticket instead of the clamped quantity fails the exact-minimum
goal. The correct implementation passes after restoration.

Because the Lean prelude has no model for `u64::checked_mul`, event creation uses the equivalent
division guard `capacity > u64::MAX / price`. Positivity of the nonzero price discharges division safety,
and Lean proves the equivalence to multiplication overflow.

## Trusted surface

The committed model preamble defines:

- `Event` and `Ticket` as Lean structures over modeled fields;
- `Address` as a natural number with nonzero set-ness;
- `NonZeroU64` as a `u64` subtype;
- the relevant `ProgramError` variants;
- concrete `lean::replace` implementations for accessors, setters, and constructors.

The `NonZero` positivity and round-trip lemmas are proved. Unlike Aeneas's opaque glue axioms, the hax
glue reduces to concrete structure operations; trust rests on the correspondence between those models
and Rust wire types. Headers and reserved padding are not modeled.

## Key findings

- The experimental Lean backend handles `&mut Event`: `future(event)` resolves to the post-state, so
  the shipping signatures need no value-threading rewrite.
- Rich `Result` postconditions cannot use hax's automatic `requires` and `ensures` `Spec` path at this
  pin. Inline Lean Hoare triples are required.
- Hand-written leaf triples can still be registered as `mvcgen` step specifications. `purchase` uses
  `record_sale` and `payment_for` modularly rather than unfolding their nonlinear arithmetic.
- Adding exact `min` equality to both contracts closed the earlier clamp gap with the existing
  arithmetic proof. No tool upgrade, new axiom, or change to the executable Rust was needed. The
  fallback tactic now splits only conjunctions and closes trivial goals, so a false numeric goal
  produces a diagnostic instead of recursively applying numeric constructors until stack overflow.
- The Lean backend still calls a native OCaml helper for shared import phases, so the tooling image
  includes an OCaml build stage in addition to the Rust frontend.
- Direct `lean` checking avoids the network re-clone that a clean `lake build` would perform.

## Limitations

- The foreign-type and `lean::replace` correspondence is trusted.
- There is no stable proof-session replay; extraction and Lean checking run every time.
- The dispatcher byte front and transition text guard remain outside this lane.

## Reproduce

```sh
just hax version
just hax verify
just solana test-client-rust ticketing hax
```
