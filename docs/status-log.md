# Status log

The dated chronology of [the prover roadmap](provers.md): what landed in each
lane and what it taught, newest first.

- **2026-07-13** - Flux lane wired up as a full SBF program: trusted
  `instructions/` shells + dispatcher + `entrypoint!` + `cdylib` around the three
  verified `flows/` cores, behavior-identical to the other lanes. `just flux
  verify` is green (`8 checked; 18 trusted; 0 ignored`, 58 constraints, ~0.2s);
  `just solana build/test-client-rust ticketing flux` builds `ticketing_flux.so`
  and the generated client's Mollusk suite passes against it - the same 7 tests
  as Creusot/Kani/Verus. The event invariant rides the `Event` type
  (`#[flux_rs::invariant]`, no `requires` threading - the idiomatic win the
  roadmap predicted); the safety core (no overselling, frames, legal status
  steps, overflow-free pricing) is checked; non-vacuity mutation-tested (dropping
  `record_sale`'s clamp makes the setter's `sold <= capacity` precondition
  unprovable). Two walls set the cost, both trusted surface: (1) transparent
  extern-specs are impossible for the wire structs - every `#[field]` needs a
  refinement index, but `Event` has `Reserved` padding and a self-referential
  `AccountHeader<Event>` header (a recursive sort) - so `Event`/`Ticket` are
  opaque (the `core::alloc::Layout` pattern) with trusted field accessors/setters,
  surface Verus got free from transparent registration; (2) `NonZeroU64` is
  unmodeled (generic over the sealed `ZeroablePrimitive`, and Flux rejects
  refining a generic `T` by an `int`) and flux-core's own
  `checked_*`/`Result`/`Option` specs are not visible to a downstream crate at
  rev `1d98676` (probed: `Result<(), ()>[b]` fails to sort-check while a local
  refined enum indexes fine) - so accounting runs on built-in `u64` arithmetic
  via trusted `nz_get`/`nz_from` bridges, the nonlinear/overflow/positivity steps
  are trusted `mul_le_mono`/`mul_fits`/`mul_pos` lemmas (the analog of Verus's
  `by (nonlinear_arith)`), and the cores refine their `Ok` payload rather than
  the `Result` discriminant (dropping the exact clamp value and the `Ok<=>cond` /
  on-error-unchanged clauses). Ergonomic upside vs Verus: plain
  rustfmt/clippy/rust-analyzer reach the whole crate (no `verusfmt` analog). 18
  trusted items - the largest surface of the wired lanes. Full report:
  [flux.md](flux.md).
- **2026-07-13** - Flux lane scaffolded: `mise/config.flux.toml`
  (nightly-2025-11-25 + rustc-dev/rust-src/llvm-tools/rustfmt/clippy), a
  `flux-tooling` Docker stage that builds Flux from source at a pinned commit
  (`1d98676`) - no release, crates.io crate, or rustup component exists - with
  Z3 4.16 and liquid-fixpoint pinned via mise github backends, a `just flux`
  (`version`/`verify`)
  module, a `flux` sysroot branch in run-tooling.sh, a narrow pre-push glob,
  `cfg(flux)` registered in check-cfg, and a `ticketing-flux` skeleton whose one
  smoke `#[flux_rs::sig]` spec verifies (breaking the output index fails it, so it
  is not vacuous). Cores/contracts pending. Template deviations surfaced up front:
  (1) the verifier is built from source (custom `cargo xtask install`, not a mise
  backend like kani/verus) - no versioned release exists, so it is pinned by
  commit; (2) like Creusot/Verus it is a rustc driver that pins but does not ship
  its nightly, resolved at runtime via `rustup which`; (3) its prebuilt Z3 asset
  needs glibc >= 2.39, so this stage's base is trixie (2.41) like Verus while
  every other lane stays on bookworm; (4) `cargo flux` DOES set `--cfg flux`
  (unlike cargo-verus), so verification-only code is `#[cfg(flux)]`-gated as under
  Kani, but `[target.'cfg(flux)'.dependencies]` does not activate under cargo-flux,
  so the `flux-rs` attribute crate rides in as an unconditional dependency (its
  attributes no-op outside verification) and the crate opts in via
  `[package.metadata.flux] enabled = true`. liquid-fixpoint ships prebuilt binaries
  only under a floating `nightly` tag - the one hermeticity wrinkle. The nonlinear
  `capacity*price <= u64::MAX` - refinement types' known-fragile case - is the open
  question deferred to wire-up.
