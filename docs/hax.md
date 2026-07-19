# hax lane

[hax](https://github.com/cryspen/hax) (Cryspen) verifies Rust by *translation*:
a rustc driver extracts the crate and the hax engine emits a model in a backend
prover - here **Lean 4**, against the [hax Lean proof
library](https://github.com/cryspen/hax/tree/main/hax-lib/proof-libs/lean)
(`Hax`, the `RustM` error monad + the `mvcgen` verification-condition generator).
It shares Aeneas's translate-then-prove quadrant, so the two run head-to-head on
the same Lean target; the difference is the *pipeline* (hax's Rust engine + the
`RustM`/`mvcgen` encoding vs Charon/LLBC + the Aeneas Std) and that hax's
contracts live **inline in the Rust source** as `hax_lib` attributes. See
[provers.md](provers.md) for where it sits in the comparison; this is the full
lane report. The F* and Rocq backends are out of scope, keeping the comparison
with Aeneas on one Lean target.

## What is wired

A full `ticketing-hax` program crate over `ticketing-interface` - a port of the
`ticketing-creusot` reference program: trusted `instructions/` shells +
dispatcher + `entrypoint!` around the `flows/` cores and the `domain/`
accounting, building a behavior-identical `ticketing_hax.so`. The generated
client's Mollusk suite passes against it - the same 7 tests as the other lanes
(`just solana test-client-rust ticketing hax`).

- **Inline, `cfg(hax)`-gated verification surface.** The contracts and the glue
  model are hax attributes gated behind `cfg(hax)` (which the driver sets during
  extraction), so ordinary `cargo build` and `cargo build-sbf` compile the
  identical program with zero verification overhead and no `hax-lib` dependency
  (it rides in only under `[target.'cfg(hax)'.dependencies]`).
- **Extraction-first control flow.** The extracted cores (`flows/`,
  `domain::event`) use explicit `if`/`match` instead of `?`, the same reason as
  Aeneas: `?` extracts through the `Try`/`ControlFlow` machinery and buries the
  Lean model in `branch`/`from_residual` scaffolding.
- **The pipeline** (`scripts/hax/verify.sh`): `cargo hax -i '<clause>' into
  --output-dir verif/ticketing_hax/extraction lean`, then `lean` against the
  image's baked `Hax`/`Qq` oleans (not `lake build`, which re-clones quote4).
  Extraction is scoped with the `-i` include clause (`-** +ticketing_hax::flows::**
  +ticketing_hax::domain::**`) - the analog of Aeneas's `start_from` - so the
  imperative shells and `entrypoint!` are never translated. Generated Lean is
  regenerated and re-checked from scratch every run and is git-ignored; the Rust
  attributes and the committed trusted model preamble are the source artifacts,
  so (as in the scaffold) there is no separate `replay`.

## What is proved

All six decision cores carry machine-checked Lean contracts, written
inline as `#[hax_lib::lean::after("theorem … := by …")]` Hoare triples (`⦃ ⦄`,
the idiom the upstream `barrett`/`adc` examples use for rich postconditions) and
proved with `hax_mvcgen` + `grind`/`omega` over the glue models. Coverage
mirrors the Creusot reference lane except where noted:

- **`payment_for`** - `payment = quantity * price`, **positive** and with **no
  overflow**, for positive `quantity <= capacity`. The nonlinear `quantity *
  price <= capacity * price <= u64::MAX` step is **proved** from the invariant
  (`Nat.mul_le_mul` + `UInt64.mulOverflow_iff`), not trusted. *Flux trusts this
  as `mul_le_mono`; Verus uses `by (nonlinear_arith)`; hax and Aeneas prove it.*
- **`create_event`** - `Ok` yields an initial event **satisfying the invariant**;
  the error occurs **exactly when** `capacity * price` exceeds `u64::MAX`. Because
  hax's Lean prelude has no model for `u64::checked_mul`, the overflow gate is
  reformulated with integer division (`c > u64::MAX / p`) - behavior-identical
  (`p >= 1` always, and the division's panic-freedom VC is discharged from the
  `NonZero` model's `nzVal_pos`), and the `Ok<=>overflow` discriminant is proved
  from `core`'s `Nat` division facts (`Nat.div_lt_iff_lt_mul`, `Nat.div_mul_le_self`).
- **`record_sale`** - preserves the event invariant (**no overselling**), changes
  only `sold` (every other modeled field pinned), grows `sold` by exactly the
  returned quantity, the quantity is positive and bounded by both `requested` and
  the remaining capacity, succeeds exactly when sales are open with headroom, and
  on error the event is unchanged. **Cost (recorded honestly):** the clamp is
  stated as the two bounds `actual <= requested` and `actual <= capacity - sold`
  rather than the *exact* value `actual = min(requested, capacity - sold)` -
  `grind` did not close the `min` characterization under the `int` spec-set. This
  is the same clause **Flux** drops; Creusot/Aeneas prove it exactly.
- **`close_sales`** and **`close_event`** - preserve the invariant, change only
  `status` (`open -> closed`; every other field pinned), report a legal
  transition, authenticate the organizer (`close_event`), and give the exact
  `Ok <=> organizer & open` / on-error-unchanged discriminant.
- **`purchase`** (ticket_buy) - the end-to-end buy composition (validate the
  organizer, `record_sale`, `payment_for`, build the ticket). The event invariant
  is preserved, only `sold` moves and grows by exactly the clamped quantity, the
  ticket carries its invariant and pins the buyer/event/payment/bump/slots, and the
  payment is `quantity * price` (positive, no overflow); on error the event is
  unchanged. Proved *modularly* by reusing the `record_sale`/`payment_for` triples
  as `mvcgen` step specs rather than unfolding them (see **Composing `purchase`**
  below), so its clamp keeps `record_sale`'s two-`<=`-bound form (not the exact
  `min`).

The `&mut Event` cores keep the sibling lanes' signatures: hax's experimental
Lean backend lowers `&mut T` into a `(T, …)` tuple return and resolves
`future(event)` to the post-state, so the frames and transitions are stated
directly over the mutated event (no value-passing fallback was needed).

### Composing `purchase`

`purchase` (ticket_buy) is the one core that *composes* two proved cores
(`record_sale` then `payment_for`) rather than reasoning locally, and hax `0.3.7`
does not wire that composition up automatically: the `Spec`-structure form
`mvcgen` would use modularly is generated from `requires`/`ensures`, but that
path's `pureEnsures` obligation rejects the `Result` match in every postcondition
here ("`hax_construct_pure`: mvcgen generated more than one goal … remove …
match-constructs"), which is exactly why the leaf cores use hand-written
`lean::after` triples. Unfolding `record_sale` + `payment_for` inline instead
re-raises the composed 64-bit nonlinear pricing goal under the `int` spec-set's
narrow-integer noise, which defeats `grind`.

The way through is to reuse the hand-written triples themselves as `mvcgen` step
specs. `record_sale_spec` and `payment_for_spec` are registered with `attribute
[spec]`, and `purchase`'s proof runs `hax_mvcgen [purchase, -record_sale,
-payment_for] <;> grind`: the `-record_sale`/`-payment_for` erase suppresses the
backend's default `@[spec]`-on-the-def unfold, so each callee is applied
*opaquely* by its contract. The nonlinear `capacity*price` bound then stays
discharged inside `payment_for_spec` and never re-enters this proof; the only new
facts `grind` needs are the `nzMk` round-trips on the clamped quantity and the
price (`model_preamble.lean`'s `nzVal_nzMk_of_pos`, keyed on the `0 < _` the leaf
postconditions already carry). This is the modular composition that the blocked
auto-`Spec` path could not give - the same recovery hax's own `chacha20` example
uses to compose spec'd callees.

## Trusted surface

The foreign wire types are deselected by the `-i` clause, so a committed model
preamble (`verif/ticketing_hax/model_preamble.lean`, spliced into the generated
module by `verify.sh` ahead of the first namespace) supplies the trusted model -
the analog of Aeneas's committed axioms and Flux's `#[extern_spec]`s:

- `Event`/`Ticket` as Lean **structures** over the modeled fields; the padded,
  self-referential header and the reserved bytes are dropped (the same fields
  Aeneas leaves unmodeled).
- `Address` as `Nat` (a 32-byte address as a number; a sound over-approximation)
  with the not-all-zero `is_set` predicate, so its equality is decidable and
  computable - no `Classical` choice leaks into the extracted glue.
- `NonZeroU64` as the `u64` subtype `{x // x ≠ 0}`, so its positivity is carried
  by the model and `nzVal_pos` is **proved**, not axiomatized (a smaller trusted
  surface than the opaque-`NonZero` lanes). The single remaining axiom is
  `nzVal_nzMk` (the `u64 -> NonZeroU64` bridge is the identity on positive values;
  its clamp-at-zero arm is dead under the callers' `v ≠ 0` obligation).
- `ProgramError` as the two variants the cores build.
- The field-level `domain::glue` given concrete Lean models via
  `#[hax_lib::lean::replace(...)]` (accessors project a structure field, setters
  are record updates, constructors build the structure, the `NonZero <-> u64`
  bridges use `nzVal`/`nzMk`). `mvcgen` unfolds these directly, so - unlike
  Aeneas's opaque glue + `@[step]` axioms - the glue is trusted only in that the
  models are asserted faithful, with no accessor/setter *axioms*.

Everything else - the clamp bounds, the no-underflow/no-overflow arithmetic
(incl. the nonlinear `capacity * price` and the division-form overflow gate), the
status step, and invariant preservation - is proved.

## Findings / deviations from the Creusot/Kani template

- **`future`/`&mut` works in the Lean backend.** The pre-flagged risk (whether
  hax `0.3.7`'s experimental Lean backend lowers `&mut` postconditions) resolved
  favorably: it threads `&mut Event` into the `RustM` tuple return and resolves
  `future(event)`, so the cores keep their `&mut` signatures.
- **Rich contracts use `lean::after`, not `requires`/`ensures`.** The
  `requires`/`ensures` -> `Spec` path works for simple predicates (the scaffold's
  `smoke_square`) but its `pureEnsures` construction cannot handle a `match` on
  the `Result` discriminant, which every real postcondition here needs. The
  contracts are still inline Rust attributes, but as embedded Lean Hoare triples
  (`lean::after`) - the same idiom hax's own `barrett`/`adc` examples use, noting
  auto-generation from `requires`/`ensures` is future work upstream. Those
  hand-written triples can still be **registered as `mvcgen` specs** (`attribute
  [spec]`) and reused across cores, so `purchase` composes modularly without the
  blocked auto-`Spec` path (see **Composing `purchase`**).
- **`checked_mul` unmodeled -> division-form overflow gate** (see `create_event`
  above): behavior-identical, keeps the bound proved.
- **Committed trusted model, spliced post-extraction.** hax has no cargo-metadata
  scoping like Charon's `opaque`, and `lean::before` injection lands inside the
  first item's namespace; splicing the committed `model_preamble.lean` into the
  generated module is the deterministic way to supply the root-level foreign-type
  model. This is the hax analog of Aeneas's committed model file.
- **Toolchain.** `cargo-hax`/`hax-driver`/`hax-rust-engine` pinned together at
  `0.3.7` over `nightly-2025-11-08`; elan `4.2.3` installs Lean `v4.29.0-rc1`; the
  hax Lean proof library is prebuilt from commit `d8b5b3d` and baked into the
  bookworm image. At `0.3.7` the Lean backend still delegates THIR import to the
  native OCaml `hax-engine` (opam-built in isolation, only the binary shipped),
  so the image is Aeneas-class to build. The ["Verification
  Facade"](https://symbolic.software/blog/2026-04-07-cryspen-hax/) critique of the
  hax pipeline is the live case study for the trusted-translation axis here.
