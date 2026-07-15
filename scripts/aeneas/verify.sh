#!/usr/bin/env bash
# Full Aeneas pipeline over the ticketing-aeneas crate: Charon extraction -> Aeneas translation to
# Lean -> lean-check. Regenerates the committed Lean under verif/ticketing_aeneas (commit the diff).
# Run inside the aeneas-tooling image via run-tooling.sh, from the repo root (/workspace).
set -euo pipefail

crate_dir="crates/ticketing/aeneas"
lean_dir="verif/ticketing_aeneas"
llbc_dir="target/aeneas"
llbc="${llbc_dir}/ticketing_aeneas.llbc"

mkdir -p "${llbc_dir}"

# 1. Charon: extract the crate to an LLBC model. The aeneas preset selects the options Aeneas expects.
#
# --sysroot default: the crate is a `cdylib` (for `cargo build-sbf`), and Charon's default MIR-only
# sysroot cannot LINK one on the host (its std libs carry MIR, not compiled objects), so cargo's
# cdylib link fails on undefined runtime symbols. The normal distributed sysroot links fine; std/core
# stay opaque either way (we don't extract them - Aeneas maps `checked_mul`/`*` via its builtin table).
# CARGO_TARGET_DIR: that sysroot's std differs from the stable rust-tooling toolchain's, so Charon's
# artifacts are isolated from `just rust check`/`just solana build` to avoid fingerprint clashes.
( cd "${crate_dir}" && CARGO_TARGET_DIR=/workspace/target/aeneas-charon \
    charon cargo --preset=aeneas --sysroot default --dest-file "/workspace/${llbc}" )

# 2. Aeneas: translate the LLBC to Lean, written into the proof project's source tree.
aeneas -backend lean "${llbc}" -dest "${lean_dir}"

# 3. Check the (regenerated) generated Lean + committed hand-written proofs.
bash scripts/aeneas/lean-check.sh
