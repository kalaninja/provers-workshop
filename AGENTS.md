# AGENTS.md

## Read first

- `README.md` for the project overview, quick start, command tour, and repository map.
- `docs/architecture.md` for the shared Solana program, wire protocol, trust boundary, and generated
  client.
- `docs/provers.md` for the prover roster, comparison rules, current statuses, and lane reports.
- For Solana-specific work, load the vendored `solana-dev` skill from
  `.agents/skills/solana-dev/`.

The skill directory is vendored from the source recorded in `skills-lock.json`. Do not hand-edit its
files; update it through `just update-skills`. `.claude/skills/solana-dev` is an intentional symlink
to the same vendored skill.

## Repository layout

- `crates/ticketing/interface` is the verifier-free wire layer: byte layouts, PDA helpers,
  instruction decoding, and the inlined `wire/` and `program/` framework modules.
- `crates/ticketing/<prover>` contains one complete program crate per implemented lane. Creusot is the
  reference split: trusted `instructions/`, verified `flows/`, and predicates and operations under
  `domain/`.
- `crates/ticketing/client` is the Codama-rendered Rust client. Never hand-edit `src/generated/`;
  regenerate it with `just solana gen-client`.
- `crates/ticketing/macros` contains `creusot_extension`, account and argument code-generation derives,
  and no-op Codama derive stand-ins.
- `idl/ticketing.json` is the committed Codama IDL, regenerated with `just solana gen-idl`.
- `clients/` contains the Codama JavaScript generation harness.
- `verif/` contains Creusot proof sessions, the committed Aeneas model and proofs, and hax's Lean
  project. hax extraction output is regenerated and ignored.

## Sources of truth

- `mise/config.toml` and `mise/config.<env>.toml` pin every tool with a usable mise backend: Rust
  toolchains, Node, pnpm, just, Markdown tooling, Agave, Kani, Verus, solvers, elan, hax, and VeriFast.
  Formatting uses `MISE_ENV=nightly`.
- `Dockerfile` and `docker/*.Dockerfile` own apt dependencies, the pinned mise installation, tools
  built from source, prover setup bundles, Lean libraries, and SBF platform tools.
- `Justfile` and `just/*.just` are the public command interface.
- `rustfmt.toml`, `clippy.toml`, `.markdownlint-cli2.jsonc`, and `lychee.toml` define formatting and
  documentation checks.
- `why3find.json` defines the Creusot prover configuration.

## Documentation standard

Project-owned documentation is `README.md`, `AGENTS.md`, `CLAUDE.md`, and `docs/**/*.md`. Vendored skill
references are outside the editorial scope.

- Write for experienced Rust, Solana, and verification readers in American English.
- Use sentence-case headings, the canonical tool capitalization from `docs/provers.md`, and a soft
  100-character line limit outside tables and code blocks.
- Use `Implemented` and `Blocked` for repository statuses; reserve upstream status claims for dated,
  primary-source-backed statements.
- Keep commands complete and copyable. Do not use slash-separated pseudo-commands.
- Distinguish proved behavior, executable test coverage, and trusted correspondence explicitly.
- Put point-in-time performance measurements and their method in `docs/provers.md`; do not duplicate
  volatile timings across lane reports.
- Give each implemented lane report the same structure: metadata, integration, verification coverage,
  trusted surface, key findings, limitations, and reproduction commands.
- Run `just docs check` before handoff. Run `just docs links` when external links or ecosystem claims
  change.

## Adding a lane

There is no single lane registry. A new prover touches this fixed set of places:

- `mise/config.<lane>.toml` for tool pins.
- `docker/<lane>.Dockerfile` for dependencies and an end-to-end verification smoke check.
- `Justfile` for `mod <lane>`.
- `just/<lane>.just` for at least `verify`, plus `version` for diagnostics.
- `just/common.just` for the tooling image variable.
- `just/tooling.just` for `build-<lane>`.
- `scripts/hooks/pre-push` for change detection and the verification gate.
- `README.md`, `docs/provers.md`, and a normalized `docs/<lane>.md` report.

Prefer mise backends over ad hoc installers. Touch `scripts/docker/run-tooling.sh` only when the tool
needs lane-specific runtime environment plumbing.

## Native commands

Run commands from the repository root and prefer recipes over lower-level invocations.

| Task | Command |
| --- | --- |
| List every command | `just --list --list-submodules` |
| Check Markdown and local links | `just docs check` |
| Check external links | `just docs links` |
| Install the pre-push hook | `just init-hooks` |
| Update vendored skills | `just update-skills` |
| Format, check, test, and lint Rust | `just rust fmt-check`, `just rust check`, `just rust test`, `just rust clippy` |
| Build the default SBF program | `just solana build` |
| Run default Mollusk integration tests | `just solana test-it` |
| Generate the IDL and Rust client | `just solana gen-idl`, `just solana gen-client` |
| Test generated-client parity | `just solana test-client-rust` |
| Prove or replay Creusot | `just creusot verify`, `just creusot replay` |
| Extract and prove hax | `just hax verify` |
| Verify the detached VeriFast model | `just verifast verify` |

Build, test, generation, and verification recipes run through Dockerized tooling. The host needs Docker
and just. `init-hooks` and the Creusot transition text check are intentional host-side operations;
`update-skills` is a maintainer command that additionally requires host `npx`.

## Handoff minimums

- For documentation changes, run `just docs check`; also run `just docs links` when links or external
  facts change.
- Before finishing Rust changes, run `just rust fmt-check` and `just rust check`.
- For behavior changes, also run `just rust test`.
- For program or Mollusk changes, also run `just solana test-it`.
- For Creusot specifications or `verif/`, run `just creusot replay`; regenerate sessions with
  `just creusot verify` when specifications change.
- For hax source, tooling, scripts, or Lean project changes, run `just hax verify`.
- For VeriFast model changes, run `just verifast verify`. For VeriFast program changes, also run
  `just solana build ticketing verifast` and `just solana test-client-rust ticketing verifast`.
- Prefer `just rust clippy` before larger submissions.
