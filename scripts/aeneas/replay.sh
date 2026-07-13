#!/usr/bin/env bash
# Replay the committed Lean proofs without re-running Charon/Aeneas: just re-check (lean-check) the
# committed generated Lean + hand-written proofs. Deterministic, fast, and the pre-push gate (mirrors
# `just creusot replay`). Run inside the aeneas-tooling image via run-tooling.sh, from the repo root.
set -euo pipefail

bash scripts/aeneas/lean-check.sh