- **2026-07-12** - Verus toolchain bumped to the newest available build,
  `release/rolling/0.2026.07.12.0b42f4c` + matching crates.io `vstd`
  `2026-07-12`, staying on stable rustc 1.96.0; `just verus verify` still
  `11 verified, 0 errors`. No *stable* release newer than 07-05 exists yet, so
  this is the rolling build - re-pin to the next stable when it lands. Tooling
  then consolidated on mise wherever a backend exists: the Verus bundle installs
  through mise's github backend (`version_prefix = "release/rolling/"` absorbs
  the tag's extra path segment; the zip's single root dir is auto-stripped, so
  the launcher still finds its bundled Z3/vstd next to the binaries), verusfmt
  through ubi, kani-verifier 0.67.0 through the cargo backend, and mise itself
  is pinned (`v2026.7.5`) instead of tracking whatever mise.run serves - the
  curl/unzip layers and `/usr/local/bin` symlinks are gone. Still
  Dockerfile-owned, since no backend can express them: apt packages, Creusot's
  opam build, Kani's `cargo kani setup` bundle, the SBF platform-tools.
  Casualty found on the way: the rolling `cargo-verus` no longer accepts
  `--version`, so `just verus version` (and the image self-test) now run
  `verus --version`. `NonZero` payoff: the `07-12`
  `vstd` ships `std_specs/nonzero.rs`, but gates it behind an **undeclared**
  `nonzero_internals` Cargo feature, so it cannot be enabled from a dependency
  (`cargo` rejects `features = ["nonzero_internals"]`). The verify recipe now
  forces the cfg on (`RUSTFLAGS=--cfg feature="nonzero_internals"`; only `vstd`
  tests it), which retires the lane's hand-rolled `NonZeroU64` registration in
  favor of vstd's model: specs use the canonical `@` view, `new`/`get` ride
  vstd's specs, and the crate-level `#![feature(nonzero_internals)]` gate, the
  `nz_int` axiom, and two of the four `nz_*` wrappers are gone (only
  `checked_mul`/`Ord::min`, unspecced by vstd, stay trusted). Formatting inside
  `verus!{}` (unreachable to rustfmt) is covered by a pinned `verusfmt` (v0.7.2)
  in the tooling image: `just verus fmt` / `fmt-check` run it `--verus-only`
  (nightly rustfmt keeps everything outside the blocks; the two do not fight),
  and `fmt-check` joins `verify` in the pre-push gate.
- **2026-07-11** - Verus lane wired up as a full SBF program: trusted
  `instructions/` shells + dispatcher + `entrypoint!` + `cdylib` around the three
  verified `flows/` cores, behavior-identical to Creusot/Kani. `just verus verify`
  is green (`11 verified, 0 errors`); `just solana build/test-client-rust
  ticketing verus` builds `ticketing_verus.so` and the generated client's Mollusk
  suite passes against it - the same 7 tests that pass against the Creusot and
  Kani `.so`s (the client test now reads `TICKETING_PROGRAM_NAME`, and the
  `just solana build`/`test*` recipes take a `lane` arg). The lane is written as
  idiomatic Verus: the cores live inside `verus!{}` with inline contracts, and
  the `verus!` proc-macro (`verus_builtin_macros`, an ordinary crates.io
  dependency pinned with `vstd`) erases the ghost layer outside `cargo verus`,
  so ordinary/SBF builds compile the same source to the plain program with no
  `vstd` (which cannot link into `no_std` SBF) and zero verification overhead;
  only `vstd` and the foreign-type registrations stay behind the `verus`
  feature. The nonlinear
  `capacity*price <= u64::MAX` is proved *unbounded* (one `by (nonlinear_arith)`,
  no `u8` cap, unlike Kani); non-vacuity mutation-tested (deleting `create_event`'s
  overflow guard turns the run red). Headline wall: `NonZeroU64` (a monomorphic
  `NonZero<u64>` whose bound `ZeroablePrimitive` has a private sealed supertrait)
  cannot be registered normally and `vstd` ships no usable `NonZero` spec
  (`nonzero.rs` landed upstream one day after the release was cut; a later
  crates.io `vstd`, `2026-07-12`, has it but gates it behind an undeclared
  `nonzero_internals` feature, so a dep cannot turn it on) - unblocked with **no
  toolchain bump** via
  `external_trait_private_bound` + a feature-gated `#![feature(nonzero_internals)]`,
  modeling it with Creusot-style axiom shims. *Verifying* the dispatcher byte
  front is deferred. Full report: [verus.md](verus.md).
