# VeriFast lane

Lane 7 of [the prover roadmap](provers.md). Status: **wired up** - a real Pinocchio program plus a
detached VeriFast proof of its decision cores. Facts are point-in-time; last reviewed 2026-07-19.

[VeriFast](https://github.com/verifast/verifast) is a separation-logic verifier for C, Java, and
(experimentally) Rust. It drives `rustc +nightly-2025-11-25` to MIR and discharges contracts written
in `//@` ghost comments by symbolic execution over its built-in Redux prover. In the Rust standard
library campaign it handles the most pointer-intensive unsafe modules (`RawVec`, `LinkedList`,
`BTreeNode`, `Vec`).

## The target-fit decision

`provers.md` flagged this lane's open question: this repo's verified layer is safe, pure, loop-free
decision logic, whereas separation logic's strengths are heap and pointer reasoning. The decision
taken here is to **re-prove the same three decision cores as every other lane** (spec coverage >=
Creusot), not to widen the workshop's scope to the trusted shells or the zero-copy `bytemuck` wire
casts. Because the verified layer has no heap, VeriFast expresses the full functional contract
without exercising separation logic at all - itself part of the finding.

## What is wired

**The program.** `crates/ticketing/verifast` is a full Pinocchio program over the shared
`ticketing-interface`, implementing the shared wire protocol and decisions: trusted `instructions/` shells
+ dispatcher/`entrypoint!`, the `flows/` decision cores, and the `domain/` accounting/lifecycle logic
(extension traits on the wire types). It carries no in-source spec layer and no verifier dependency,
like the Aeneas lane. `just solana build ticketing verifast` emits `ticketing_verifast.so`, and the
lane-agnostic client SBF suite passes against it (`just solana test-client-rust ticketing verifast`
- 7/7, including the full create -> buy -> close lifecycle).

**The proof.** VeriFast verifies a self-contained model of the three cores in
`crates/ticketing/verifast/verification/model.rs`: `create_event`, `purchase`, and `close_event`
against `//@ req`/`//@ ens` contracts, plus the domain predicates they are written in
(`event_invariant`, `is_legal_transition_from`, the frames, `event_is_initial`, the ticket
predicates) as VeriFast `fix` fixpoints - the analog of Creusot's `#[logic]` functions.
`just verifast verify` reports `0 errors found (143 statements verified)`; it re-verifies from source
each run (no committed proof session to replay). The model file is a standalone VeriFast target
outside the crate's module tree - `cargo` never compiles it.

## Why the proof is detached, not on the shipping cores

Every other lane verifies its shipping cores directly (cfg-gated specs on the real program). VeriFast
cannot: its Rust frontend verifies a single crate and does not import type definitions from external
dependencies. A spike pinned the shape of the wall - `cargo verifast` **compiled** the whole
dependency graph (curve25519-dalek, bytemuck, solana-address, pinocchio, ticketing-interface) under
VeriFast's rustc, but then reported `No such struct: "ticketing_interface::Event"`: the `--extern`
rlibs are linked, yet the frontend has no view into their structs. Verifying the real cores would
mean re-declaring the entire wire layer (`Event`/`Ticket`/`NonZeroU64`/`Address`/`AccountHeader`/
`BoundedString` + const generics) as a trusted `.rsspec` - a large trusted surface with uncertain
const-generic/`PhantomData` support. So the lane proves a **model intended to reproduce the cores'
decision semantics** instead. This is the lane's headline cost, unique in the roster: the verified
artifact is a model, not the shipping code, and model-to-program fidelity remains trusted (the
shipping program is separately covered by the client SBF suite).

## The model - findings

- **`unsafe fn` buys clean contracts.** VeriFast verifies a safe `fn` against its RustBelt semantic
  type, which for a function returning a composite (`Result`/`Option`/struct) demands the
  ownership-token ceremony (`thread_token`, `<T>.own`, and open/close of every returned value).
  Marking the cores `unsafe fn` drops that ceremony, so the contracts read as pure `//@ req`/`//@ ens`
  like the other lanes' `requires`/`ensures`. The bodies contain no unsafe operations; a file-level
  `#![allow(unsafe_op_in_unsafe_fn)]` keeps the pre-2024 implicit-unsafe-body behavior rather than
  wrapping semantically-empty `unsafe` blocks around internal calls.
- **u64-modeled scalars, ranges carried by the invariant.** `NonZeroU64` is bridged to `u64` (as in
  the Flux and Aeneas lanes), so `capacity`/`price_lamports` carry an explicit `1 <= x <= u64::MAX`
  in `event_invariant`, and `sold` carries its `0 <=` lower bound - VeriFast does not range-constrain
  the fields of a by-value struct on its own (the ownership predicate that would carry those bounds
  is exactly what `unsafe fn` skips, so the invariant states them). `Address` is modeled by set-ness
  (`!= 0`), `EventStatus` by a `u8` tag (0 = open, 1 = closed), `EventName` by an equality tag. The
  wire header and reserved padding, irrelevant to the invariants, are dropped (as in the hax lane's
  Lean model), so their frame clauses are omitted.
- **One trusted axiom: the overflow-checked multiply.** `checked_mul_u64` is `assume_correct` (a
  trusted body, VeriFast's `#[trusted]` analog) with the contract "None exactly on `u64` overflow,
  else the product" - the direct analog of Creusot's `ax_checked_mul`. It is the model's only axiom
  (hence the `-allow_assume` flag on the recipe); everything else is proved.
- **Nonlinear arithmetic is manual, and proved.** VeriFast deliberately does not discharge nonlinear
  multiplication automatically. `purchase` must show the payment cannot overflow -
  `quantity * price <= capacity * price <= u64::MAX` - and that the product is non-negative; both
  come from two hand-invoked `mul_mono_l` steps (the monotonicity lemma in `prelude_core.rsspec`).
  With those, `payment = quantity * price` is a direct, proven-overflow-free multiply, so - unlike
  Creusot's `checked_mul` in the same spot - the model's `purchase` needs no overflow branch. This
  echoes the Kani lane's finding that the nonlinear bound is the real cost, but VeriFast proves it
  unbounded (no `u8` capacity cap).
- **Enums are friction.** `#[derive(Clone, Copy)]` on an enum trips the frontend ("Expressing shared
  ownership of an enum value is not yet supported"), and matching an owned enum field partially moves
  it; modeling `EventStatus` as a `u8` tag sidesteps both. There is no `#[derive]` in the model.
  Other small syntax notes: `fix` predicates must `match` on a parameter (not a field), a boolean
  fixpoint used as an assertion needs `== true`, and ghost `match` arms need named bindings. On the
  plus side, struct update syntax (`Event { status: 1, ..event }`) verifies fine, and the model
  needs no `verifast_options` directives: with every function carrying a contract and the arithmetic
  proven overflow-free, neither `skip_specless_fns` nor `ignore_unwind_paths` has anything to do.

## Coverage vs Creusot

The model proves full parity on the reference clauses: the event invariant (organizer set, no
oversell, bounded economics), `create_event`'s `Ok <=> !overflow` discriminant and full `is_initial`,
`purchase`'s exact clamp `min(requested, capacity - sold)`, the exact `sold` delta,
`payment == quantity * price` (proved overflow-free), the ticket `is_initial` + `is_consistent_with`,
the legal status transition and the frame conditions on both mutating cores, and on-error-unchanged.
No clause is dropped.

Representational departures (not clause drops): the model's `&mut Event` cores are value-threading
`fn`s returning the updated event (the backward-function shape Aeneas/hax also use); the wire
header/reserved padding are not modeled; and - the load-bearing one - the proof is over a model, so
it does not by itself certify the shipping cores. Their behavior parity rests on the model being a
faithful transcription plus the client SBF suite exercising the deployed `.so`.

## Transition guard

`scripts/check-ticketing-transitions.sh` scans `crates/ticketing/creusot/src/flows` and matches the
`#[cfg_attr(creusot, ensures(...))]` attribute shape, so it does not cover this lane. The
`is_legal_transition_from` clause is present on both mutating cores of the model; it is simply not
machine-checked by that guard here.
