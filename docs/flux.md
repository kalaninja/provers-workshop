# Flux lane

Lane 4 of [the prover roadmap](provers.md). Status: **wired up**. Facts are
point-in-time; last reviewed 2026-07-13.

[Flux](https://github.com/flux-rs/flux) refines Rust's types with logical
predicates (liquid/refinement types) and checks them by SMT (liquid-fixpoint +
Z3) with heavy inference. It is the opposite end of the ergonomics spectrum from
the deductive lanes: specs are *type annotations*, not `requires`/`ensures`
blocks, and a green check takes a fraction of a second.

Why fourth in the roster: to contrast refinement types against the deductive
tools before the interactive provers. The premise from [provers.md](provers.md)
was that specs become refined signatures (`#[flux_rs::sig(...)]`) and struct
invariants (`#[flux_rs::invariant]`) - a different way of *saying* the same
contracts. That premise held for the invariant; the story of the lane is how far
Flux's foreign-type support at the pinned rev bends the rest.

## What is wired

A full `ticketing-flux` program crate over the shared `ticketing-interface` -
behavior-identical to the other lanes, split along the verification boundary:

- `instructions/` - trusted imperative shells (account parsing, PDA creation,
  clock reads, the transfer CPI, persistence) plus the dispatcher and
  `entrypoint!`. Each handler is `#[flux_rs::trusted]` (Flux lifts its signature
  but never looks inside), the analog of the Verus shells being outside
  `verus!{}`.
- `flows/` - the decision cores (`create_event`, `purchase`, `close_event`),
  each carrying its contract as a `#[flux_rs::sig]` refinement signature.
- `domain/` - the refinement vocabulary and the verified typed mutators
  (`record_sale`, `payment_for`, `validate_organizer`, `require_sales_open`,
  `close_sales`), plus the trusted shim layer the two limitations below force.

`just flux verify` checks the crate in ~200 ms (`8 checked; 18 trusted; 0
ignored`, 58 constraints solved), re-verifying from scratch each run (no
committed artifacts, like Kani/Verus). `just solana build ticketing flux` builds
the SBF `.so`, and the generated client's Mollusk suite passes against it
(`just solana test-client-rust ticketing flux`) - the same 7 tests that pass
against Creusot/Kani/Verus, since the program is behavior-identical.

The `flux_rs` attribute crate rides in as an unconditional dependency whose
attributes strip to no-ops unless `cargo flux` is driving, so ordinary
`cargo build` / `cargo build-sbf` compile the crate as plain Rust with zero
verification overhead. `cargo flux` sets `--cfg flux`, so the verification-only
surface (the `flux_rs::defs!` logic block and the `#[extern_spec]` type
registrations) is `#[cfg(flux)]`-gated, exactly as the Kani lane gates
`#[cfg(kani)]`.

## What is verified

Every core is machine-checked; the load-bearing safety properties all go
through:

- **The event invariant rides the `Event` type** (`#[flux_rs::invariant]`):
  `is_set(organizer)`, `0 < capacity`, `0 < price`, `0 <= sold`,
  `sold <= capacity`, and the nonlinear `capacity * price <= u64::MAX`. Because
  it is a type invariant, every core that constructs or mutates an `Event` must
  re-establish it - Flux checks this with no `requires`/`ensures` threading, the
  idiomatic win the roadmap predicted.
- **No overselling.** `record_sale` clamps the sale into the remaining headroom
  and the sold counter provably never passes capacity (the trusted setter takes
  `v: u64{v <= capacity}`, so `record_sale` must prove `sold + actual <=
  capacity` before it can write). This is the property whose deletion turns the
  run red (see non-vacuity below).
- **Frame conditions.** `record_sale` changes only `sold` (monotone up), the
  other fields pinned; `close_sales`/`close_event` change only `status`, with a
  legal `open -> closed` step (`legal_status`); `purchase` grows only `sold`.
- **Bounded, overflow-free pricing.** `payment_for` is *total* (not `Result`):
  `quantity <= capacity` and the invariant's `capacity * price <= u64::MAX` give
  `quantity * price <= u64::MAX`, and both factors are positive, so the payment
  is a valid `NonZeroU64` - no overflow branch survives.
- **The create-time economics guard is load-bearing.** `event_new` demands
  `capacity * price <= u64::MAX`, so `create_event` must run its overflow guard
  before an event can be constructed - the bound the invariant then carries for
  the account's life.

## Two walls shaped the lane

**1. Foreign structs cannot be transparent here, so `Event`/`Ticket` are
opaque.** The idiomatic target was a transparent `#[extern_spec]` that refines
each wire field (so cores read `event.sold` directly, as Verus does). Two
properties of the wire types block it at this rev: every `#[field(..)]` in a
transparent extern-spec struct must carry a refinement index (there is no
"unrefined field" form), yet `Event` has `Reserved<4>` alignment padding and a
`header: AccountHeader<Event>` field that is **self-referential** - refining it
would make `Event`'s sort recursive (the same recursion Verus needed
`accept_recursive_types` for). So `Event`/`Ticket` are modeled **opaque** with
the invariant on the type - the `core::alloc::Layout` pattern from Flux's own
`flux-core`, which is exactly `#[opaque] #[refined_by(..)] #[invariant(..)]`.
The cost: the cores cannot read or write the opaque fields directly, so state
flows through a **trusted accessor layer** - `get_capacity`/`get_price`/
`get_sold`/`is_open`/`organizer_eq` and the strong-update setters
`set_sold`/`set_status_closed`. This is trusted surface Verus did not pay
(its `external_type_specification` registered the structs transparently).

