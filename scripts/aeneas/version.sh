#!/usr/bin/env bash
# Diagnostic: the pinned Charon / Aeneas / Lean versions in the aeneas-tooling image.
set -euo pipefail

printf 'charon:  '; charon version
printf 'aeneas:  '; aeneas -version
printf 'lean:    '; lean --version
