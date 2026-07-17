#!/usr/bin/env bash
# Extract the ticketing-hax smoke crate to Lean and compile the generated module plus its inline
# proof against the image's prebuilt Hax/Qq oleans. Run inside hax-tooling via run-tooling.sh, from
# the repository root (/workspace).
set -euo pipefail

crate_dir="crates/ticketing/hax"
lean_dir="verif/ticketing_hax"
extraction_dir="${lean_dir}/extraction"
hax_lean="/opt/hax/src/hax-lib/proof-libs/lean/.lake/build/lib/lean"
qq_lean="/opt/hax/src/hax-lib/proof-libs/lean/.lake/packages/Qq/.lake/build/lib/lean"

# Generated output is not a proof artifact: the proof lives in the Rust attribute. Start clean so
# removed/renamed Rust items cannot survive as stale Lean modules and accidentally keep a run green.
rm -rf "${extraction_dir}"
mkdir -p "${extraction_dir}"

( cd "${crate_dir}" && cargo hax into --output-dir "/workspace/${extraction_dir}" lean )

# Drive Lean directly instead of `lake build`: Lake re-resolves Hax's quote4 dependency into this
# workspace and clones it from GitHub on every clean run. Both dependency oleans are baked into the
# image, so LEAN_PATH makes the proof check hermetic and leaves Lake only as an editor descriptor.
( cd "${lean_dir}" && LEAN_PATH="${hax_lean}:${qq_lean}" lean extraction/ticketing_hax.lean )