**2. `NonZeroU64` is not modeled, and Flux's own `core` specs are not visible to
this crate.** Two compounding gaps:

- `NonZeroU64` is `core::num::NonZero<T>` over the sealed, unstable
  `ZeroablePrimitive`. An `#[extern_spec]` on the struct is accepted only in its
  generic form (`impl<T> NonZero<T> where T: ZeroablePrimitive`, gated by
  `#![feature(nonzero_internals)]` - the same wall Verus hit), but Flux then
  rejects refining a generic type parameter by an `int` value ("expected
  `T::sort`"). So `NonZero` values cannot be tracked; they are bridged to refined
  `u64` through the trusted `nz_get`/`nz_from` shims and all accounting runs on
  `u64`, where Flux is precise.
- More consequentially, `flux-core`'s refinement specs for the standard library
  - `checked_add`/`checked_sub`/`checked_mul` on `u64`, and the `is_ok`/`is_some`
  refinements on `Result`/`Option` - **are not applied to a downstream crate**
  at this rev. A probe confirmed it: `Result<(), ()>[b]` and
  `Option<i32>[b]` fail to sort-check in this crate (while a *local*
  `#[refined_by(..)]` enum indexes fine), and `flux_rs::assert(new_sold ==
  sold + actual)` after `sold.checked_add(actual)` cannot be proved. So the
  cores cannot use the `checked_*` refinements, and signatures cannot refine a
  `Result`'s discriminant.

The consequences, all honest costs recorded below: arithmetic uses Flux's
*built-in* `+`/`-`/`*` (whose values it does track, discharging the bounds from
the invariant) instead of `checked_*`; the create-time overflow *detection* is a
trusted `mul_fits` shim (relating `checked_mul(..).is_some()` to `a*b <=
u64::MAX`); the one nonlinear step and the payment-positivity step are trusted
`mul_le_mono`/`mul_pos` lemmas (the analog of Verus's `by (nonlinear_arith)`);
and the cores refine their **`Ok` payload type** (`Result<Event{v: ..}, _>`)
rather than the `Result` discriminant, so the `Ok <=> condition` and "on error,
unchanged" clauses the deductive lanes state are dropped.

## Paradigm contrasts

- **Specs are types; the invariant rides the type.** Where Creusot/Kani/Verus
  attach `requires`/`ensures` (as attributes or inline), Flux puts the event
  invariant on the `Event` type once (`#[flux_rs::invariant]`) and it is assumed
  on every `Event` value and re-checked on every construction - no per-function
  precondition threading. This is the ergonomic payoff the roadmap predicted, and
  it landed even with `Event` opaque.
- **Near-zero annotation, but foreign-type-heavy code pays it back in trusted
  shims.** The cores themselves carry only their signatures - no loop invariants
  (they are loop-free), no proof blocks. But because Flux models neither the wire
  structs' fields nor `NonZeroU64` nor `core`'s method specs here, the lane's
  *trusted* surface (18 items) is the largest of the wired lanes: the accessor
  layer, the `NonZero` bridges, three arithmetic lemmas/guards, the two
  constructors, and the shells. Refinement types moved the cost from annotation
  volume to trusted foreign-type surface.
- **Plain-Rust tooling works everywhere - the flip side of Verus.** Flux's
  annotations are ordinary attribute macros and `defs!`, so `rustfmt`, `clippy`
  (`-D warnings`), and rust-analyzer all reach the whole crate unchanged - no
  `verusfmt` analog, no macro-interior blind spot. No `#[allow]`s survive; the
  one pedantic nudge was `payment_for` being total tripping `unnecessary_wraps`
  until it was made to return `u64`.
- **Failures debug in-source, in rustc style.** Like Verus (and unlike Creusot's
  Why3-IDE loop), Flux reports an unproved obligation as a compiler diagnostic
  with a source span ("a postcondition cannot be proved", pointing at both the
  failing return and the clause). With sub-second checking the diagnose-fix loop
  stays in the editor; the whole lane was brought up this way, including probing
  the `Result`/`checked_*` visibility gaps with throwaway `flux_rs::assert`s.
- **`--cfg flux`, like Kani.** `cargo flux` sets `--cfg flux`, so the
  verification-only surface is `#[cfg(flux)]`-gated (`cfg(flux)` registered in
  `unexpected_cfgs`) - the Kani model, not the Verus feature model. But the
  `flux_rs` dep itself is unconditional (`[target.'cfg(flux)'.dependencies]`
  does not activate under cargo-flux), opting in via `[package.metadata.flux]
  enabled = true`.

## Findings

- **Trusted surface (the honest cost column): 18 items, the largest so far.**
  The opaque wire types force an accessor/mutator layer Verus got for free from
  transparent registration (`get_*`, `is_open`, `organizer_eq`, `set_sold`,
  `set_status_closed`); `NonZeroU64` is bridged by `nz_get`/`nz_from` (the
  analog of Verus's `nz_*`); the nonlinear `capacity*price` monotonicity, the
  payment positivity, and the create-time overflow test are the trusted
  `mul_le_mono`/`mul_pos`/`mul_fits` lemmas (Flux's `core` specs being absent
  here, even `checked_mul`'s bound is trusted); `event_new`/`ticket_new` build
  the opaque structs; and the `instructions/` shells are trusted like every
  lane. Verus's opaque `is_set` + `Address` equality carry over unchanged.
- **The nonlinear bound is unbounded, like Verus/Creusot - but trusted, not
  proved.** No `capacity <= u8` cap (contrast Kani). `record_sale`'s
  `actual * price <= u64::MAX` and `payment_for`'s product bound come from the
  `mul_le_mono` lemma over `int`. Verus *proves* the same step with one
  `by (nonlinear_arith)` and Creusot's solver ensemble proved it with nothing in
  the sources; Flux (here, without `core`'s specs) *trusts* the one-line lemma -
  net, its nonlinear handling sits with Creusot's `ax_*` era, a notch below
  Verus.
- **Refinement of foreign/std types is where the tool fought hardest.** The
  transparent-struct path (recursion + mandatory field indices), the generic
  `NonZero` refinement (`expected T::sort`), and the invisible `core`
  `Result`/`Option`/`checked_*` specs are three independent limits at rev
  `1d98676`; each is documented inline in `domain/extern_specs.rs`. The parts
  Flux does natively - `int`/`bool`/`u64` refinements, built-in arithmetic,
  opaque structs with `#[invariant]`, `&mut` strong updates - are clean and
  terse.
- **Non-vacuity by mutation test.** Dropping `record_sale`'s clamp (selling the
  full `requested` regardless of remaining capacity) turns the run red: the
  trusted setter's `v <= capacity` precondition becomes unprovable, so the
  no-oversell guarantee is demonstrably load-bearing, not vacuous.
- **Behavior parity is executable.** The lane-agnostic client Mollusk suite
  passes against `ticketing_flux.so` (`TICKETING_PROGRAM_NAME=ticketing_flux`) -
  the same 7 tests that pass for the other three lanes.

## Deferred / not modeled

- **Exact functional postconditions the deductive lanes state.** The exact
  clamp value (`actual == min(requested, remaining)`), the `Ok <=> condition`
  discriminant characterization, and "on error, the event is unchanged" are
  dropped: the first needs `NonZeroU64` value tracking, the last two need
  `Result` refinement - neither available here. What survives is the safety
  core: invariant preservation, no overselling, bounded clamp, frames, legal
  transitions.
- **Arithmetic-overflow *checking* is not enabled.** `#![flux::opts(check_overflow
  = ..)]` is a `flux::` tool inner-attribute that a stable downstream crate
  cannot carry cleanly; the no-overflow property is instead established through
  the explicit bounds (`sold + actual <= capacity <= u64::MAX`, `quantity *
  price <= u64::MAX`) rather than Flux's built-in overflow pass.
- **`Ticket` is refined only by its two addresses** (for the set-address
  invariant); `quantity`/`paid`/slots are not tracked, so the ticket-consistency
  clause (`ticket.quantity <= event.sold`) is not surfaced.
- **The dispatcher byte front** (the panic-freedom property Kani proves) stays a
  trusted shell, as in the Creusot/Verus lanes.
- **The account transition guard** (`scripts/check-ticketing-transitions.sh`)
  stays Creusot-only; the Flux flows express the transition through the `defs!`
  frame predicates (`same_except_status(..)` + `legal_status(..)`) rather than
  a single named `is_legal_transition_from(*arg)` clause, so generalizing the
  guard to this lane would need a syntax change, not just a path change.
