# The provers

The living roadmap of the workshop: which Rust verification tools go onto the
ticketing program's verified cores, in what order, and what happened with each.
The fixed target and the ground it stands on are described in the
[README](../README.md); this file tracks the variable part. It is the index:
roster, ground rules, the running comparison, and a short summary per lane -
each wired lane gets a full report of its own next to this file
([creusot.md](creusot.md), [kani.md](kani.md)), and the dated chronology lives
in [status-log.md](status-log.md).

Statuses and tool facts below are point-in-time. **Last reviewed: 2026-07-08.**
Re-verify a tool's state (releases, toolchain pins, open blockers) before
starting its lane.

## Roster

| # | Prover | Paradigm | Status |
| --- | --- | --- | --- |
| 1 | [Creusot](https://github.com/creusot-rs/creusot) | deductive, translated to Why3 + SMT | wired up (the baseline) - [report](creusot.md) |
| 2 | [Kani](https://github.com/model-checking/kani) | bounded model checking (CBMC) | wired up - [report](kani.md) |
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
- A `docs/<prover>.md` lane report (what is wired, findings, costs), kept
  short here as a summary; this page stays the index.

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
conditions with SMT solvers. The baseline lane: contracts on the flow cores
and domain ops, trusted axioms for the foreign types, 23 committed proof
sessions under `verif/` with deterministic replay in the pre-push hook.

Full report: [creusot.md](creusot.md).

### 2. Kani - wired up

[Kani](https://github.com/model-checking/kani) (AWS) bit-blasts MIR through
CBMC - the paradigm inverts: the SAT solver hunts for one violating execution
within bounds instead of proving conformance for all inputs, and verification
covers exactly what the harnesses reach. Four harnesses (three flow-core
contracts plus the dispatcher byte front, all `kani::cover!`-guarded) pass in
~310s wall-clock (`-j`); nothing to commit - every run re-verifies from
scratch. Headline lessons: nonlinear `capacity*price` forced a `u8` capacity
bound, and the trusted surface is smaller than Creusot's (no axioms) but its
boundary is implicit in the harness set.

Full report: [kani.md](kani.md).

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
