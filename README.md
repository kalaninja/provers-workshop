# pinocchio-workshop

A small but complete [Pinocchio](https://github.com/anza-xyz/pinocchio) Solana
program - an event-ticketing toy - built the way we build production programs.
It is the companion repository for a three-part article series, one part per
practice:

1. **Hermetic builds** - every command runs inside Docker tooling images
   through `just` + [`mise`](https://mise.jdx.dev); the host needs Docker and
   `just`, nothing else.
2. **Zero-copy wire data** - on-chain state and instruction payloads are
   `#[repr(C)]` structs cast directly from account bytes, with every byte
   validated by `bytemuck::CheckedBitPattern`.
3. **Formal verification** - each instruction's decision core is proved with
   [Creusot](https://github.com/creusot-rs/creusot): no overflow, invariant
   preservation, exact state transitions. The imperative shells stay trusted.

Plus the supporting cast: a [codama](https://github.com/codama-idl/codama) IDL
generated from the interface crate and a rendered Rust client tested against
the program byte-for-byte.

## The series

This repo is the code a three-part series walks through - one part per practice
above. Links land here as each part is published:

| Part | Article | Practice |
| --- | --- | --- |
| 1 | An AI Writes My Solana Programs. Here's the Environment That Makes That Safe | Hermetic builds |
| 2 | Zero-Copy or Bust: Solana Data Models an AI Can't Quietly Break | Zero-copy wire data |
| 3 | I Don't Review My AI's Arithmetic. A Theorem Prover Does | Formal verification |

<!-- On publication, link each title: [<title>](<url>) -->

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

Everything else - Rust (stable + two pinned nightlies), the Agave/SBF
toolchain, node/pnpm for codegen, Creusot/Why3/SMT solvers - lives in the
tooling images built from the repo's `Dockerfile` and pinned by `mise*.toml`.
Images build on demand the first time a recipe needs them; the Creusot image
is the slow one (it compiles Why3 and the provers through opam - expect tens
of minutes once).

## Commands

```sh
just --list                  # everything below, discoverable

just rust fmt / fmt-check    # pinned-nightly rustfmt
just rust check / clippy / test / machete

just solana build            # cargo build-sbf -> target/deploy/ticketing_program.so
just solana test-it          # Mollusk integration tests against the .so
just solana gen-idl          # interface crate -> idl/ticketing.json
just solana gen-client       # idl/ticketing.json -> crates/ticketing/client/src/generated
just solana test-client-rust # generated-client wire parity + SBF lifecycle tests

just creusot verify          # prove all verification conditions (Why3 + SMT)
just creusot replay          # deterministically replay committed proofs (pre-push)
just creusot verify-ide      # open the Why3 IDE on any unproved goal

just init-hooks              # wire the repo-tracked pre-push hook into this clone
```

## Repo map

```
mise.toml                   tool pins (stable Rust, just, nextest, node, pnpm)
mise.nightly.toml           pinned nightly rustfmt        (MISE_ENV=nightly)
mise.solana.toml            pinned Agave / cargo-build-sbf (MISE_ENV=solana)
mise.creusot.toml           Creusot's pinned nightly       (MISE_ENV=creusot)
Dockerfile                  tooling stages: base / rust / node / solana / creusot
scripts/docker/run-tooling.sh   docker-run wrapper every recipe goes through
scripts/hooks/pre-push      change-scoped checks (rust lane, creusot replay)
just/                       recipe modules (rust, solana, creusot, tooling)

crates/ticketing/interface  wire layer: byte layouts + program-side helpers
crates/ticketing/program    the Pinocchio program: trusted shells + verified cores
crates/ticketing/client     codama-rendered Rust client + handwritten checked decoders
crates/ticketing/macros     proc-macros: creusot_extension, codegen derives, no-op codama stand-ins

idl/ticketing.json          committed codama IDL (regenerated via `just solana gen-idl`)
clients/                    codama-js codegen harness (runs in the node image)
verif/                      committed Creusot proof sessions (one proof.json per VC)
why3find.json               prover configuration (alt-ergo, z3, cvc5 + tactics)
```

## How the pieces fit

### Build system

`just` is a thin host-side wrapper: every recipe shells into
`scripts/docker/run-tooling.sh <image> <command>`, which `docker run`s the
matching tooling image with the repo mounted at `/workspace`, your UID/GID,
and per-repo caches under `.cache/`. Inside the container, binaries resolve
through `mise exec` against the committed `mise*.toml` pins - `MISE_ENV`
selects the overlay (`nightly` for rustfmt, `solana` for Agave, `creusot` for
the verifier's nightly). Tools are baked into the images at build time
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

### Proofs

`crates/ticketing/program` is split along the verification boundary:

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
