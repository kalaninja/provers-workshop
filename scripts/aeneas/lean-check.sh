#!/usr/bin/env bash
# Type-check the committed Lean proof project against the aeneas-tooling image's prebuilt Aeneas +
# mathlib libraries. Shared by verify.sh and replay.sh; run inside the image via run-tooling.sh.
#
# We compile with `lean` directly rather than `lake build`. Lake insists on re-resolving the
# dependency graph on every invocation, and for a downstream project that (transitively) requires
# mathlib it re-clones mathlib and friends from the network each run ("URL has changed; ... cloning
# again") - not hermetic, and slow. Instead we point LEAN_PATH at the oleans the image already built
# under /opt/aeneas/src/backends/lean/.lake (the Aeneas library plus its mathlib dependency tree) and
# invoke `lean` module by module. No network, no re-clone; the baked build is reused as-is.
set -euo pipefail

aeneas_lean=/opt/aeneas/src/backends/lean
proj=verif/ticketing_aeneas

# Search path: the Aeneas library, then every prebuilt dependency package (mathlib, batteries, ...).
lean_path="${aeneas_lean}/.lake/build/lib/lean"
for d in "${aeneas_lean}"/.lake/packages/*/.lake/build/lib/lean; do
  lean_path="${lean_path}:${d}"
done

out="${proj}/.lake/build/lib"
mkdir -p "${out}"
# Our own build dir goes first so the proofs module resolves the freshly built model.
export LEAN_PATH="/workspace/${out}:${lean_path}"

# Compile the generated model first, then the hand-written proofs that import it. `lean` exits
# non-zero on any error, so a failed proof fails the check.
lean -o "${out}/TicketingAeneas.olean" "${proj}/TicketingAeneas.lean"
lean -o "${out}/TicketingAeneasProofs.olean" "${proj}/TicketingAeneasProofs.lean"

echo "aeneas: Lean proofs check out"
