# The provers

The living roadmap of the workshop: which Rust verification tools go onto the
ticketing program's verified cores, in what order, and what happened with each.
The fixed target and the ground it stands on are described in the
[README](../README.md); this file tracks the variable part. It is the index:
roster, ground rules, the running comparison, and a short summary per lane -
each wired lane gets a full report of its own next to this file
([creusot.md](creusot.md), [kani.md](kani.md)), and the dated chronology lives
in [status-log.md](status-log.md).

Statuses and tool facts below are point-in-time. **Last reviewed: 2026-07-17.**
Re-verify a tool's state (releases, toolchain pins, open blockers) before
starting its lane.

## Roster

| # | Prover | Paradigm | Status |
| --- | --- | --- | --- |
| 1 | [Creusot](https://github.com/creusot-rs/creusot) | deductive, translated to Why3 + SMT | wired up (the baseline) - [report](creusot.md) |
| 2 | [Kani](https://github.com/model-checking/kani) | bounded model checking (CBMC) | wired up - [report](kani.md) |
| 3 | [Verus](https://github.com/verus-lang/verus) | SMT over a verification dialect of Rust | wired up - [report](verus.md) |
| 4 | [Flux](https://github.com/flux-rs/flux) | refinement (liquid) types | wired up - [report](flux.md) |
| 5 | [Aeneas](https://github.com/AeneasVerif/aeneas) | functional translation to Lean, interactive proofs | wired up - [report](aeneas.md) |
| 6 | [hax](https://github.com/cryspen/hax) | translation to F* / Rocq / Lean | wired up (5/6 cores) - [report](hax.md) |
| 7 | [VeriFast](https://github.com/verifast/verifast) | separation logic | on hold - target-fit question |
| 8 | [Prusti](https://github.com/viperproject/prusti-dev) | deductive, Viper backend | blocked - edition 2024 / resolver 3 unsupported |

The order runs from cheap integration to expensive, and every step adds a new
paradigm instead of a second data point in one we already have: bounded model
checking (Kani), an SMT-checked dialect (Verus), refinement types (Flux), then
translation into an interactive theorem prover (Aeneas, and hax to compare
pipelines). Kani first also produces the second column of the comparison table
fastest. The two riskiest entries sit at the tail: VeriFast because its sweet
spot may not be this repo's verified layer at all, Prusti because its stale
Rust/Cargo toolchain cannot parse the workspace manifest.

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
- Spec coverage is measured against the Creusot reference lane and must not
  fall below it. Proving *more* is welcome; a clause may be dropped only when
  the tool genuinely cannot state it, and every such omission is recorded in
  the lane report (`docs/<prover>.md`) as an honest tool-limit cost - never
  silently.
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
| Trusted surface | `instructions/` shells; `ax_*` axioms + opaque `int`/`is_set` for `NonZeroU64`/`Address` | `instructions/` shell interiors past account parsing - the dispatcher byte front (discriminator + args parse + account-count checks) is verified panic-free; foreign-type ops are plain std (`checked_mul`, `==`, ...) + a concrete `is_set`, all verified, no axioms | opaque `is_set` + trusted `Address` eq, `nz_checked_mul`/`nz_min` wrappers (the two `NonZero` ops vstd leaves unspecced; the `NonZero` model itself is vstd's) + header/reserved ctors, plus `external_type_specification` registrations for `Event`/`Ticket`/`EventStatus`/`ProgramError` + wire types (the layer Creusot got free by translating the graph); trusted `instructions/` shells like every lane | the largest surface of the wired lanes: opaque `Event`/`Ticket` (invariant on the type) force trusted field accessors + strong-update setters - the read/write layer Verus got free from transparent registration; `nz_get`/`nz_from` bridge the unmodeled `NonZeroU64`; `mul_le_mono`/`mul_pos`/`mul_fits` are trusted because flux-core's own `checked_*`/`Result`/`Option` specs are not visible to a downstream crate here; opaque `is_set` + `Address` eq + `event_new`/`ticket_new` ctors; trusted `instructions/` shells like every lane | same *shape* as Flux (opaque `Event`/`Ticket` + trusted field accessors/setters/ctors, `nz_get`/`nz_from` bridges, `is_set`/`Address` eq - the `domain::glue` axiomatized with `@[step]`/`@[simp]` specs in Lean), but **smaller on arithmetic**: the nonlinear `capacity*price`/pricing bounds Flux trusts (`mul_le_mono`/`mul_pos`) are *proved* here, not axiomatized; the `set_sold` no-oversell precondition (`v <= capacity`) is the load-bearing trusted entry, and the glue ctor axioms carry the Creusot constructors' `requires` as hypotheses. Trusted `instructions/` shells (never extracted, via `start_from`) | committed model preamble (`Event`/`Ticket` as Lean structures dropping header/reserved; `Address` as `Nat` + `is_set`; `NonZeroU64` as a `u64` subtype so `nzVal_pos` is *proved*; `ProgramError` inductive) + `domain::glue` given concrete Lean models via `lean::replace`, so `mvcgen` unfolds them with **no accessor/setter axioms** (unlike Aeneas). Only `nzVal_nzMk` (the `nz_from` identity-on-positives bridge) stays an axiom; the nonlinear `capacity*price` and the division-gate bounds are proved. Trusted `instructions/` shells (never extracted, via the `-i` clause) | - | - |
| Contract legibility | `requires`/`ensures` attributes + pearlite (`@`, `*x`/`^x`, `Int`) | function contracts on the fns (`#[cfg_attr(kani, kani::requires/ensures)]`, `-Z function-contracts`) checked by `#[kani::proof_for_contract]`; `old(*e)`/`*e` for pre/post - close to Creusot's clauses | inline `requires`/`ensures` in `verus!{}` (specs and code in one dialect); `*old(x)`/`*final(x)` for pre/post, vstd's `@` views for `NonZero`, frame conditions as functional-update struct `==` - the closest match to Creusot's pearlite | refined-type signatures (`#[flux_rs::sig(...)]`) with the event invariant on the `Event` type (`#[flux_rs::invariant]`), so it is never threaded through a `requires`; `&mut Event[@e] ... ensures event: Event{new: ..}` for pre/post frames; the `Ok` payload is refined (`Result<Event{v: ..}, _>`) since the `Result` discriminant is not refinable from a downstream crate here | hand-written Lean 4 over the Charon/Aeneas-generated model, in a *separate file* (`TicketingAeneasProofs.lean`) - the farthest from Creusot's inline pearlite. `⦃ ⦄` total-correctness triples proved with `step`/`scalar_tac`/`simp_all`; `&mut Event` renders as a backward function returning `(.., Event)`, `?` as a `Result`-branch, so pre/post frames are stated over logical projections of the opaque `Event` | inline Rust attributes, but embedded Lean Hoare triples (`#[hax_lib::lean::after("theorem ⦃ ⦄ … := by hax_mvcgen …")]`, the `barrett`/`adc` idiom) rather than `requires`/`ensures` - the `Spec` path's `pureEnsures` rejects a `Result`-match postcondition. `&mut Event` lowers to a `RustM` tuple return with `future(event)` for the post-state; frames read the modeled `Event` structure fields directly | - | - |
| Completeness | full deductive proof; the nonlinear `capacity*price <= u64::MAX` closes with no in-code hint (Why3 + the alt-ergo/z3/cvc5 ensemble handle it) | bounded model check; the cores are loop-free (no unwinding bounds) but `capacity` bounded to `u8` to keep the nonlinear `capacity*price` tractable; `kani::cover!` guards the bounded domain against vacuous passes; the dispatcher harness bounds its parser loops with `#[kani::unwind]` | full SMT proof, unbounded - the nonlinear `capacity*price <= u64::MAX` is discharged with one `assert ... by (nonlinear_arith)` - unbounded like Creusot, but Creusot's solver ensemble needed no in-code hint where Verus's single Z3 does - and no `u8` capacity cap (unlike Kani); dispatcher byte front deferred | SMT + inference, unbounded (no `u8` cap): the invariant, no-oversell, frames, legal status steps, and the exact payment product (`p == q * price`, stated in the signature) are checked; the nonlinear `capacity*price <= u64::MAX` fit and the payment positivity ride *trusted* lemmas (`mul_le_mono`/`mul_pos`), not proved, since flux-core's specs are absent here - a notch below Verus. Dropped (tool limits, recorded in the lane report): exact clamp value, `Ok<=>cond`, on-error-unchanged; overflow-checker not enabled | interactive Lean proof, unbounded (no `u8` cap). All 6 cores proved at the Creusot lane's coverage: invariant/no-oversell, full frames over every modeled field (name/slots/bump included), the exact clamp `min(requested, capacity - sold)` and exact `sold` delta, legal status step, on-error-unchanged with exact error discriminants, the end-to-end `purchase` composition (the full `is_initial` analog: ticket invariant under `is_set` preconditions + exact, positive economics), and - unlike Flux - the nonlinear `capacity*price <= u64::MAX` (mathlib monotonicity) **and** `create_event`'s `Ok<=>overflow` discriminant (modeled `checked_mul`) with the invariant's base case | translation + `mvcgen`/`grind`, unbounded (no `u8` cap). 5 of 6 cores proved at ~Creusot coverage: invariant/no-oversell, full frames, legal status step, on-error-unchanged with exact discriminants, the **proved** nonlinear `capacity*price <= u64::MAX`, and `create_event`'s `Ok<=>overflow` (reformulated with integer division, since `checked_mul` is unmodeled). Two costs (lane report): `record_sale`'s clamp proved as `<=` bounds not the exact `min` (as Flux drops), and the `purchase` composition extracted but **not yet proved** | - | - |
| Proof / replay speed | tbd (time `just creusot verify` / `replay`) | ~310s wall-clock from scratch (`-j` parallel harnesses; bounded by the dispatcher's ~300s, `purchase` ~210s, others 32-54s); no replay | fast (`11 verified` in seconds of solver time); re-verifies from scratch, no replay | ~0.2s from scratch (`8 checked`, 58 constraints; liquid-fixpoint + Z3); re-verifies each run, no replay | `verify` slow (Charon rebuilds a MIR-linkable sysroot each run - minutes); `replay` re-checks the committed Lean in seconds (`lean` against prebuilt oleans). Proof-*authoring* is the highest of the roster | re-verifies from scratch each run (extraction + `lean` against baked oleans); no replay. Proof-authoring high, like Aeneas (hand-written Lean triples) | - | - |
| Artifact story | 23 `proof.json` sessions under `verif/`, deterministic replay in pre-push | none committed - re-verifies from scratch every run | none committed - re-verifies from scratch (fast SMT queries) | none committed - re-verifies from scratch (specs are inline attributes) | **best of the roster**: committed Lean (the generated model + the hand proofs) under `verif/ticketing_aeneas/`, deterministically re-checked by `just aeneas replay` in pre-push | generated Lean regenerated + re-checked from scratch each run (git-ignored); the committed artifacts are the inline Rust `lean::after` proofs + the trusted `model_preamble.lean`. No `replay` (as in the scaffold) | - | - |
| Code bending | none in core bodies; extension-trait plumbing for foreign impls | cores call std directly (no axiom shims); predicates are `#[cfg(kani)]` bool fns on hand-written `*Ops` traits; PODs built field-by-field | low - the cores are written inside `verus!{}` and the proc-macro erases the ghost layer outside `cargo verus`, so the same source builds host+SBF like the siblings; residual bending is free `fn`s instead of `*Ops` traits, one `by (nonlinear_arith)` hint, and the verify recipe forcing `--cfg feature="nonzero_internals"` (vstd ships its `NonZero` model behind an undeclared feature) | moderate-high: `Event`/`Ticket` opaque, so cores read/write via trusted accessors (transparent extern-specs can't take the wire structs' padding + self-referential header); `NonZeroU64` bridged to `u64`, so accounting is `u64` and the exact clamp quantity is dropped; plain built-in `+`/`-`/`*` + a `mul_fits` guard replace `checked_*` (flux-core std specs invisible downstream). Offsetting win: no loop invariants or proof blocks, and plain rustfmt/clippy/rust-analyzer reach the whole crate (no verusfmt analog) | the *least* bending in the crate: the cores are ordinary Rust with no in-source spec at all, so host+SBF build the plainest source of any lane. The bending is *outside* the crate - the Charon config (`start_from`/`opaque`, `--sysroot default` to link the cdylib) and the separate Lean model + trusted glue specs. `NonZeroU64` kept out of the extracted cores (opaque), so accounting is `u64` like Flux | moderate: cores are `?`-free (extraction-first, like Aeneas) and carry inline `cfg(hax)` attribute strings (Lean `lean::after` triples + glue `lean::replace` models); `create_event`'s overflow gate rewritten to division (behavior-identical). Host+SBF build the plain program - the attributes erase off `cfg(hax)` | - | - |
| Toolchain fit | own nightly (2026-04-21) + opam-built Why3/solvers; the heaviest image | self-contained bundle: kani-verifier 0.67.0 (mise cargo backend) + CBMC/kissat via `cargo kani setup`; light image | release bundle (verus `0.2026.07.12` rolling + bundled Z3 4.12.5, mise github backend) over a pinned stable 1.96.0 rustup toolchain; needs glibc >= 2.39 so its base is trixie not the shared bookworm; `vstd` pinned from crates.io (newest, `2026-07-12`); no stable release newer than 07-05 yet, re-pin when one lands | built from source (`cargo xtask install`) at a pinned git rev - no release/crate/rustup component; pins but does not ship nightly-2025-11-25 (rustc-dev/rust-src/llvm-tools via rustup); Z3 4.16 (glibc-2.39) + liquid-fixpoint (floating `nightly` tag) pinned via mise github backends, so its base is trixie like Verus | the heaviest lane: opam-built Charon+Aeneas (pinned commits, Charon via Aeneas's `charon-pin`) + a Lean `v4.31.0`/elan toolchain + the mathlib tree (prebuilt cache, not compiled). Stays on bookworm. Not hermetic through `lake` (re-clones mathlib), so checked via `lean` + `LEAN_PATH` against the baked oleans; the cdylib forces `charon --sysroot default` | `cargo-hax`/`hax-driver`/`hax-rust-engine` `0.3.7` (mise cargo backend) over `nightly-2025-11-08` + elan `4.2.3`/Lean `v4.29.0-rc1`; the hax Lean library prebuilt from commit `d8b5b3d`. At `0.3.7` the Lean backend delegates THIR import to a native OCaml helper (opam-built in isolation, binary only), so the image is Aeneas-class. Stays on bookworm; checked via `lean` (not `lake`) against baked oleans | - | blocked before compilation: the 2023 release bundle uses Rust/Cargo 1.73 and rejects this workspace's resolver 3 manifest |
| Maintenance | active, research-led (v0.12.0 pinned) | active (AWS); drives the std verification campaign | active, research-led (weekly releases; drives a Rust std-verification effort) | active, research-led (UCSD); no tagged releases - git-HEAD only | active, research-led (Inria/AeneasVerif; Microsoft uses it for the SymCrypt Rust port). No semantic releases - dated `nightly-*` tags only, Charon pinned via Aeneas's `charon-pin` | active, research-led (Cryspen); powers libcrux's verified ML-KEM (shipped in Firefox and Signal). The Lean backend is experimental at `0.3.7` | - | latest release 2023-08-22; default branch last changed 2024-03-26 and still pins a 2023 nightly |

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

### 3. Verus - wired up

[Verus](https://github.com/verus-lang/verus) checks a verification dialect of
Rust against Z3, with specs and proofs written in the language itself. The
[std-verification experience reports](https://arxiv.org/abs/2510.01072) treat
it as the current state of the art for proving deep functional properties of
Rust.

Why third: the strongest deductive competitor to the baseline - and the
integration friction is the experiment. Code must live inside the `verus!{}`
macro (or the `verus_spec`/`verus_verify` attribute forms), so either the Verus
surface enters the normal and SBF builds or the cores get re-stated in a
parallel module, straining the byte-identical premise. How far the cores (or
the shells around them) must bend is exactly what this lane measures.

The full lane is wired: `mise/config.verus.toml`, a `verus-tooling` Docker stage,
`just verus verify`, a pre-push glob, and a `ticketing-verus` crate whose three
cores + domain contracts verify (`11 verified, 0 errors`). Full report:
[verus.md](verus.md). Three tooling-integration findings diverge from the
Creusot/Kani template:

- **Not self-contained.** Verus is a rustc driver whose release bundle pins a
  stable rustc (1.96.0) but does *not* ship it: mise provisions that
  toolchain with `rustc-dev`/`rust-src`/`llvm-tools` (like Creusot) and
  installs the bundle (verus + cargo-verus + bundled Z3 4.12.5) on top via its
  github backend.
- **Newer glibc than the shared base.** The release binaries need glibc >= 2.39,
  and the workshop's `debian:bookworm-slim` base ships 2.36. Only this stage's
  base is overridden (to `debian:trixie-slim`, glibc 2.41) via a
  `TOOLING_BASE_IMAGE` build-arg; every other lane stays on bookworm.
- **Gates by Cargo feature, not cfg.** `cargo-verus` drives via
  `RUSTC_WRAPPER=verus-driver` and exposes no cargo-visible `--cfg verus` (unlike
  `cargo kani`/`cargo creusot`), so the `[target.'cfg(<prover>)'.dependencies]`
  trick cannot work; what still needs gating - the optional `vstd` dependency
  and the foreign-type registrations - sits behind a `verus` Cargo feature
  instead (no `cfg(verus)` in `unexpected_cfgs`). The rest of the spec surface
  needs no gate at all: the `verus!` proc-macro erases it outside `cargo verus`.
  `vstd` is not auto-injected either - it rides in as an optional, feature-gated
  crates.io dependency pinned to the bundled version.

It is a full program like the siblings (trusted `instructions/` shells +
dispatcher + `entrypoint!` + `cdylib`, verified `flows/` cores), written as
idiomatic Verus: the cores live inside `verus!{}` with inline contracts, and
the `verus!` proc-macro (an ordinary crates.io dependency) erases the ghost
layer outside `cargo verus`, so `cargo build-sbf` produces a real `.so` from
the same source while `vstd` (which cannot link into the `no_std` SBF build)
never enters it. The generated client's Mollusk suite passes against
`ticketing_verus.so` just as it does against the Creusot and Kani `.so`s.

`NonZeroU64` was the integration wall: a monomorphic `NonZero<u64>` whose bound
`ZeroablePrimitive` has a private sealed supertrait, unregisterable the normal
way, and `vstd` ships its own `NonZero` model (`std_specs/nonzero.rs`) behind a
feature its manifest never declares, so no dependency can enable it through
Cargo. The lane forces that cfg on in the verify recipe
(`RUSTFLAGS=--cfg feature="nonzero_internals"` - only `vstd` tests it), so the
specs ride vstd's canonical `@` view and `new`/`get` specs with no unstable
feature gate or hand-rolled registration in the program crate; only
`checked_mul`/`Ord::min` (unspecced by vstd on `NonZero`) remain as trusted
wrappers. The nonlinear `capacity*price <= u64::MAX` is proved unbounded (one
`by (nonlinear_arith)`, no `u8` cap); proofs are fast SMT queries with no
committed artifacts. *Verifying* the dispatcher byte front is deferred (its code
is present but trusted; proving it needs specs on the unsafe parsers + a
`BoundedString` loop invariant - a weak deductive-Verus fit).

### 4. Flux - wired up

[Flux](https://github.com/flux-rs/flux) refines Rust's types with logical
predicates (liquid types) and checks them by SMT with inference. Annotation
overhead is near zero compared to contract-based tools and checking takes a
fraction of a second; it has been used to
[verify process isolation in Tock OS](https://events.ucsc.edu/event/cse-colloquium-flux-refinement-types-for-verified-rust-systems/).

Why fourth: the opposite end of the ergonomics spectrum before diving into
ITPs. Specs become refined signatures (`#[flux_rs::sig(...)]`) and struct
invariants (`#[flux_rs::invariant]`, e.g. `sold <= capacity`) instead of
requires/ensures clauses - a different way of *saying* the same contracts.

The full lane is wired: a `ticketing-flux` crate whose three cores + domain
mutators verify (`8 checked; 18 trusted`, ~0.2s), plus the tooling
(`mise/config.flux.toml`, a `flux-tooling` Docker stage, `just flux verify`, a
pre-push glob, `cfg(flux)` in `unexpected_cfgs`). `just solana build/test-client-rust
ticketing flux` builds `ticketing_flux.so` and the generated client's Mollusk
suite passes against it - the same 7 tests as the other lanes. Full report:
[flux.md](flux.md).

The event invariant landed on the type idiomatically (`#[flux_rs::invariant]`,
carried without threading through any `requires`), and the safety core - no
overselling, frame conditions, legal status steps, bounded/overflow-free
pricing - is machine-checked. Two verification walls define the cost, both
honest trusted-surface entries: **foreign structs cannot be transparent here**
(every transparent `#[field]` needs a refinement index, but `Event` has
`Reserved` padding and a self-referential `AccountHeader<Event>` header - a
recursive sort - so `Event`/`Ticket` are modeled *opaque* with the invariant on
the type and reached through trusted accessors/setters); and **`NonZeroU64` is
unmodeled while flux-core's std specs are invisible downstream** (so accounting
runs on `u64` via trusted `nz_*` bridges, the nonlinear/overflow steps are
trusted lemmas - the analog of Verus's `by (nonlinear_arith)` - and the cores
refine their `Ok` payload rather than the `Result` discriminant, dropping the
exact clamp value and the `Ok<=>cond` / on-error-unchanged clauses). On the
ergonomic upside, plain rustfmt/clippy/rust-analyzer reach the whole crate (no
`verusfmt` analog).

Two tooling deviations from the Creusot/Kani/Verus template, unchanged from the
scaffold:

- **Built from source, pinned by commit.** Flux publishes no release, crates.io
  crate, or rustup component, and its install is a custom `cargo xtask install`
  (not a mise backend like kani/verus), so only that build lives in the
  Dockerfile - the nightly toolchain and both solver backends (Z3 4.16,
  liquid-fixpoint) are pinned via mise, like the other lanes. Like Creusot/Verus
  it is a rustc driver that pins but does not ship its nightly
  (nightly-2025-11-25, resolved at runtime via `rustup which`); its prebuilt Z3
  asset needs glibc >= 2.39, so this stage's base is trixie like Verus.
- **`--cfg flux` works, but the dependency table does not.** Unlike cargo-verus,
  `cargo flux` sets `--cfg flux`, so verification-only code is `#[cfg(flux)]`-gated
  as under Kani and `cfg(flux)` is registered in `unexpected_cfgs`. But
  `[target.'cfg(flux)'.dependencies]` does not activate under cargo-flux (cargo
  probes `--print cfg` before the per-package flux profile sets the cfg), so the
  `flux_rs` attribute crate rides in as an unconditional dependency - its
  attributes strip to no-ops outside verification - and the crate opts into
  checking via `[package.metadata.flux] enabled = true`.

The pre-flagged nonlinear risk (`capacity * price <= u64::MAX`) played out as
predicted: liquid inference does not open a symbolic product, and here - without
flux-core's own specs in reach - it is a trusted `mul_le_mono` lemma rather than
a proved step, landing in the trusted-surface column as the honest cost.

### 5. Aeneas - wired up

[Aeneas](https://github.com/AeneasVerif/aeneas) extracts Rust through
[Charon](https://github.com/AeneasVerif/charon) into a pure functional model
and proves it interactively - primarily in Lean (F*, Rocq and HOL4 backends
exist). Microsoft uses it to
[verify the Rust port of SymCrypt](https://lean-lang.org/use-cases/aeneas/).

Why fifth: the first interactive prover in the series. Proofs are written by
hand in Lean (`progress`/`step`, `scalar_tac`, ...), a different cost model from
everything before it; the payoff is expressiveness and the best artifact story
of all - the proofs are committed Lean code, deterministically re-checked.

The full lane is wired: a `ticketing-aeneas` program crate (a port of the
Creusot reference program, written extraction-first - explicit `if`/`match`, no
`?` in the extracted cores) builds a behavior-identical `ticketing_aeneas.so`
(the generated client's Mollusk suite passes - the same 7 tests as the other
lanes), Charon extracts its `flows/` + `domain/` cores to Lean, and hand-written
proofs verify all six cores at the Creusot lane's spec coverage. The headline
result: the pure logic runs on `u64`, so the arithmetic Flux had to *trust* is
*proved* here - `payment_for`'s `payment = quantity * price`, positive, with no
overflow (the nonlinear `capacity*price <= u64::MAX` closed by mathlib
monotonicity, where Flux trusts a `mul_le_mono` lemma), plus `create_event`'s
exact `Ok<=>overflow` discriminant that Flux drops. `record_sale` (no
overselling, full frames, the exact clamp `min(requested, capacity - sold)` and
`sold` delta, on-error-unchanged), `close_sales`/`close_event` (status step,
organizer auth), and the end-to-end `purchase` composition (under `is_set`
preconditions: the ticket invariant, ticket pins event/buyer/payment, the exact
positive economics) round out the safety core. Full report:
[aeneas.md](aeneas.md). Three integration facts diverge from the Creusot/Kani
template, deeper than any prior lane's:

- **No in-Rust verification surface.** Aeneas proves from the outside - Charon
  extracts to LLBC, Aeneas translates to Lean, and the contracts are hand-written
  Lean - so there is no spec to strip, no `cfg(aeneas)` in check-cfg, no verifier
  dependency, and no root `Cargo.toml` edit. `ticketing-aeneas` is the plainest
  crate of any lane, compiling unchanged on host and SBF.
- **Committed-Lean artifact story.** Unlike Kani/Verus/Flux (re-verify from
  scratch, nothing committed), the proofs are committed Lean re-checked
  deterministically - the best artifact story of the roster. The project splits
  into two committed files: `TicketingAeneas.lean`, the Aeneas-*generated* model
  (regenerated by `verify`, never hand-edited - the analog of Creusot's Coma or
  the codama client's `src/generated/`), and `TicketingAeneasProofs.lean`, the
  *hand-written* proofs that `import` it (the actual verification artifact, where
  the contracts live). So the lane, like Creusot, splits `verify` (regenerate the
  model via Charon+Aeneas, then check) from a cheap `replay` (re-check the
  committed Lean, the pre-push gate). The re-check runs `lean` directly against
  the image's prebuilt oleans rather than `lake build`: Lake re-resolves the
  dependency graph every run and re-clones mathlib from the network ("URL has
  changed; ... cloning again"), so `scripts/aeneas/lean-check.sh` points
  `LEAN_PATH` at the baked Aeneas + mathlib build instead - hermetic, no network.
- **Heaviest image; mathlib is the new cost.** opam-built Charon+Aeneas
  (Creusot-class weight) plus a Lean/elan toolchain whose Aeneas base library
  `require`s mathlib (fetched as a prebuilt cache, not compiled). The Charon
  commit is dictated by Aeneas's `charon-pin`, not chosen independently. Stays on
  bookworm (Lean/elan and the rustup nightly support glibc 2.36, unlike
  Verus/Flux).

Costs and risks: still to spike at wire-up - Charon has to extract the crate
graph with pinocchio/bytemuck in it, or those get marked opaque.

### 6. hax - wired up (5/6 cores)

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

The lane is wired: `ticketing-hax` is the full Pinocchio program (trusted
`instructions/` shells + `entrypoint!` + `cdylib`, `flows/`/`domain/` cores),
building a behavior-identical `ticketing_hax.so` that passes the generated
client's Mollusk suite (the same 7 tests as the other lanes). Five of the six
decision cores - `payment_for`, `create_event`, `record_sale`, `close_sales`,
`close_event` - are proved in Lean; the `ticket_buy`/`purchase` composition is
extracted but not yet proved. Full report: [hax.md](hax.md).

- `cargo-hax`, `hax-driver`, and `hax-rust-engine` are pinned together at
  `0.3.7` over upstream's `nightly-2025-11-08`; elan `4.2.3` installs Lean
  `v4.29.0-rc1`. The hax Lean proof library is built from the exact commit
  behind that release (`d8b5b3d`) and baked into the bookworm tooling image.
- The Lean backend dispatches through hax's Rust engine, but at `0.3.7` that
  engine still delegates THIR import/shared phases to native `hax-engine` over
  a JSON protocol. The helper is opam-built with upstream's OCaml `5.1.1` in an
  isolated builder; only its binary reaches the final image. Two Rust build
  helpers (`hax-export-json-schemas` and unpublished
  `hax-engine-names-extract`) generate the engine's schema universe. This makes
  the image Aeneas-class to build despite the Rust/Lean runtime pipeline.
  `just hax verify` performs Rust -> hax -> Lean without network access at run
  time: direct `lean` checks against the baked Hax/Qq oleans because a clean
  `lake build` re-clones quote4.
- Contracts are inline `cfg(hax)` attributes, but the rich cores use embedded
  Lean Hoare triples (`#[hax_lib::lean::after("theorem ⦃ ⦄ … := by hax_mvcgen
  …")]`, the idiom hax's `barrett`/`adc` examples use) rather than
  `requires`/`ensures`: the `Spec` path the smoke proof used cannot express a
  `Result`-match postcondition (its `pureEnsures` construction rejects it). The
  extraction/opacity boundary landed as a committed `model_preamble.lean` (wire
  types as Lean structures / `Nat` / a `u64` subtype) spliced into the generated
  module, with `domain::glue` given concrete Lean models via `lean::replace`.
- Findings vs the pre-flagged risks: `future(event)`/`&mut` **works** in the
  experimental Lean backend (cores keep their `&mut` signatures); `u64::checked_mul`
  is unmodeled, so `create_event`'s overflow gate is reformulated with integer
  division (behavior-identical, still proved). The nonlinear `capacity*price <=
  u64::MAX` is proved (as in Aeneas). Costs recorded in `docs/hax.md`:
  `record_sale`'s clamp is proved as `<=` bounds rather than the exact `min` (as
  Flux also drops), and `purchase` (which hax `0.3.7` will not compose from the
  per-core theorems) is the one remaining proof.
- Generated Lean is regenerated and re-checked from scratch every run (git-
  ignored); the source artifacts are the inline Rust proofs and the committed
  model preamble, so there is no separate replay command. The F* and Rocq
  backends are out of scope, keeping the comparison with Aeneas on one Lean
  target.

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

### 8. Prusti - blocked

[Prusti](https://github.com/viperproject/prusti-dev) (ETH Zurich) verifies
annotated Rust through the Viper infrastructure. Kept on the roster
deliberately, but development has stalled and it is absent from the std
campaign. The latest
[release](https://github.com/viperproject/prusti-dev/releases/tag/v-2023-08-22-1715)
is from 2023-08-22 and bundles `nightly-2023-08-15` (Rust/Cargo 1.73); the
default branch last changed on 2024-03-26 and still pins
[`nightly-2023-09-15`](https://github.com/viperproject/prusti-dev/blob/master/rust-toolchain).
Both toolchains predate edition 2024 and Cargo resolver 3.

The required compatibility spike was run against this workspace on
2026-07-16 with the official Ubuntu release. `cargo-prusti` stopped while
parsing the root manifest, before compiling any crate or invoking Viper:

```text
error: failed to parse manifest at `Cargo.toml`

Caused by:
  `resolver` setting `3` is not valid, valid options are "1" or "2"
```

There is therefore no `ticketing-prusti` crate or Prusti Docker/mise/just
lane. Downgrading the workspace or making an isolated edition-2021 toy would
no longer test the same modern Solana program and is out of scope. Revisit
Prusti only after an upstream release or maintained fork supports edition
2024 and resolver 3; if that happens, its value remains a syntax/ergonomics
comparison against Creusot, since both are deductive and annotation-based.

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
