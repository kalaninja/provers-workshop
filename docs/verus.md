# Verus lane

Lane 3 of [the prover roadmap](provers.md). Status: **wired up**. Facts are
point-in-time; last reviewed 2026-07-12.

[Verus](https://github.com/verus-lang/verus) checks a verification dialect of
Rust against Z3, with specs and proofs written in the language itself. The
[std-verification experience reports](https://arxiv.org/abs/2510.01072) treat
it as the current state of the art for proving deep functional properties of
Rust.

Why third in the roster: the strongest deductive competitor to the Creusot
baseline, and the integration friction is the experiment. Two things turned out
to be the story: one foreign type (`NonZeroU64`) nearly blocked the lane, and
keeping the crate a *real, deployable program* (not just a proof artifact) drove
the whole spec-layer shape.

## What is wired

A full `ticketing-verus` program crate over the shared `ticketing-interface` -
behavior-identical to `ticketing-creusot`/`ticketing-kani`, split along the
verification boundary exactly like the siblings:

- `instructions/` - trusted imperative shells (account parsing, PDA creation,
  clock reads, the transfer CPI, persistence) plus the dispatcher and
  `entrypoint!`. Plain Rust outside `verus!{}`, so outside verification by
  construction.
- `flows/` - the decision cores (`create_event`, `purchase`, `close_event`),
  written inside `verus!{}` with their `requires`/`ensures` inline in the
  signature.
- `domain/` - state predicates as `spec fn`s next to the verified typed
  mutators (`record_sale`, ...) in the same `verus!{}` blocks. `NonZeroU64` is
  modeled by vstd's `std_specs/nonzero` (`@` views); the residual foreign-type
  layer is an opaque `Address` and two trusted `nz_*` wrappers
  (`checked_mul`/`min`) - the residue of Creusot's axiom analog.

`just verus verify` discharges every core with the bundled Z3 (`11 verified, 0
errors`), re-verifying from scratch each run (no committed artifacts).
`just solana build ticketing verus` builds the SBF `.so`, and the generated
client's Mollusk suite passes against it (`just solana test-client-rust
ticketing verus`) - the same 7 tests that pass against Creusot and Kani, since
the program is behavior-identical across lanes.

**How a `verus!{}` tool yields a byte-identical program.** Verus verified code
lives inside the `verus!{}` macro, which needs `vstd`; `vstd` cannot link into
the `no_std` SBF build. The macro itself resolves this: `verus_builtin_macros`
(an ordinary crates.io proc-macro, pinned to the `vstd` version) **erases**
every ghost item - `spec fn`s, contracts, `proof{}` blocks, `broadcast use` -
unless the Verus driver is compiling (`--cfg verus_keep_ghost`, which only
`cargo verus` sets). So the exec cores sit in ungated `verus!{}` blocks with
their contracts inline, and ordinary `cargo build` / `cargo build-sbf` expand
the same source to the plain program (host + SBF, zero verification overhead,
no `vstd` in the graph; the proc-macro never links into the target artifact).
Only two things stay behind the `verus` Cargo feature that
`cargo verus verify --features verus` turns on: the optional `vstd` dependency
and the foreign-type registration block (spec-only declarations with no meaning
outside verification). This keeps the "code bending" at zero - the
program text *is* Verus text - contrary to the initial expectation that Verus
would force either a verification-only crate or a cfg-gated attribute dialect.

## The NonZeroU64 wall (headline finding)

`Event`/`Ticket` embed `NonZeroU64` fields, so Verus must be given a
specification for the type before it can name, read, or construct those
structs - and on the pinned release that model is walled off from a normal
user crate:

- `vstd` ships the canonical model (`std_specs/nonzero.rs`: the `NonZero<T>`
  registration, the `@` view, `new`/`get` specs, the not-zero axiom) but gates
  it behind `#[cfg(feature = "nonzero_internals")]` **without declaring that
  feature** in its manifest - on crates.io and in the git tree alike - so no
  dependency can turn it on through Cargo (verified: `cargo` rejects
  `features = ["nonzero_internals"]` outright; only Verus's own build of `vstd`
  enables the cfg).
- A user crate cannot rebuild the model on its own terms either: the
  `impl View for NonZero<T>` is orphan-ruled out, and a from-scratch
  registration must reproduce the sealed-bound dance (`NonZero<T>`'s generic
  form is mandatory, its `ZeroablePrimitive` bound and `private::Sealed`
  supertrait are unstable std internals reachable only via
  `external_trait_private_bound` plus a crate-level
  `#![feature(nonzero_internals)]`) and then re-state the whole model by hand -
  a view axiom plus `new`/`get`/`checked_mul`/`min` wrappers. The lane's first
  draft did exactly this, and it consumed the whole integration budget.

The resolution is vstd-native: force the cfg on, for the verification build
only - `just verus verify` runs with
`RUSTFLAGS=--cfg feature="nonzero_internals"`. Only `vstd` tests that cfg, and
`vstd`'s own `verus_keep_ghost`-gated `#![feature(nonzero_internals)]` makes
the module compile under the driver. The specs ride vstd's model directly -
`n@` views in the contracts, vstd's `new`/`get` specs at the call sites - and
the program crate carries no unstable feature gate, no `NonZero` registration,
and no positivity axiom of its own. What vstd leaves unspecced on `NonZero` -
`checked_mul` (a per-type inherent method its generic specs miss) and
`Ord::min` (no `OrdSpec` impl) - remains as the two trusted
`nz_checked_mul`/`nz_min` wrappers, the residue of Creusot's `ax_*` layer.

## Paradigm contrasts

- **Same question as Creusot, unbounded.** Like Creusot, Verus *proves* the
  contracts for all inputs via SMT - not Kani's bounded search. So the event
  invariant's nonlinear `capacity * price <= u64::MAX` is discharged with **no
  `capacity <= u8` bound**: the one genuine step (multiplication monotonicity in
  `record_sale`) is a single `assert(...) by (nonlinear_arith)`, and
  `payment_for`'s overflow branch falls out as a contradiction in `purchase`
  rather than being re-proved. Sharpest contrast with Kani, whose SAT backend
  forced the `u8` bound. Against **Creusot** the *result* is identical (both
  unbounded) but the *effort* differs: Creusot discharged the same bound with no
  nonlinear hint anywhere in its sources - its Why3 + multi-solver ensemble
  (alt-ergo/z3/cvc5) closed it - while Verus's single bundled Z3 does not open a
  symbolic-times-symbolic product by default, so the `by (nonlinear_arith)` block
  is required. Same theorem, more hand-holding.
- **One-stage to Z3, not two-stage through Why3.** Creusot translates
  Rust+pearlite into WhyML and lets `why3find` dispatch each verification
  condition across that solver ensemble, then commits the resulting sessions for
  deterministic replay. Verus is a rustc driver that encodes straight to its one
  bundled Z3 - no intermediate verification language, no committed sessions,
  re-verified from scratch each run. Fewer moving parts and a lighter image, at
  the cost of a single solver (hence the manual nonlinear hint above) and no
  replayable artifact.
- **Spec layer erased by the macro, not cfg-gated.** Creusot/Kani gate spec
  material with `#[cfg_attr(prover, requires/ensures)]` attributes on plain-Rust
  cores. Verus inverts this: the cores are written in the Verus dialect (inside
  `verus!{}`, contracts inline in the signature) and the proc-macro erases the
  ghost layer from ordinary builds, instead of cfg stripping attributes. Same
  shipped result - the exec bodies expand to the plain Rust of the sibling
  lanes - but the source reads as idiomatic Verus rather than
  attribute-decorated Rust. The residual bending: free `spec fn`/`fn` instead
  of the siblings' `*Ops` extension traits (Verus traits over foreign types
  have rough edges).
- **Foreign types are registered, not translated.** Creusot translates the whole
  crate graph and sees `Event`/`Ticket`/`EventStatus` fields for free; Verus
  verifies a single crate, so every foreign type is registered with an
  `external_type_specification` before spec/exec code can touch it - a
  declarative layer Creusot never pays.
- **Foreign-function contracts attach in place, not via wrappers.** Both
  deductive lanes assume the same axiom about `Address` equality - runtime `eq`
  returns `true` exactly on logically equal values - but they pin it
  differently. Creusot's `extern_spec!` for `PartialEq::eq` speaks through
  `DeepModel`, and the orphan rule forbids implementing that foreign trait for
  the foreign `Address`, so the axiom lands on a crate-local extension-trait
  wrapper and the exec code bends to `self.organizer.ax_eq(organizer)`.
  Verus's `assume_specification` (like its `external_type_specification`) is a
  verifier-level registration, not a real trait impl - the orphan rule does not
  apply - so the same contract attaches to `<Address as PartialEq>::eq` itself
  and the call site stays the idiomatic `e.organizer == *organizer`. Same
  trusted content, different attachment point.

## Findings

- **Trusted surface: the `NonZeroU64` model moved into vstd.** The `@` view,
  the `new`/`get` specs, and the not-zero axiom come from vstd's
  `std_specs/nonzero` - still assumptions, but standard-library-grade ones, the
  same trust class as the `u64` `checked_add`/`checked_sub` specs the cores
  already lean on - so the lane states no `NonZero` axiom of its own. What
  stays lane-trusted: the opaque `is_set` and `Address` equality, the
  `nz_checked_mul`/`nz_min` wrappers (the two ops vstd leaves unspecced), and
  the `account_header()`/`zero_reserved()` constructors - strictly below
  Creusot's `ax_*` set, which states the whole `NonZero` model in-lane. On top
  sits the registration layer Creusot got free: transparent
  `external_type_specification` mirrors for
  `Event`/`Ticket`/`EventStatus`/`ProgramError` (the last a plain enum, so exec
  code constructs error variants directly - no trusted constructors), and
  opaque ones for the wire padding (`AccountHeader<T>`, `Reserved<N>`,
  `BoundedString<N>`). The `instructions/` shells are trusted, exactly as in
  every lane. Versus Kani (which axiomatizes nothing) the surface is still
  larger; the trade is the unbounded nonlinear proof.
- **The practical delta from Creusot is arithmetic hand-holding, not power.**
  Same theorems, a slightly smaller trusted surface (see above), both unbounded
  over `int` - the one place the two deductive lanes diverge in *effort* is
  nonlinear arithmetic: Creusot's
  Why3 + solver ensemble (alt-ergo/z3/cvc5) closed `capacity*price <= u64::MAX`
  with nothing written in the sources, while Verus's lone bundled Z3 needed the
  explicit `assert(...) by (nonlinear_arith)`. The flip side of the single-solver,
  direct-to-Z3 pipeline: Verus commits no proof sessions and the contracts read as
  ordinary Rust.
- **`NonZeroU64`'s leftovers need wrappers, not `assume_specification`.**
  `checked_mul` is a per-type inherent method (not generic over
  `ZeroablePrimitive`, so vstd's generic `NonZero` specs miss it), and `min` is
  a provided `Ord` method Verus specs only through `OrdSpec`, which vstd does
  not implement for `NonZero`. Both are trusted `#[verifier::external_body]`
  wrappers specced on the `@` view. (`u64`'s own `checked_add`/`checked_sub`
  and `NonZeroU64::new`/`get` are spec'd by `vstd`, so the cores call them
  directly; the sale clamp's spec-side `min` is `vstd::math::min`.)
- **Idioms that differ from the naive port.** The pinned release uses the new
  `&mut` model: a dereferenced mutable parameter in an `ensures` *must* be
  disambiguated as `*old(x)`/`*final(x)` - a bare `*x` post-state is a compile
  error. Frame conditions are functional-update struct equalities
  (`*final(event) == Event { sold: final(event).sold, ..*old(event) }`),
  folded into the same `ensures` match as the outcome clauses. `NonZero` values
  enter specs through vstd's `@` view - typed `u64`, so non-negativity comes
  from the type and no positivity axiom or `broadcast use` is needed - and
  `let...else` verifies fine (so the cores stay clippy-clean without
  `manual_let_else` allows). `AccountHeader<T>`'s `PhantomData<fn() -> T>`
  makes `Event` look self-recursive through a function-pointer position,
  needing `accept_recursive_types(T)` on the opaque registration.
- **One manual proof in the whole lane.** `record_sale`'s multiplication
  monotonicity (`assert ... by (nonlinear_arith)`) is the only in-body proof;
  `purchase` composes `record_sale`/`payment_for`/`ticket_new` with no hints
  (an earlier attribute-mode draft needed two `proof!{}` hand-holds there;
  in the `verus!{}` form Z3 discharges the same obligations from the contracts
  alone).
- **Non-vacuity by mutation test.** Verus proves over all inputs, so the only
  vacuity vector is a self-contradictory `requires`. Deleting `create_event`'s
  `capacity.checked_mul(price)` guard turns the run red (`event_new`'s
  `capacity*price <= u64::MAX` precondition becomes unprovable), so the guard is
  demonstrably load-bearing.
- **Behavior parity is executable, not just asserted.** The generated client's
  Mollusk suite is lane-agnostic (it loads `ticketing_<lane>.so` via
  `TICKETING_PROGRAM_NAME`); the same 7 tests pass against `ticketing_verus.so`,
  `ticketing_kani.so`, and `ticketing_creusot.so`, and the Verus `.so` is the same
  size as the Creusot one.

## Deferred

- **Verifying the dispatcher byte front** (the panic-freedom property the Kani
  lane proves) - the dispatcher *code* is present as a trusted shell, but proving
  it in Verus is a poor deductive fit: the interface's `InstructionArgs::parse` is
  `unsafe` (raw pointers, `bytemuck::checked`) and `BoundedString`/`Reserved`
  validation *loops* need explicit `invariant`/`decreases` clauses - exactly where
  Kani spent its only `#[kani::unwind]`. A future "widen the verified surface"
  step.
- **The account transition guard** (`scripts/check-ticketing-transitions.sh`) is
  still Creusot-only: hardcoded to `crates/ticketing/creusot/src/flows`, invoked
  only from `just/creusot.just`, and matching Creusot's
  `is_legal_transition_from(*arg)` form, not Verus's `old(event)` form. The Verus
  flows keep the `is_legal_transition_from` predicate name (both `purchase` and
  `close_event` carry it) so the guard can be generalized later; today it guards
  neither Verus nor Kani.
