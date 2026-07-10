# Status log

The dated chronology of [the prover roadmap](provers.md): what landed in each
lane and what it taught, newest first.

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
