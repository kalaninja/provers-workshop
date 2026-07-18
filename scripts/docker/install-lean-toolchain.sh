#!/usr/bin/env bash
# Install the Lean toolchain named by a lean-toolchain file via elan (ELAN_HOME comes from the
# lane env) and expose lean/lake/leanc on PATH. The pinned source tree is the single source of
# truth for the Lean version; shared by the aeneas and hax lane Dockerfiles, bind-mounted into
# the RUN that installs the toolchain (the script content is part of that layer's cache key).
set -euo pipefail

file="${1:?usage: install-lean-toolchain.sh <lean-toolchain-file>}"
toolchain="$(tr -d '[:space:]' < "${file}")"

elan toolchain install "${toolchain}"
tc="$(ls -d "${ELAN_HOME}/toolchains/"*/ | head -n1)"
for b in lean lake leanc; do ln -sf "${tc}bin/${b}" "/usr/local/bin/${b}"; done
lean --version
