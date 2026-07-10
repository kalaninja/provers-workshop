# The provers

The living roadmap of the workshop: which Rust verification tools go onto the
ticketing program's verified cores, in what order, and what happened with each.
The fixed target and the ground it stands on are described in the
[README](../README.md); this file tracks the variable part.

Statuses and tool facts below are point-in-time. **Last reviewed: 2026-07-08.**
Re-verify a tool's state (releases, toolchain pins, open blockers) before
starting its lane.

## Roster

| # | Prover | Paradigm | Status |
| --- | --- | --- | --- |
| 1 | [Creusot](https://github.com/creusot-rs/creusot) | deductive, translated to Why3 + SMT | wired up (the baseline) |
| 2 | [Kani](https://github.com/model-checking/kani) | bounded model checking (CBMC) | wired up |
| 3 | [Verus](https://github.com/verus-lang/verus) | SMT over a verification dialect of Rust | planned |
| 4 | [Flux](https://github.com/flux-rs/flux) | refinement (liquid) types | planned |
| 5 | [Aeneas](https://github.com/AeneasVerif/aeneas) | functional translation to Lean, interactive proofs | planned |
| 6 | [hax](https://github.com/cryspen/hax) | translation to F* / Rocq / Lean | planned |
| 7 | [VeriFast](https://github.com/verifast/verifast) | separation logic | on hold - target-fit question |
| 8 | [Prusti](https://github.com/viperproject/prusti-dev) | deductive, Viper backend | on hold - likely toolchain-blocked |

The order runs from cheap integration to expensive, and every step adds a new
paradigm instead of a second data point in one we already have: bounded model
checking (Kani), an SMT-checked dialect (Verus), refinement types (Flux), then
translation into an interactive theorem prover (Aeneas, and hax to compare
pipelines). Kani first also produces the second column of the comparison table
fastest. The two riskiest entries sit at the tail: VeriFast because its sweet
spot may not be this repo's verified layer at all, Prusti because it likely
cannot build the workspace anymore.

## Ground rules - what every lane must provide

Distilled from the Creusot lane; this is the template a new prover copies.

- Each prover gets its own program crate `crates/ticketing/<prover>` over the
  shared `ticketing-interface`. The program's *behavior* stays identical across
  prover crates - same instructions and decisions over the same wire state - but
  the code may be written the way that prover is best used, not forced into a
  shared source (Creusot routes foreign-type ops through `#[trusted]` axiom
  shims; Kani calls std directly, since it models those types precisely - the
  "Code bending" axis tracks exactly this). Spec and proof material stays
  cfg-gated (`#[cfg(<prover>)]` items / `#[cfg_attr(<prover>, ...)]` attributes,
  mirroring `cfg(creusot)`) so the crate's normal and SBF builds compile with
  zero verification overhead.
- The new cfg is registered in the root `Cargo.toml` `unexpected_cfgs`
  check-cfg list, next to `cfg(creusot)`.
- Prover-only dependencies go under `[target.'cfg(<prover>)'.dependencies]` in
  that prover's crate, the way `creusot-std` is wired in `ticketing-creusot`
  today. The interface crate stays verifier-free.
- A lane of its own: `mise.<prover>.toml` selected by `MISE_ENV=<prover>`, a
  `<prover>-tooling` Dockerfile stage, an image variable in `just/common.just`,
  a `build-<prover>` recipe in `just/tooling.just`, and a `just/<prover>.just`
  module with at least `verify`. `scripts/docker/run-tooling.sh` is touched
  only if the tool needs special HOME/XDG plumbing (Creusot does, for opam).
- Committable proof artifacts, where the tool has them, live in a sibling of
  `verif/`, with a deterministic `replay`-style recipe to re-check them.
- A `<prover>_changed` glob in `scripts/hooks/pre-push` scopes the hook.
- `scripts/check-ticketing-transitions.sh` (every `&mut` core argument must
  carry an `is_legal_transition_from` ensures clause) scans the prover crate's
  `src/flows/` and keeps guarding for free as long as the prover keeps that
  predicate name.

## What we compare

The README's four axes plus the operational ones that only show up after
running a tool for a while:

1. **Trusted surface** - what must be axiomatized or marked trusted for the
   proof to go through.
2. **Contract legibility** - how the same contract reads in the tool's syntax.
3. **Completeness** - full proof or bounded, and structural limits (e.g.
   monomorphic code only).
4. **Proof / replay speed** - wall-clock to prove from scratch and to re-check.
5. **Artifact story** - committed, deterministically replayable proof
   artifacts, or re-verification from scratch every run.
6. **Code bending** - how far the cores must bend to fit the tool.
7. **Toolchain fit** - image weight, pinning story, hermetic-build friction.
8. **Maintenance** - release cadence, bus factor, production users.

Columns fill in as lanes land.

| Axis | Creusot | Kani | Verus | Flux | Aeneas | hax | VeriFast | Prusti |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Trusted surface | `instructions/` shells; `ax_*` axioms + opaque `int`/`is_set` for `NonZeroU64`/`Address` | `instructions/` shell interiors past account parsing - the dispatcher byte front (discriminator + args parse + account-count checks) is verified panic-free; foreign-type ops are plain std (`checked_mul`, `==`, ...) + a concrete `is_set`, all verified, no axioms | - | - | - | - | - | - |
| Contract legibility | `requires`/`ensures` attributes + pearlite (`@`, `*x`/`^x`, `Int`) | function contracts on the fns (`#[cfg_attr(kani, kani::requires/ensures)]`, `-Z function-contracts`) checked by `#[kani::proof_for_contract]`; `old(*e)`/`*e` for pre/post - close to Creusot's clauses | - | - | - | - | - | - |
| Completeness | full deductive proof | bounded model check; the cores are loop-free (no unwinding bounds) but `capacity` bounded to `u8` to keep the nonlinear `capacity*price` tractable; `kani::cover!` guards the bounded domain against vacuous passes; the dispatcher harness bounds its parser loops with `#[kani::unwind]` | - | - | - | - | - | - |
| Proof / replay speed | tbd (time `just creusot verify` / `replay`) | ~310s wall-clock from scratch (`-j` parallel harnesses; bounded by the dispatcher's ~300s, `purchase` ~210s, others 32-54s); no replay | - | - | - | - | - | - |
| Artifact story | 23 `proof.json` sessions under `verif/`, deterministic replay in pre-push | none committed - re-verifies from scratch every run | - | - | - | - | - | - |
| Code bending | none in core bodies; extension-trait plumbing for foreign impls | cores call std directly (no axiom shims); predicates are `#[cfg(kani)]` bool fns on hand-written `*Ops` traits; PODs built field-by-field | - | - | - | - | - | - |
| Toolchain fit | own nightly (2026-04-21) + opam-built Why3/solvers; the heaviest image | self-contained bundle (kani-verifier 0.67.0 + CBMC/kissat) via `cargo kani setup`; light image | - | - | - | - | - | - |
| Maintenance | active, research-led (v0.12.0 pinned) | active (AWS); drives the std verification campaign | - | - | - | - | - | - |

## Prover notes

### 1. Creusot - wired up

[Creusot](https://github.com/creusot-rs/creusot) translates Rust carrying
pearlite contracts into Why3; `why3find` discharges the verification
conditions with SMT solvers (alt-ergo, z3, cvc5, cvc4 - see `why3find.json`,
with `compute_specified` + `split_vc` tactics).

What is wired: contracts on the flow cores and domain ops
(`#[cfg_attr(creusot, requires(...)/ensures(...))]`), pure predicates under
`#[cfg(creusot)]`, trusted axioms for foreign types, 23 committed proof
sessions under `verif/`, `just creusot verify / replay / verify-ide`, and the
pre-push replay gate. The full walkthrough is in the README's "Verified cores"
section.

Lessons that set the baseline for comparison:

- Foreign types need an explicit trusted computing base: `NonZeroU64` and
  `Address` are modelled in `src/domain/` by `#[trusted]` shims
  (`ax_checked_mul`, `ax_eq`, ...) tied to opaque logic functions (`int`,
  `is_set`).
- The orphan rule pushes foreign-type contracts through crate-local extension
  traits; the `creusot_extension` proc-macro keeps trait and impl attributes
  from drifting.
- Proof sessions replay deterministically: reviewable artifacts, no solver
  roulette in CI.

### 2. Kani - wired up

[Kani](https://github.com/model-checking/kani) (AWS) compiles MIR to goto-C
and bit-blasts it through CBMC. It is the workhorse of the
[Rust standard library verification campaign](https://github.com/model-checking/verify-rust-std).

What is wired: a full `ticketing-kani` program crate over the shared
`ticketing-interface` - same behavior as `ticketing-creusot`, written the Kani
way (std called directly, no axiom shims) - with a `cfg(kani)` spec layer:
domain predicates as ordinary `#[cfg(kani)]` boolean methods on hand-written
`*Ops` extension traits, and function contracts on each flow core
(`#[cfg_attr(kani, kani::requires/ensures)]` + `#[kani::modifies]`, mirroring the
Creusot clauses) checked by `#[kani::proof_for_contract]` harnesses under
`-Z function-contracts` (kept on the CLI: cargo-kani can default flags only
from the workspace-root manifest, not per crate); each harness ends in
`kani::cover!` reachability checks on the interesting outcomes. Built via a
`kani-tooling` Docker stage (`cargo install kani-verifier` 0.67.0 +
`cargo kani setup`); `just kani verify` runs the four harnesses in parallel
(`-j`) and passes them all (0 failures, all covers satisfied). The fourth is a
plain `#[kani::proof]` over the dispatcher byte front: every payload up to one
past the largest layout, with no accounts - always an error, never a panic. No
replay - Kani re-verifies from scratch (wall-clock ~310s, bounded by that
harness; `purchase` ~210s).

Why second: a genuinely different paradigm at the lowest integration cost. The
gating maps one-to-one onto the existing scheme (Kani even sets `--cfg kani`
itself), and the cores are loop-free and monomorphic, so there are no
loop-unwinding bounds. The catch turned out to be arithmetic, not loops (below).

Findings:

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
- **Nonlinear arithmetic is the real cost, not loops.** `purchase` must prove
  `quantity * price <= capacity * price <= u64::MAX` (multiplication
  monotonicity) so its post-mutation error path is unreachable, and CBMC
  bit-blasts nonlinear 64-bit arithmetic poorly - `close_event`, with only the
  invariant's single product, ran ~5 min at full width. Bounding `capacity` to
  `u8` (price stays full 64-bit, so the `u64` overflow boundary is still reached)
  plus the `kissat` backend brought the plain assume/assert run to ~85s (the
  shipped contract harnesses cost more - see below). The loop-free "bounded
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

### 3. Verus - planned

[Verus](https://github.com/verus-lang/verus) checks a verification dialect of
Rust against Z3, with specs and proofs written in the language itself. The
[std-verification experience reports](https://arxiv.org/abs/2510.01072) treat
it as the current state of the art for proving deep functional properties of
Rust.

Why third: the strongest deductive competitor to the baseline - and the
integration friction is the experiment. Code must live inside the `verus!{}`
macro, so either `vstd`/`builtin` enter the normal and SBF builds (the macro
erases ghost code under plain rustc) or the cores get re-stated in a parallel
module, straining the byte-identical premise. How far the cores (or the shells
around them) must bend is exactly what this lane measures.

Notes for when its turn comes: `NonZeroU64`/`Address` go through
`assume_specification` / external type specs, so the axiom analog remains;
releases pin their own rustc; proofs are fast SMT queries with no committed
session artifacts.

### 4. Flux - planned

[Flux](https://github.com/flux-rs/flux) refines Rust's types with logical
predicates (liquid types) and checks them by SMT with inference. Annotation
overhead is near zero compared to contract-based tools and checking takes
seconds; it has been used to
[verify process isolation in Tock OS](https://events.ucsc.edu/event/cse-colloquium-flux-refinement-types-for-verified-rust-systems/).

Why fourth: the opposite end of the ergonomics spectrum before diving into
ITPs. Specs become refined signatures (`#[flux::sig(...)]`) and struct
invariants (`#[flux::invariant]`, e.g. `sold <= capacity`) instead of
requires/ensures clauses - a different way of *saying* the same contracts, not
just proving them.

Risk to probe early: the event invariant is nonlinear
(`capacity * price <= u64::MAX`) and liquid inference leans on linear
arithmetic; this may need a trusted lemma, which goes straight into the
trusted-surface column as an honest cost.

### 5. Aeneas - planned

[Aeneas](https://github.com/AeneasVerif/aeneas) extracts Rust through
[Charon](https://github.com/AeneasVerif/charon) into a pure functional model
and proves it interactively - primarily in Lean (F*, Rocq and HOL4 backends
exist). Microsoft uses it to
[verify the Rust port of SymCrypt](https://lean-lang.org/use-cases/aeneas/).

Why fifth: the first interactive prover in the series. Proofs are written by
hand in Lean (`progress`, `scalar_tac`, ...), a different cost model from
everything before it; the payoff is expressiveness and the best artifact story
of all - the proofs are committed Lean code, replayed by `lake build`.

Costs and risks: the tooling image gains a Lean toolchain (elan) plus
OCaml-built charon/aeneas, so expect Creusot-like image weight; Charon has to
extract the crate graph with pinocchio/bytemuck in it, or those get marked
opaque - spike the extraction before committing to the lane.

### 6. hax - planned

[hax](https://github.com/cryspen/hax) (Cryspen) translates a large subset of
Rust into F*, Rocq or Lean; contracts are written in Rust as
`hax_lib::requires/ensures` attributes - the closest ITP-flavored analog to
this repo's cfg-gated attribute style. Production precedent:
[libcrux](https://github.com/cryspen/libcrux)'s verified ML-KEM, shipped in
Firefox and Signal.

Why sixth, right after Aeneas: same paradigm quadrant (translate, then prove
in an ITP), so the experience transfers and the two pipelines can be compared
head-to-head on identical cores. Extra material for the trusted-surface axis:
the April 2026
["Verification Facade" critique](https://symbolic.software/blog/2026-04-07-cryspen-hax/)
of semantic gaps in the hax pipeline - a live case study in what the
*translation itself* asks you to trust.

### 7. VeriFast - on hold

[VeriFast](https://github.com/verifast/verifast) is a separation-logic
verifier; in the std campaign it handles the most pointer-intensive unsafe
modules, and it supports concurrent code.

The hold is a target-fit question, not a quality one: this repo's verified
layer is safe, pure, loop-free functions, and separation logic's strengths sit
elsewhere. Its natural target here would be the *trusted shells and the
zero-copy wire casts* (the bytemuck layer) - extending the verified surface
rather than re-proving the same contracts. Whether to widen the workshop's
scope that way is the decision to make when its turn comes.

### 8. Prusti - on hold

[Prusti](https://github.com/viperproject/prusti-dev) (ETH Zurich) verifies
annotated Rust through the Viper infrastructure. Kept on the roster
deliberately, but development has stalled and it is absent from the std
campaign; its pinned toolchain predates edition 2024, so it most likely cannot
build this workspace (edition 2024, resolver 3) at all.

Before any real investment: a cheap spike - point it at the workspace and
record the exact failure here. If it ever unblocks, its value is a
syntax/ergonomics comparison against Creusot, since both are deductive and
annotation-based.

## Considered and dropped

- **MIRAI** - abstract interpreter from Meta; archived, no development.
- **RefinedRust** - research prototype; requires Rocq/Iris expertise and aims
  at unsafe code.
- **coq-of-rust** - Rust-to-Rocq translation; low activity, quadrant covered
  by hax/Aeneas.
- **crux-mir** - symbolic testing; marginal maintenance, overlaps Kani.
- **goto-transcoder (ESBMC)** - too early.
- **Miri, proptest/bolero, fuzzing** - valuable, but not provers; out of scope
  by the workshop's definition.

## Status log

- **2026-07-10** - Kani surface widened to the dispatcher byte front: a plain
  `#[kani::proof]` harness proves `process_instruction` error-only and
  panic-free for every payload up to one past the largest layout, with no
  accounts. It needs the lane's only `#[kani::unwind]` (`BoundedString`
  validation loops over symbolic-length slices, structurally unbounded to
  CBMC) and is now the slowest harness (~300s). `just kani verify` runs
  harnesses in parallel (`-j`, wall-clock ~310s); the notes pick up
  concrete-playback and the `stub_verified` composition caveat.
- **2026-07-09** - Kani harness hygiene pass: `kani::cover!` reachability
  checks in every harness (both result arms plus the `purchase` sale clamp;
  all satisfied), and arbitrary builders moved onto Kani's built-in `Arbitrary`
  (`NonZeroU8`/`NonZeroU64`) and a `Contiguous`-drawn status (`bytemuck` added
  under `[target.'cfg(kani)'.dependencies]`). Verify now ~290s (`purchase`
  211s).
- **2026-07-08** - Kani lane wired up: `ticketing-kani` - same program as the
  Creusot lane, written the Kani way (std directly, no axiom shims) - with a
  `cfg(kani)` spec layer and three function contracts checked by
  `#[kani::proof_for_contract]` (`-Z function-contracts`); `just kani verify`
  passes (0 failures, ~190s). Finding: full 64-bit nonlinear `capacity*price` is
  intractable for CBMC, so `capacity` is bounded to `u8` (price full-width;
  overflow still reached).
- **2026-07-08** - Kani lane scaffolded: `mise.kani.toml`, `kani-tooling` Docker
  stage, `just kani` (`version`/`verify`), `run-tooling.sh` KANI_HOME plumbing, a
  narrow pre-push glob, `cfg(kani)` registered in check-cfg, and a `ticketing-kani`
  crate with one smoke `#[kani::proof]`. Cores/contracts pending.
- **2026-07-08** - model set to one program crate per prover over the shared
  `ticketing-interface`; the Creusot baseline crate renamed `ticketing-program`
  -> `ticketing-creusot` (SBF artifact `ticketing_creusot.so`, proof sessions
  under `verif/ticketing_creusot_rlib/`).
- **2026-07-08** - roster fixed at eight tools, order set (Kani next), this
  document created. Creusot baseline already wired (23 proof sessions under
  `verif/`).