- **2026-07-10** - Verus lane scaffolded: `mise/config.verus.toml` (stable 1.96.0 +
  rustc-dev/rust-src/llvm-tools), a `verus-tooling` Docker stage (downloads and
  extracts `release/0.2026.07.05.49b8806` with its bundled Z3 4.12.5), `just
  verus` (`version`/`verify`), a narrow pre-push glob, and a `ticketing-verus`
  skeleton whose one smoke `verus!{}` proof verifies (`1 verified, 0 errors`;
  breaking the postcondition fails it, so it is not vacuous). Cores/contracts
  pending. Three template deviations surfaced up front: (1) the stage needs a
  real rustup toolchain - Verus is a rustc driver that pins but does not ship
  rustc; (2) its release binaries need glibc >= 2.39, so this stage's base is
  overridden to `debian:trixie-slim` (2.41) while every other lane stays on
  bookworm (2.36); (3) the lane gates by a `verus` Cargo feature, not
  `[target.'cfg(verus)']` - cargo-verus drives via a RUSTC_WRAPPER and exposes
  no cargo-visible cfg, so no `cfg(verus)` goes in `unexpected_cfgs`, and `vstd`
  rides in as an optional, feature-gated crates.io dependency pinned to the
  bundled version (`=0.0.0-2026-06-14-0213`).
- **2026-07-10** - docs restructured: full Creusot and Kani write-ups moved to
  [creusot.md](creusot.md) and [kani.md](kani.md), this chronology moved to
  `status-log.md`; [provers.md](provers.md) stays the index (roster, ground
  rules, comparison, summaries).
- **2026-07-10** - Kani surface widened to the dispatcher byte front: a plain
  `#[kani::proof]` harness proves `process_instruction` error-only and
  panic-free for every payload up to one past the largest layout, with no
  accounts. It needs the lane's only `#[kani::unwind]` (`BoundedString`
  validation loops over symbolic-length slices, structurally unbounded to
  CBMC) and is now the slowest harness (~300s). `just kani verify` runs
  harnesses in parallel (`-j`, wall-clock ~310s); the notes pick up
  concrete-playback, the `stub_verified` composition caveat, and the paradigm
  contrasts (violation search vs validity proof, harness-scoped obligations
  with an implicit trusted boundary, the proof layer kept out of the program
  text).
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
- **2026-07-08** - Kani lane scaffolded: `mise/config.kani.toml`, `kani-tooling` Docker
  stage, `just kani` (`version`/`verify`), `run-tooling.sh` KANI_HOME plumbing, a
  narrow pre-push glob, `cfg(kani)` registered in check-cfg, and a `ticketing-kani`
  crate with one smoke `#[kani::proof]`. Cores/contracts pending.
- **2026-07-08** - model set to one program crate per prover over the shared
  `ticketing-interface`; the Creusot baseline crate renamed `ticketing-program`
  -> `ticketing-creusot` (SBF artifact `ticketing_creusot.so`, proof sessions
  under `verif/ticketing_creusot_rlib/`).
- **2026-07-08** - roster fixed at eight tools, order set (Kani next),
  `provers.md` created. Creusot baseline already wired (23 proof sessions under
  `verif/`).
