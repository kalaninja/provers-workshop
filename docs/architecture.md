# Architecture

This document describes the fixed target shared by every prover lane. The comparison itself lives in
[The provers](provers.md).

## Build and tooling model

`just` is the public command interface. Build, test, code-generation, and prover recipes call
`scripts/docker/run-tooling.sh`, which mounts the repository at `/workspace`, preserves the caller's
UID and GID, and stores reusable caches under `.cache/`.

Inside each image, `mise` resolves repository-pinned tools from `mise/config.toml` and its environment
overlays. `MISE_EXEC_AUTO_INSTALL=false` prevents tools from appearing implicitly at runtime. The
exception is the Codama package graph: `just solana gen-client` runs `pnpm install --frozen-lockfile`
inside the Node image before rendering, so those JavaScript dependencies come from the committed
lockfile rather than being baked into the image.

Three commands intentionally run partly or wholly on the host:

- `just init-hooks` sets the repository-local Git hooks path.
- `just creusot check-transitions` runs a repository text guard.
- `just update-skills` refreshes the vendored agent skill and requires host `npx`.

## Wire protocol

`crates/ticketing/interface` is the source of truth for every byte read or written by the program.

### Wire primitives

- `AccountHeader<T>` contains a seven-byte discriminator and schema version. Only the exact expected
  bytes are accepted as a valid bit pattern.
- `Reserved<N>` models explicit padding and accepts only all-zero bytes.
- `BoundedString<N>` is length-prefixed fixed storage with a canonical zero-filled tail.

All wire types use `#[repr(C)]`, compile-time size and alignment assertions, and
`bytemuck::CheckedBitPattern`. Account access uses checked in-place casts rather than a separate
deserialization allocation.

### State and instruction data

The `Event` and `Ticket` accounts store their PDA seed fields, status, economics, bump, and creation
and update slots. Event names carry bytes rather than a UTF-8 guarantee.

Instruction payloads follow the same checked-layout model. The generated client emits the complete
88-byte `EventCreateArgs`, including six reserved bytes. For compatibility, the parser also accepts
an 82-byte manually encoded form and supplies the omitted trailing zero padding.

### Account helpers

The interface's program module provides:

- checked `StateAccount` borrows;
- `CanonicalPda` and `PdaAccount` derivation tied to the account's seed fields;
- owner, length, signer, writability, and canonical-address validation;
- instruction argument decoding;
- `SlotGuard`, which rejects clock regression and stamps `updated_slot` when a mutation completes.

## Trusted shells and verified cores

Every program crate keeps imperative Solana mechanics outside the decision cores:

```text
instruction shell
  -> parse and validate accounts
  -> call a pure decision core
  -> perform CPI and persist state
```

The shells own account parsing, PDA creation, Clock access, lamport-transfer CPI, and persistence.
The cores decide event creation, ticket purchase, and event closure over wire-level values.

Creusot is the reference specification. Its contracts cover:

- event and ticket invariants;
- exact state frames and legal status transitions;
- exact positive purchase clamping;
- no overselling;
- bounded, overflow-free economics;
- exact error discriminants and on-error unchanged state;
- end-to-end ticket consistency.

Other lanes target that coverage and disclose every tool-driven gap in their lane report. A shared
SBF client suite checks executable protocol parity; it does not claim that independently compiled
program binaries are byte-for-byte identical.

## IDL and client

The interface crate also provides the Codama source annotations. Local no-op derive stand-ins keep
Codama out of normal Rust and SBF dependency graphs.

```text
interface Rust source
  -> just solana gen-idl
  -> idl/ticketing.json
  -> just solana gen-client
  -> crates/ticketing/client/src/generated/
```

`tests/wire_parity.rs` sends generated client encodings through the program's own decoder.
`tests/svm_lifecycle.rs` loads a selected `ticketing_<lane>.so` and exercises the create, buy, and
close lifecycle in Mollusk. Together they currently provide seven shared client tests.

## Proof artifacts

- Creusot commits one `proof.json` session per verification condition and supports deterministic
  replay.
- Aeneas commits its generated Lean model and hand-written proof file; replay checks both without
  re-extraction.
- hax regenerates ignored Lean extraction output and checks inline proofs on every run.
- Kani, Verus, Flux, and VeriFast re-verify from source and commit no proof sessions.

The detailed trusted surface and artifact story for each tool are recorded in the corresponding lane
report.
