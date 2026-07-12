# Kani lane

Lane 2 of [the prover roadmap](provers.md). Status: **wired up**. Facts are
point-in-time; last reviewed 2026-07-10.

[Kani](https://github.com/model-checking/kani) (AWS) compiles MIR to goto-C
and bit-blasts it through CBMC. It is the workhorse of the
[Rust standard library verification campaign](https://github.com/model-checking/verify-rust-std).

Why second in the roster: a genuinely different paradigm at the lowest
integration cost. The gating maps one-to-one onto the existing scheme (Kani
even sets `--cfg kani` itself), and the cores are loop-free and monomorphic,
so there are no loop-unwinding bounds. The catch turned out to be arithmetic,
not loops (below).

## What is wired

A full `ticketing-kani` program crate over the shared
`ticketing-interface` - same behavior as `ticketing-creusot`, written the Kani
way (std called directly, no axiom shims) - with a `cfg(kani)` spec layer:
domain predicates as ordinary `#[cfg(kani)]` boolean methods on hand-written
`*Ops` extension traits, and function contracts on each flow core
(`#[cfg_attr(kani, kani::requires/ensures)]` + `#[kani::modifies]`, mirroring the
Creusot clauses) checked by `#[kani::proof_for_contract]` harnesses under
`-Z function-contracts` (kept on the CLI: cargo-kani can default flags only
from the workspace-root manifest, not per crate); each harness ends in
`kani::cover!` reachability checks on the interesting outcomes. Built via a
`kani-tooling` Docker stage (kani-verifier 0.67.0 via mise's cargo backend +
`cargo kani setup`); `just kani verify` runs the four harnesses in parallel
(`-j`) and passes them all (0 failures, all covers satisfied). The fourth is a
plain `#[kani::proof]` over the dispatcher byte front: every payload up to one
past the largest layout, with no accounts - always an error, never a panic. No
replay - Kani re-verifies from scratch (wall-clock ~310s, bounded by that
harness; `purchase` ~210s).

## Paradigm contrasts

- **The solver is asked the opposite question.** Creusot builds verification
  conditions and asks the SMT solvers to *prove* them - the program conforms
  to the spec for all inputs. Kani unrolls the program (bounded) into a
  bit-level formula and asks the SAT solver to *find one execution that
  violates a check*; `UNSAT` - no such execution exists within bounds - is the
  pass verdict. That single inversion explains the lane's shape: bounds are
  what make the question decidable (`capacity <= 255`, `#[kani::unwind]`), a
  failure comes back as a concrete input trace (what `--concrete-playback`
  replays as a unit test), and a pass is just a verdict with no proof object
  behind it - hence the empty artifact story and from-scratch reruns.
- **Verification is harness-scoped; the trusted boundary is implicit.**
  Creusot translates the whole crate - every function is an obligation unless
  explicitly opted out, which is why the shell module carries
  `#[cfg_attr(creusot, trusted)]`. Kani has no crate-wide obligations:
  exactly the code reachable from a `#[kani::proof*]` harness is checked, and
  everything else is silently out of scope, no marker needed. The same fact
  cuts both ways: widening coverage never touches program code (the dispatcher
  harness landed without editing `instructions/`), but nothing in the code
  shows where verification ends - the boundary lives in the harness set, and
  shrinking it is invisible in review (the `kani::cover!` guards are the
  counterweight).
- **The proof layer stays out of the program text.** Harnesses are ordinary
  functions in a `#[cfg(kani)]`-only module (`proofs.rs`); the program modules
  carry nothing Kani-specific beyond the cfg-gated contracts and predicate
  methods - and even those are a choice made for apples-to-apples with
  Creusot, since plain assume/assert harnesses would check the same properties
  with zero annotations on the code. Creusot cannot make that trade: pearlite
  specs, logic functions, and `#[trusted]` markers necessarily live in the
  code they describe.

## Findings

- **Smaller trusted surface than Creusot.** Creusot's `ax_*` shims and the
  opaque `int`/`is_set` are *axioms*. The Kani lane drops that shim layer
  entirely - it calls std directly (`NonZeroU64::checked_mul`, `min`, `==`) and
  defines `is_set` concretely (`self != Address::default()`), so the model
  checker verifies all of it. Only the `instructions/` shells stay outside the
  proof - and since the dispatcher harness below, only their interiors past
  account parsing.
- **Byte parsers are BMC home turf.** A plain `#[kani::proof]` harness feeds
  `process_instruction` every payload up to one byte past the largest wire
  layout, with an empty account list: it always errors and never panics, which
  verifies the discriminator match, both args parsers, and the account-count
  rejection - a slice of the shells Creusot cannot reach cheaply. The
  runtime-touching interiors (PDA creation, clock, CPI, persistence) remain
  the trusted surface. Two costs: the lane's only `#[kani::unwind]`
  (`BoundedString` validation iterates a slice whose bounds are symbolic
  payload bytes - structurally unbounded to CBMC, symex spins forever without
  the bound), and the lane's slowest harness (~300s).
- **Nonlinear arithmetic is the real cost, not loops.** The event invariant
  carries `capacity * price <= u64::MAX` - the worst-case payout fits - from
  creation for the account's whole life, and `purchase` must prove
  `quantity * price <= capacity * price <= u64::MAX` (multiplication
  monotonicity; the `record_sale` clamp gives `quantity <= capacity`) so its
  post-mutation error path is unreachable. That is hostile ground for CBMC:
  bit-blasting unrolls a 64x64 multiplication into a quadratic
  partial-product circuit (a hardware multiplier in SAT clauses), a
  symbolic-times-symbolic product is the pathological case, and on a pure SAT
  backend there is no arithmetic theory to appeal to - monotonicity cannot be
  proved symbolically, only ground through the bits. Telling symptom:
  `close_event` multiplies nothing (it flips a status byte), yet ran ~5 min at
  full width because its contract drags the invariant's product through pre-
  and post-state. Bounding `capacity` to `u8` tames every product at once -
  each has `capacity`, or a quantity clamped by it, as one operand - while
  `price` stays full 64-bit, keeping the `u64` overflow boundary reachable;
  with the `kissat` backend that brought the plain assume/assert run to ~85s
  (the shipped contract harnesses cost more - see below). The alternatives all
  cost more: `stub_verified` composition does not help, because the
  monotonicity step links the invariant's product to the payment's product
  *across* two contracts, so it stays in the caller's formula whatever is
  stubbed; an unverified `kani::stub` of the lemma would remove the wall but
  reintroduces exactly the axiom layer this lane exists to avoid (and unlike
  Creusot's, unproved); weakening the spec (not claiming the error path
  unreachable) proves less than the baseline. Bounding is the only option that
  keeps both zero axioms and spec parity - solver shopping (bitwuzla, cvc5,
  CBMC's `--refine`) might still shave constants, not the complexity class.
- **The contract pins the overflow guard.** The point of all that machinery:
  you cannot forget `capacity.checked_mul(price_lamports).ok_or(...)?` in
  `create_event`. Deleting it fails the harness two ways at once
  (mutation-tested): the `Ok` arm's ensures breaks - `is_initial` demands the
  invariant, and overflowing inputs stay reachable under the `u8` bound
  because `price` is full-width - and the `Err`-arm cover ("overflow rejection
  is reachable") goes unsatisfiable. From creation on, every mutation's
  `ensures(post.invariant())` carries the bound forward, which is what makes
  `payment_for`'s overflow branch provably unreachable in `purchase`.
  Forgetting the check is not a silent bug but a red verification run. The loop-free "bounded
  caveat evaporates" claim holds for unwinding but not for integer width:
  Creusot's SMT discharges the same nonlinear bound unbounded. Sharpest contrast
  in the comparison so far.
- **A bounded domain needs vacuity guards.** With `capacity` clamped and every
  `requires` assumed, an over-constrained input space would let the `ensures`
  pass vacuously. Each harness therefore ends with `kani::cover!` checks that
  the interesting outcomes stay reachable - both result arms everywhere, plus
  the sale clamp in `purchase`; an unsatisfiable cover fails the run. Covers
  are extra SAT queries against the same formula: they lifted `purchase` from
  144s to ~211s and the whole (sequential) run from ~190s to ~290s. One
  composition caveat: the contracts state unbounded `requires` but are checked
  over `capacity <= 255` - harmless while nothing consumes them, but a future
  caller proof using `stub_verified(purchase)` would assume the unchecked gap.
- **No `kani::Arbitrary` for the foreign PODs.** The orphan rule blocks
  `impl kani::Arbitrary for Event`, so the harnesses build `Event`/`Ticket`
  field-by-field. The leaves still come from Kani's library: built-in
  `Arbitrary` impls for `NonZeroU8`/`NonZeroU64` (capacity draws through the
  typed `NonZeroU8` domain), and enum discriminants from the full `Contiguous`
  range (`kani::any_where` + `from_integer`), which picks up future status
  variants automatically; `bytemuck` rides along under
  `[target.'cfg(kani)'.dependencies]` for that trait.
- **Function contracts read like Creusot's, at a cost.** The clauses sit on the
  functions as `#[cfg_attr(kani, kani::requires/ensures)]` + `#[kani::modifies]`,
  checked by `#[kani::proof_for_contract]` - the closest apples-to-apples with
  Creusot's `requires`/`ensures`. Two frictions: `-Z function-contracts` is
  unstable, and on a `&mut` argument the postcondition must dereference into
  owned locals (`let post = *event; let pre = old(*event);`) because Kani's
  post-state rewrite doesn't see through `event.method()`. The `modifies` frame
  reasoning also makes verification ~2.5x slower than a plain assume/assert
  harness (`purchase` 144s vs 54s, measured before the cover checks).
- **Frame conditions use plain `==`.** `Event`/`Ticket` gain a `PartialEq`/`Eq`
  derive in the shared `ticketing-interface` so the frame reads
  `post == Event { .. }` like Creusot's pearlite `==` (Rust has no logical
  equality, so `==` here is `PartialEq::eq`). `just creusot replay` confirms the
  derive is inert for the Creusot lane — its pearlite `==` is structural,
  independent of any `PartialEq` impl.
- **Counterexamples replay as unit tests.** `-Z concrete-playback` turns a
  failing check into a runnable `#[test]` seeded with the concrete inputs
  (`--concrete-playback=print|inplace`) - the lane's debugging loop when a
  harness regresses; nothing to commit while everything passes.
