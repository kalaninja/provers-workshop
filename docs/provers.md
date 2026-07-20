# The provers

This is the comparison index for the workshop. [Architecture](architecture.md) describes the fixed
Solana target; this page records what changes when the prover changes. Repository-derived facts and
external links were reviewed on **2026-07-19**.

External ecosystem statements are point-in-time observations, not promises about future releases or
maintenance. Recheck upstream before changing a pin or reviving a blocked lane.

## Roster

| # | Prover | Paradigm | Status | Report |
| --- | --- | --- | --- | --- |
| 1 | Creusot | Deductive verification through Why3 and SMT | Implemented baseline | [Creusot](creusot.md) |
| 2 | Kani | Bounded model checking through CBMC | Implemented | [Kani](kani.md) |
| 3 | Verus | SMT over a verification dialect of Rust | Implemented | [Verus](verus.md) |
| 4 | Flux | Refinement types through liquid-fixpoint and Z3 | Implemented | [Flux](flux.md) |
| 5 | Aeneas | Rust translation and interactive Lean proofs | Implemented | [Aeneas](aeneas.md) |
| 6 | hax | Rust translation to inline Lean proofs | Implemented | [hax](hax.md) |
| 7 | VeriFast | Separation logic and symbolic execution | Implemented program plus detached model | [VeriFast](verifast.md) |
| 8 | Prusti | Deductive verification through Viper | Blocked | See [Prusti](#prusti) |

The order introduces a different verification model at each step: deductive proof, bounded model
checking, an SMT-checked Rust dialect, refinement types, translation to an interactive theorem
prover, a second translation pipeline on the same Lean target, and finally separation logic. Prusti
remains on the roster because its compatibility failure is useful evidence, not because a partial
lane exists.

## Comparison rules

Creusot defines the reference specification. Later lanes target the same three top-level decision
cores and the domain operations beneath them, but exact parity is not asserted where a tool cannot
express a clause. Every omission must be named in the lane report and in the coverage table below.

An implemented lane must provide:

- a program crate over `ticketing-interface` that preserves the shared wire protocol and decisions;
- verification material absent from ordinary and SBF target artifacts;
- a pinned tooling image and a `just <prover> verify` workflow;
- a change-scoped pre-push gate;
- an SBF build passing the shared generated-client suite;
- a report identifying verified scope, trusted surface, artifacts, and limitations.

Custom verifier cfgs belong in the workspace `unexpected_cfgs` allowlist. Verifier-only dependencies
must use the activation mechanism the tool actually supports. Proof artifacts live under `verif/`
when the tool produces stable, reviewable files.

The account-transition text guard is narrower than these general rules: it currently scans only the
Creusot flow sources and their `cfg_attr(creusot, ensures(...))` syntax. Other lanes state transition
properties in their contracts, but the guard does not independently enforce them.

## Verification coverage

The three cores are `create_event`, ticket purchase, and `close_event`. The reference domain
operations are `payment_for`, `record_sale`, and `close_sales`.

| Prover | Verification model | Coverage and bound | Disclosed gap |
| --- | --- | --- | --- |
| Creusot | Full deductive proof | All cores and domain operations, unbounded | Imperative instruction shells remain trusted |
| Kani | Bounded model checking | Core contracts plus dispatcher byte front; capacity bounded to `u8` | Proof is valid only inside the harness domain |
| Verus | Full SMT proof | All cores and domain operations, unbounded | Dispatcher byte front remains trusted |
| Flux | Refinement checking | Invariant, frames, no oversell, legal status steps, and exact payment | Exact clamp, result discriminants, on-error frames, and ticket consistency are not expressed |
| Aeneas | Generated model plus Lean proof | All cores and domain operations at reference coverage, unbounded | Wire types and glue are modeled through trusted specifications |
| hax | Generated Lean plus inline Hoare proofs | All cores and domain operations, unbounded | Clamp is proved by upper bounds rather than exact `min` |
| VeriFast | Detached symbolic model | Three top-level cores at reference behavioral coverage, unbounded | Shipping cores and separate leaf operations are not verified directly |
| Prusti | None | Compatibility spike stops before compilation | Edition 2024 and Cargo resolver 3 unsupported |

The shared client suite supplies executable protocol parity, not proof parity and not binary identity.
In particular, passing the suite does not close VeriFast's model-to-program correspondence gap.

## Trusted surface and artifacts

| Prover | Main trusted surface | Artifact model |
| --- | --- | --- |
| Creusot | Instruction shells and foreign-type axioms | 23 committed Why3 sessions; deterministic replay |
| Kani | Code outside the harness reachability graph | No proof object; re-verifies from source |
| Verus | Instruction shells, foreign registrations, `Address`, and two `NonZero` wrappers | No proof object; re-verifies from source |
| Flux | Opaque wire accessors, constructors, `NonZero` bridges, and arithmetic helpers | No proof object; re-verifies from source |
| Aeneas | Opaque wire model and field-level glue specifications | Committed generated Lean plus hand-written proofs; deterministic replay |
| hax | Committed foreign-type model and asserted `lean::replace` mappings | Inline proofs and model preamble; extraction regenerated on each run |
| VeriFast | One checked-multiply axiom and detached-model fidelity | No proof object; re-verifies the detached source model |

The trusted surface is part of the result. A shorter annotation file is not automatically stronger if
it moves more behavior behind opaque accessors or model correspondence.

## Performance methodology

Wall-clock numbers are comparative development measurements, not portable benchmarks. Each recipe ran
once as an untimed warm-up and then three times on the same otherwise active workstation. The table
reports the median and full observed range. `SKIP_IMAGE_BUILD=1` excluded image construction; registry,
Git, toolchain, and dependency caches stayed warm. The timed interval still includes `just`, container
startup, compilation required by the recipe, extraction, and the verifier itself.

Cargo-Verus incrementally skips an unchanged verified crate. Its package output was therefore cleaned
before every timed invocation; shared dependencies remained warm. The other workflows actively invoke
their prover or proof checker on repeated runs and were measured without extra invalidation. Creusot's
`verify` measurement is session-aware because the repository intentionally commits its proof sessions.

Measurements below were taken on 2026-07-19: Intel Core i7-12800H, 20 logical CPUs, 30 GiB RAM,
Linux 7.0.0-28 x86_64, Docker 29.6.2, and `linux/amd64` tooling images.

| Prover | Verify median [range] | Replay median [range] | Stable result count |
| --- | --- | --- | --- |
| Creusot | 1.02 s [0.99-1.02] | 1.41 s [1.40-1.43] | 23 proof sessions |
| Kani | 354.70 s [353.82-360.17] | Not available | 4 harnesses; 9/9 covers satisfied |
| Verus | 1.80 s [1.66-2.76] | Not available | 11 verified functions |
| Flux | 0.96 s [0.90-1.00] | Not available | 8 checked, 18 trusted, 58 constraints |
| Aeneas | 11.59 s [11.10-20.16] | 9.40 s [9.09-9.86] | Generated model plus hand-written Lean proofs |
| hax | 24.39 s [24.03-27.32] | Not available | Inline proofs plus model preamble |
| VeriFast | 0.62 s [0.59-0.96] | Not available | 143 statements verified |

## Tool and backend map

The automated table records the exact path exercised by the pinned workshop lane, avoiding broader
claims about every backend a current or future upstream release may support.

### Automated verification

| Prover | Workshop verification path |
| --- | --- |
| Creusot | Why3 verification conditions to Alt-Ergo, Z3, CVC4, and CVC5 |
| Kani | CBMC goto program to Kissat |
| Verus | Direct SMT encoding to bundled Z3 |
| Flux | liquid-fixpoint constraints to Z3 |
| VeriFast | Symbolic execution with the built-in Redux prover |
| Prusti | Viper backend not reached by the compatibility spike |

### Translation to an interactive prover

| Prover | Upstream targets | Workshop target |
| --- | --- | --- |
| Aeneas | F*, Coq, HOL4, Lean | Lean 4 |
| hax | F*, Coq/Rocq, Lean, EasyCrypt, ProVerif | Lean 4 |

Lean is fixed for both translation lanes so the comparison isolates the translation and proof models
rather than changing the target prover at the same time. It also lets both lanes use the same mathlib
arithmetic facts for the nonlinear capacity-times-price bound.

## Referenced use cases

These links support the limited claim that each tool has been applied to the named artifact. They are
not endorsements or claims that every deployment uses the same backend exercised here.

| Prover | Referenced use case |
| --- | --- |
| Creusot | [CreuSAT](https://github.com/sarsko/CreuSAT), a verified SAT solver |
| Kani | [s2n-quic](https://github.com/aws/s2n-quic) and [Firecracker security-boundary proofs](https://model-checking.github.io/kani-verifier-blog/2023/08/31/using-kani-to-validate-security-boundaries-in-aws-firecracker.html) |
| Verus | [VeriSMo](https://github.com/microsoft/verismo) and [Anvil](https://github.com/anvil-verifier/anvil) |
| Flux | [TickTock](https://nilehmann.github.io/papers/sosp25-ticktock.pdf) verification of isolation in [Tock](https://tockos.org) |
| Aeneas | [Microsoft's Rust SymCrypt verification](https://lean-lang.org/use-cases/aeneas/) |
| hax | Verified cryptographic components in [libcrux](https://github.com/celabshq/libcrux) through the F* backend |
| VeriFast | Accepted tool in the [Rust standard-library verification campaign](https://model-checking.github.io/verify-rust-std/tools/verifast.html) |

## Lane summaries

### Creusot

Creusot is the reference deductive lane. Plain Rust cores carry pearlite contracts, Why3 dispatches
verification conditions to a solver ensemble, and 23 committed sessions support deterministic replay.
Its main cost is the explicit trusted model for foreign `NonZeroU64` and `Address` behavior.

### Kani

Kani inverts the proof question: CBMC searches for a violating execution within a bounded domain. It
also verifies the dispatcher byte front, a good fit for BMC, but the nonlinear economics force a
`u8` capacity bound. `kani::cover!` checks keep both result branches and the sale clamp reachable.

### Verus

Verus proves the cores unbounded with inline contracts and direct Z3 diagnostics. Its `verus!` macro
erases ghost content from ordinary builds, allowing the same source to produce SBF. The main
integration wall is the pinned `vstd` model for `NonZeroU64`.

### Flux

Flux carries the event invariant on the refined type and checks the safety core with very little
annotation. Opaque foreign wire types and unavailable downstream standard-library refinements move
substantial behavior into trusted accessors and arithmetic helpers, and several exact functional
clauses cannot be stated.

### Aeneas

Aeneas extracts ordinary Rust through Charon, generates a functional Lean model, and checks separate
hand-written proofs. It reaches the reference arithmetic and frame coverage but pays for a large
trusted wire-model interface and a separate proof-authoring workflow.

### hax

hax also targets Lean, but its Hoare proofs live inline in Rust attributes and run through `RustM` and
`mvcgen`. The nonlinear bound is proved, generated Lean is disposable, and the committed model
preamble states the foreign-type correspondence. The exact clamp remains a disclosed gap.

### VeriFast

VeriFast confirms a target-fit limitation: the verified layer is heap-free, so its separation-logic
strength is not exercised. Its frontend cannot import the shared wire structs, forcing a detached
model. The model proves the three top-level core contracts, while model fidelity remains trusted.

## Prusti

The official [2023-08-22 release](https://github.com/viperproject/prusti-dev/releases/tag/v-2023-08-22-1715)
uses a Rust/Cargo generation that predates edition 2024 and resolver 3. A compatibility spike against
this workspace on 2026-07-16 stopped while parsing the root manifest:

```text
error: failed to parse manifest at `Cargo.toml`

Caused by:
  `resolver` setting `3` is not valid, valid options are "1" or "2"
```

There is no Prusti crate, image, or `just` lane. Downgrading the workspace would change the target and
is outside the comparison rules. Revisit only when an upstream release or maintained fork accepts the
current workspace manifest.

## Considered but not integrated

The following are scope decisions, not general judgments about the tools:

- MIRAI: archived abstract interpreter.
- RefinedRust: targets unsafe Rust through Rocq/Iris and would introduce a different proof project.
- coq-of-rust: overlaps the translation quadrant already represented by Aeneas and hax.
- crux-mir: symbolic testing overlaps the Kani lane.
- goto-transcoder with ESBMC: not mature enough when the roster was selected.
- Miri, property testing, and fuzzing: valuable testing tools, but not formal provers under the
  workshop's comparison definition.
