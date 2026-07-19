# provers-workshop

A workshop for putting different Rust formal-verification tools on the *same*
real Solana program and comparing them. The program - a small but complete
[Pinocchio](https://github.com/anza-xyz/pinocchio) event-ticketing toy, built
the way we build production programs - and its verified decision cores are the
fixed target; each prover is wired in behind the same `just` workflow, so its
proof style, ergonomics, and trusted computing base can be judged on realistic
code instead of a synthetic benchmark.

The base program stands on three practices, and the comparison happens on top
of them:

1. **Hermetic builds** - every command runs inside Docker tooling images
   through `just` + [`mise`](https://mise.jdx.dev); the host needs Docker and
   `just`, nothing else.
2. **Zero-copy wire data** - on-chain state and instruction payloads are
   `#[repr(C)]` structs cast directly from account bytes, with every byte
   validated by `bytemuck::CheckedBitPattern`.
3. **Verified cores** - each instruction's decision core is a pure function
   carrying a machine-checked contract: no overflow, invariant preservation,
   exact state transitions, while the imperative shells stay trusted. *This is
   the layer the workshop varies* - swapping out the tool that discharges those
   contracts.

Plus the supporting cast: a [codama](https://github.com/codama-idl/codama) IDL
generated from the interface crate and a rendered Rust client tested against
the program byte-for-byte.

## The provers

The verified cores stay fixed; the tool that proves them is the variable. Each
prover gets its own program crate over the shared `ticketing-interface`, plus
its own `just` lane and proof artifacts, so results sit side by side on
identical contracts.

| # | Prover | Approach | Status |
| --- | --- | --- | --- |
| 1 | [Creusot](https://github.com/creusot-rs/creusot) | deductive, translated to Why3 + SMT (alt-ergo / z3 / cvc5) | wired up |
| 2 | [Kani](https://github.com/model-checking/kani) | bounded model checking (CBMC) | wired up |
| 3 | [Verus](https://github.com/verus-lang/verus) | SMT over a verification dialect of Rust | wired up |
| 4 | [Flux](https://github.com/flux-rs/flux) | refinement (liquid) types | wired up |
| 5 | [Aeneas](https://github.com/AeneasVerif/aeneas) | functional translation to Lean | wired up |
| 6 | [hax](https://github.com/cryspen/hax) | translation to F* / Rocq / Lean | wired up - 5/6 cores proved in Lean |
| 7 | [VeriFast](https://github.com/verifast/verifast) | separation logic | wired up - program + detached model |
| 8 | [Prusti](https://github.com/viperproject/prusti-dev) | deductive, Viper backend | blocked - edition 2024 / resolver 3 unsupported |

Creusot, Kani, Verus, Flux, Aeneas, and VeriFast are wired up today, and the
hax-to-Lean lane proves five of its six cores in Lean (the `ticket_buy`
composition is the one pending proof). The VeriFast lane ships a real program
like the others, but VeriFast cannot verify its cores directly - its Rust
frontend cannot ingest the `ticketing-interface` dependency graph - so its proof
is a detached, behaviorally-identical model of the three cores (see its report). The ordering rationale, integration ground rules, and the running
comparison live in the roadmap index [docs/provers.md](docs/provers.md); each
wired lane has a full report next to it ([docs/creusot.md](docs/creusot.md),
[docs/kani.md](docs/kani.md), [docs/verus.md](docs/verus.md),
[docs/flux.md](docs/flux.md), [docs/aeneas.md](docs/aeneas.md),
[docs/verifast.md](docs/verifast.md)). What we
compare per prover: what has to be marked trusted, how the contracts read,
proof and replay speed, and how far the cores must bend to fit the tool.

## The program

Three instructions over two PDA-backed accounts:

| Instruction | Accounts | What it does |
| --- | --- | --- |
| `event_create` | organizer (signer), event PDA, system program | Creates an `Event` with a fixed ticket `capacity` and `price_lamports` |
| `ticket_buy` | buyer (signer), event, organizer, ticket PDA, system program | Sells up to the remaining capacity (clamped), records a per-buyer `Ticket`, pays the organizer in lamports |
| `event_close` | organizer (signer), event | `SalesOpen -> SalesClosed` |

- `Event` PDA: `["event", organizer]` - one event per organizer.
- `Ticket` PDA: `["ticket", event, buyer]` - one ticket per buyer; its
  existence is the "already bought" signal.

## Prerequisites

- Docker (with BuildKit)
- [`just`](https://github.com/casey/just)

Everything else - pinned Rust toolchains, the Agave/SBF
toolchain, node/pnpm for codegen, the verifiers and their solvers - lives in
the tooling images built from the repo's Dockerfiles. Every tool mise has a
backend for is pinned in `mise*.toml` (toolchains, Agave, kani-verifier, the
Verus bundle, verusfmt, cargo-hax, the hax driver/Rust engine, elan); the root
`Dockerfile` and `docker/*.Dockerfile` own the rest (apt packages, the pinned
mise itself, Creusot's opam build, Kani's setup bundle, SBF platform-tools,
hax's isolated opam-built protocol helper, and the pinned hax Lean proof
library).
Images build on demand the first time a recipe needs them; the Creusot image
is the slow one (it compiles Why3 and the provers through opam - expect tens
of minutes once).

## Commands

```sh
just --list                  # everything below, discoverable

just rust fmt / fmt-check    # pinned-nightly rustfmt
just rust check / clippy / test / machete

just solana build            # cargo build-sbf -> target/deploy/ticketing_creusot.so
just solana test-it          # Mollusk integration tests against the .so
just solana gen-idl          # interface crate -> idl/ticketing.json
just solana gen-client       # idl/ticketing.json -> crates/ticketing/client/src/generated
just solana test-client-rust # generated-client wire parity + SBF lifecycle tests

just creusot verify          # prove all verification conditions (Why3 + SMT)
just creusot replay          # deterministically replay committed proofs (pre-push)
just creusot verify-ide      # open the Why3 IDE on any unproved goal

just kani verify             # bounded model-check the cores (CBMC); re-proves from scratch

just verus verify            # prove the cores (Verus -> bundled Z3); re-proves from scratch
just verus fmt               # format inside verus!{} blocks (pinned verusfmt)
just verus fmt-check         # Verus-lane formatting gate (pre-push)

just flux verify             # refinement-type-check the cores (Flux -> fixpoint/Z3); re-proves from scratch

just aeneas verify           # extract (Charon) -> translate (Aeneas) -> check the Lean proofs
just aeneas replay           # deterministically re-check the committed Lean proofs (pre-push)

just hax verify              # extract Rust -> Lean and compile the inline core proofs
just hax version             # show the pinned hax engine and Lean versions

just verifast verify         # verify the three decision cores (detached VeriFast model)

just init-hooks              # wire the repo-tracked pre-push hook into this clone
```

## Repo map

```
mise/config.toml            tool pins (stable Rust, just, nextest, node, pnpm)
mise/config.<env>.toml      per-lane overlays: nightly, solana, creusot, kani, verus, flux, aeneas, hax, verifast (MISE_ENV=<env>)
Dockerfile                  shared base plus rust / node / solana tooling stages
docker/                     one Dockerfile per prover lane, built from a tagged shared base
scripts/docker/run-tooling.sh   docker-run wrapper every recipe goes through
scripts/hooks/pre-push      change-scoped checks (Rust + each verifier lane)
just/                       recipe modules (rust, solana, creusot, kani, verus, flux, aeneas, hax, verifast, tooling)

crates/ticketing/interface  wire layer: byte layouts + program-side helpers (shared by all provers)
crates/ticketing/creusot    the Creusot program crate: trusted shells + verified cores (one crate per prover)
crates/ticketing/hax        hax-to-Lean lane: inline contracts, 5/6 cores proved in Lean
crates/ticketing/client     codama-rendered Rust client + handwritten checked decoders
crates/ticketing/macros     proc-macros: creusot_extension, codegen derives, no-op codama stand-ins

idl/ticketing.json          committed codama IDL (regenerated via `just solana gen-idl`)
clients/                    codama-js codegen harness (runs in the node image)
verif/                      proof projects/sessions (hax extraction is generated and ignored)
why3find.json               prover configuration (alt-ergo, z3, cvc5 + tactics)
docs/provers.md             prover roadmap index: roster, order, running comparison, summaries
docs/<prover>.md            full per-lane reports (creusot.md, kani.md)
docs/status-log.md          dated chronology of the workshop
```

## How the pieces fit

### Build system

`just` is a thin host-side wrapper: every recipe shells into
`scripts/docker/run-tooling.sh <image> <command>`, which `docker run`s the
matching tooling image with the repo mounted at `/workspace`, your UID/GID,
and per-repo caches under `.cache/`. Inside the container, binaries resolve
through `mise exec` against the committed `mise*.toml` pins - `MISE_ENV`
selects the overlay (`nightly` for rustfmt, `solana` for Agave, or a prover name
such as `creusot`/`hax`). Tools are baked into the images at build time
(`MISE_EXEC_AUTO_INSTALL=false`), so a recipe either runs the pinned tool or
fails loudly.

### Zero-copy data

`crates/ticketing/interface` is the single source of truth for every byte the
program reads or writes:

- `src/wire/` - the primitives: `AccountHeader<T>` (7-byte discriminator +
  schema version, only its exact bytes are a valid bit pattern), `Reserved<N>`
  (explicit padding that must be all-zero), `BoundedString<N>` (length-prefixed
  fixed storage with canonical zero tail).
- `src/state/` - `Event` and `Ticket`: `#[repr(C)]` structs with documented
  layout tables, compile-time size/alignment asserts, and
  `NoUninit + CheckedBitPattern` so account data casts in place with full
  validation (`bytemuck::checked::try_from_bytes`) - no deserialization step,
  no heap.
- `src/args/` - instruction payloads on the same model; `EventCreateArgs`
  shows the `TRAILING_RESERVED` trick (foreign clients may omit the trailing
  alignment pad; the parser zero-fills and re-validates it).
- `src/program/` - the Pinocchio-side helpers: `StateAccount` (checked casts),
  `CanonicalPda`/`PdaAccount` (PDA derivation bound to the account's own seed
  fields, create/read/mutate with owner + length + canonical-address checks),
  and the `InstructionArgs` parser. Every account carries
  `created_slot`/`updated_slot`; mutations open a slot-pinned `SlotGuard`
  borrow that rejects clock regressions up front and stamps `updated_slot` on
  drop.

### Verified cores (Creusot baseline)

`crates/ticketing/creusot` is split along the verification boundary:

- `src/instructions/` - **trusted shells** (`#[trusted]` under Creusot):
  account parsing, PDA creation, clock reads, the lamport-transfer CPI,
  persistence.
- `src/flows/` - **verified cores**: pure functions over wire-level values.
  Their `requires`/`ensures` contracts are the program's specification - e.g.
  `ticket_buy::purchase` proves the sale is clamped to remaining capacity,
  `payment = quantity * price` cannot overflow, the event invariant
  (`sold <= capacity`, bounded economics) is preserved, and on error the event
  is unchanged.
- `src/domain/` - the predicates (`invariant`, `is_initial`,
  `is_status_*`) and typed mutators those contracts are written in, plus the
  trusted axioms modelling foreign types (`NonZeroU64`, `Address`) - the
  proof's explicit trusted computing base.

All specification attributes are `cfg(creusot)`-gated: normal and SBF builds
compile the exact same code with zero verification overhead. `cargo creusot
prove` (in the creusot image) translates the crate to Why3, discharges every
verification condition with SMT solvers, and records one `proof.json` per VC
under `verif/`; `just creusot replay` re-checks those sessions
deterministically and is wired into the pre-push hook.

This is the layer the workshop swaps. A new prover gets its own program crate
(`crates/ticketing/<prover>`) over the same `ticketing-interface`, carrying
identical `src/flows` and `src/domain` core bodies re-expressed in its syntax,
its own trusted shells, and a sibling `just` lane and proof-artifact directory
alongside Creusot's - leaving the program's behavior byte-identical across
prover crates. Its specs must cover no less than the Creusot lane's (more is
welcome); a clause is dropped only where the tool genuinely cannot state it,
recorded in the lane report as an honest tool-limit cost.

### IDL and client

The interface crate doubles as the codama source: `#[derive(Codama*)]` markers
and `#[codama(...)]` attributes sit directly on the wire types (the derives
are no-op stand-ins from `crates/ticketing/macros`, so codama never enters
normal builds). `just solana gen-idl` walks the source AST and writes
`idl/ticketing.json`; `just solana gen-client` renders the Rust client from
that IDL with `@codama/renderers-rust` in the node image. Generated code under
`src/generated/` is never edited by hand - `tests/wire_parity.rs` proves the
client's encodings decode through the program's own decoder, and
`tests/svm_lifecycle.rs` drives the real `.so` end-to-end through the
generated builders.

## License

Apache-2.0 (see `LICENSE`).
