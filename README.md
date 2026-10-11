# Provers Workshop

Provers Workshop compares Rust formal-verification tools on the same nontrivial target: a small,
complete [Pinocchio](https://github.com/anza-xyz/pinocchio) event-ticketing program for Solana.
Each implemented lane preserves one wire protocol and one set of decision semantics, then expresses
and verifies those decisions in the prover's native style.

This project continues [Pinocchio Workshop](https://github.com/kalaninja/pinocchio-workshop),
extending its Creusot-verified ticketing example into a comparison of seven Rust verification tools.

The program is deliberately a workshop target, not production software. Its architecture is
production-oriented—zero-copy accounts, canonical PDAs, checked state transitions, a generated IDL,
and an SBF-tested client—but the committed program ID is not deployable and the code has not received
a production security audit.

## What this repository compares

Every lane is evaluated on the same questions:

- Which code and models must be trusted?
- How legible are the contracts?
- Is verification bounded or unbounded, and what properties are omitted?
- How repeatable are proof artifacts and verification runs?
- How much must ordinary Rust bend to fit the tool?
- How difficult is the toolchain to pin and reproduce?

The detailed comparison and dated measurements live in
[the prover index](docs/provers.md). Each implemented lane has a full report under `docs/`.

| # | Prover | Approach | Repository status |
| --- | --- | --- | --- |
| 1 | [Creusot](https://github.com/creusot-rs/creusot) | Deductive verification through Why3 and SMT | Implemented baseline |
| 2 | [Kani](https://github.com/model-checking/kani) | Bounded model checking through CBMC | Implemented |
| 3 | [Verus](https://github.com/verus-lang/verus) | SMT over a verification dialect of Rust | Implemented |
| 4 | [Flux](https://github.com/flux-rs/flux) | Refinement types through liquid-fixpoint and Z3 | Implemented |
| 5 | [Aeneas](https://github.com/AeneasVerif/aeneas) | Rust translation and interactive Lean proofs | Implemented |
| 6 | [hax](https://github.com/cryspen/hax) | Rust translation to inline Lean proofs | Implemented |
| 7 | [VeriFast](https://github.com/verifast/verifast) | Separation logic and symbolic execution | Implemented program plus detached model |
| 8 | [Prusti](https://github.com/viperproject/prusti-dev) | Deductive verification through Viper | Blocked by edition 2024 and resolver 3 |

All seven implemented program crates build to SBF and pass the same seven-test generated-client
suite. Six lanes verify the shipping decision cores directly or through generated models. VeriFast
is the explicit exception: its Rust frontend cannot import the interface crate's type definitions,
so it verifies a detached transcription of the three top-level cores. That model-to-program
correspondence remains trusted.

## The program

The shared protocol has three instructions and two PDA-backed account types:

| Instruction | Accounts | Behavior |
| --- | --- | --- |
| `event_create` | Organizer, event PDA, system program | Create an event with fixed capacity and price |
| `ticket_buy` | Buyer, event, organizer, ticket PDA, system program | Clamp a purchase to remaining capacity and pay the organizer |
| `event_close` | Organizer, event | Transition `SalesOpen` to `SalesClosed` |

- Event PDA: `["event", organizer]`—one event per organizer.
- Ticket PDA: `["ticket", event, buyer]`—one ticket per buyer and event.
- Event names are limited to 64 bytes. The program preserves bytes but does not validate UTF-8;
  clients must validate before treating a name as text.

See [Architecture](docs/architecture.md) for account layouts, the trusted-shell/verified-core split,
the build system, proof artifacts, and IDL/client generation.

## Prerequisites

- Docker with BuildKit
- [`just`](https://github.com/casey/just)

Build, test, code-generation, and verification recipes run in pinned Docker tooling images. The
host-side exceptions are `just init-hooks`, the Creusot transition text check, and the maintainer-only
`just update-skills` command; the last one also requires `npx` on the host.

The first invocation builds its tooling image. Creusot, Aeneas, and hax compile substantial OCaml or
Lean stacks and can take tens of minutes on a cold machine.

## Quick start

```sh
just --list --list-submodules
just rust check
just rust test
just solana test-it
just creusot replay
```

The default Solana and Creusot commands target the baseline `ticketing-creusot` lane.

## Commands

```sh
# Documentation
just docs check
just docs links

# Rust workspace
just rust fmt
just rust fmt-check
just rust check
just rust clippy
just rust test
just rust machete

# Solana program and generated client
just solana build
just solana test-it
just solana gen-idl
just solana gen-client
just solana test-client-rust

# Creusot
just creusot verify
just creusot replay
just creusot verify-ide

# Other prover lanes
just kani verify
just verus verify
just verus fmt-check
just flux verify
just aeneas verify
just aeneas replay
just hax verify
just verifast verify
```

To test another SBF lane against the shared client, pass the package and lane explicitly:

```sh
just solana test-client-rust ticketing verus
```

## Repository map

```text
crates/ticketing/interface   Shared wire layouts, PDA helpers, and instruction decoding
crates/ticketing/<prover>    Seven program crates over the same interface
crates/ticketing/client      Generated Rust client plus checked decoders and SBF tests
crates/ticketing/macros      Program and verification proc-macros

idl/ticketing.json           Committed Codama IDL
clients/                     Codama JavaScript generation harness
verif/                       Creusot sessions and Lean proof projects

mise/                        Pinned host-independent tool versions
Dockerfile, docker/          Shared and prover-specific tooling images
Justfile, just/              Public command interface
scripts/                     Tooling wrapper, proof pipelines, and pre-push hook
docs/                        Architecture, comparison, and lane reports
```

Generated files under `crates/ticketing/client/src/generated/` and the Aeneas generated Lean model
must not be hand-edited. Regenerate them through their documented `just` recipes.

## Deployment warning

The committed program ID is an example address whose keypair is not retained. Before deploying,
replace `package.metadata.solana.program-id` in the interface manifest, then regenerate the IDL and
client. No deployment recipe in this repository should be treated as a production release process.

## License

Apache-2.0. See [LICENSE](LICENSE).
