#!/usr/bin/env bash
# Extract the ticketing-hax decision cores to Lean and compile the generated modules plus their
# inline proofs against the image's prebuilt Hax/Qq oleans. Run inside hax-tooling via
# run-tooling.sh, from the repository root (/workspace).
set -euo pipefail

crate_dir="crates/ticketing/hax"
lean_dir="verif/ticketing_hax"
extraction_dir="${lean_dir}/extraction"
hax_lean="/opt/hax/src/hax-lib/proof-libs/lean/.lake/build/lib/lean"
qq_lean="/opt/hax/src/hax-lib/proof-libs/lean/.lake/packages/Qq/.lake/build/lib/lean"

# Scope extraction to the verified surface: deselect everything, then re-include the `flows` cores
# and the `domain` layer (`domain::glue`'s extraction is swapped for its trusted `lean::replace`
# models). This reproduces the Aeneas lane's `start_from = ["crate::flows"]`; the imperative
# `instructions/` shells and the `entrypoint!` are never translated.
include='-** +ticketing_hax::flows::** +ticketing_hax::domain::**'

# Generated output is not a proof artifact: the proof lives in the Rust attribute. Start clean so
# removed/renamed Rust items cannot survive as stale Lean modules and accidentally keep a run green.
rm -rf "${extraction_dir}"
mkdir -p "${extraction_dir}"

( cd "${crate_dir}" && cargo hax into -i "${include}" --output-dir "/workspace/${extraction_dir}" lean )

# Splice the committed trusted wire-type model into the generated module, just ahead of the first
# `namespace`: the `-i` clause deselects the foreign program crates, so their types are referenced
# but undeclared, and this file (the declared trusted computing base) supplies the logical model the
# glue `lean::replace` definitions and the proofs are written against.
gen="${extraction_dir}/ticketing_hax.lean"
awk 'NR==FNR { pre = pre $0 ORS; next }
     /^namespace / && !spliced { printf "%s", pre; spliced = 1 }
     { print }' "${lean_dir}/model_preamble.lean" "${gen}" > "${gen}.tmp"
mv "${gen}.tmp" "${gen}"

# Drive Lean directly instead of `lake build`: Lake re-resolves Hax's quote4 dependency into this
# workspace and clones it from GitHub on every clean run. Both dependency oleans are baked into the
# image, so LEAN_PATH makes the proof check hermetic and leaves Lake only as an editor descriptor.
( cd "${lean_dir}" && LEAN_PATH="${hax_lean}:${qq_lean}" lean extraction/ticketing_hax.lean )
