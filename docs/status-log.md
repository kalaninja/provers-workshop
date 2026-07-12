# Status log

The dated chronology of [the prover roadmap](provers.md): what landed in each
lane and what it taught, newest first.

- **2026-07-12** - Verus toolchain bumped to the newest available build,
  `release/rolling/0.2026.07.12.0b42f4c` + matching crates.io `vstd`
  `2026-07-12`, staying on stable rustc 1.96.0; `just verus verify` still
  `11 verified, 0 errors`. No *stable* release newer than 07-05 exists yet, so
  this is the rolling build - re-pin to the next stable when it lands. The
  `Dockerfile` release-asset derivation was generalized (`${VERUS_RELEASE##*/}`)
  to handle the extra `rolling/` path segment. `NonZero` payoff: the `07-12`
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
