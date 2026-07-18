# AGENTS.md

## Read First

- `README.md` for the project overview, command table, and how the pieces fit
- `docs/provers.md` for the prover roadmap index: roster, order, statuses, comparison, per-lane
  summaries; full lane reports are `docs/<prover>.md` (`creusot.md`, `kani.md`), the dated
  chronology is `docs/status-log.md`
- For Solana-specific work, load the vendored `solana-dev` skill from `.agents/skills/solana-dev/`

## Repo Layout

- `crates/ticketing/interface` - wire layer: byte layouts, PDA helpers, instruction decoding, plus
  the inlined `wire/` and `program/` framework modules; shared by every prover crate
- `crates/ticketing/creusot` - the Creusot program crate (one crate per prover): trusted shells
  (`instructions/`), verified cores (`flows/`), domain predicates and axioms (`domain/`)
- `crates/ticketing/client` - codama-rendered Rust client; `src/generated/` is generated output and
  must not be hand-edited (regenerate via `just solana gen-client`)
- `crates/ticketing/macros` - the program's proc-macros: `creusot_extension`, the
  `InstructionAccounts`/`InstructionArgs`/`StateAccount` codegen derives, and no-op codama derive
  stand-ins
- `idl/ticketing.json` - committed codama IDL, regenerated via `just solana gen-idl`
- `clients/` - codama-js codegen harness (runs in the node tooling image)
- `verif/` - proof projects and committed Creusot sessions; hax keeps only its Lake project here
  and regenerates the ignored Lean extraction on every verification run

## Source Of Truth

- `mise/config.toml` plus the `mise/config.<env>.toml` overlays (nightly, solana, creusot, kani,
  verus, flux, aeneas, hax) pin every tool mise has a backend for: the rustup toolchains, node/pnpm/just,
  and the ubi/aqua/cargo/github-backend binaries (cargo-machete, cargo-nextest, Agave, kani-verifier,
  the Verus bundle, verusfmt, Flux's Z3 and liquid-fixpoint, Aeneas's Charon nightly and elan,
  cargo-hax plus its driver/Rust engine and elan);
  formatting uses `MISE_ENV=nightly`
- `Dockerfile` defines the shared base plus Rust/Node/Solana tooling stages;
  `docker/*.Dockerfile` defines one image per prover lane. Together they own what mise cannot express: apt
  packages, the pinned mise itself, Creusot's opam-built toolchain, Kani's `cargo kani setup`
  bundle, Flux's from-source build (flux-driver + precompiled spec libs), Aeneas's opam-built
  Charon+Aeneas and its Lean/mathlib toolchain, hax's isolated opam-built native helper plus pinned
  Lean proof library, and the SBF platform-tools install
- `Justfile` and `just/*.just` are the primary command interface
- `rustfmt.toml` and `clippy.toml` define Rust formatting and lint behavior
- `why3find.json` defines the prover configuration

## Native Commands

Prefer `just` recipes over ad hoc lower-level commands. Run from the repository root.

| Task | Command |
| --- | --- |
| List commands | `just --list` |
| Install pre-push hook | `just init-hooks` |
| Update agent skills | `just update-skills` |
| Rust format / check / test / clippy | `just rust fmt-check`, `just rust check`, `just rust test`, `just rust clippy` |
| Build the SBF program | `just solana build` |
| Mollusk integration tests | `just solana test-it` |
| Generate IDL / Rust client | `just solana gen-idl`, `just solana gen-client` |
| Client wire-parity + SBF tests | `just solana test-client-rust` |
| Prove / replay Creusot sessions | `just creusot verify`, `just creusot replay` |
| Extract and prove the hax Lean lane | `just hax verify` |

## Repo Defaults

- Run commands from the repository root unless a task explicitly requires another directory
- Lint, check, test, build, and verification flows run through the Dockerized tooling; the host
  needs Docker and `just` only
- Inside tooling containers, repository-managed binaries resolve through `mise exec`

## Handoff Minimums

- Before finishing Rust changes, run `just rust fmt-check` and `just rust check`
- For behavior changes, also run `just rust test`
- For changes touching the program or Mollusk integration tests, also run `just solana test-it`
- For changes touching Creusot specs (`domain/`, `flows/`) or `verif/`, run `just creusot replay`
  (regenerate sessions with `just creusot verify` when specs changed)
- For changes touching the hax crate, toolchain, scripts, or Lean project, run `just hax verify`
- Prefer `just rust clippy` before larger submissions
