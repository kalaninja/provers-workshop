#!/usr/bin/env bash
# Show every independently pinned moving part of the hax -> Lean lane.
set -euo pipefail

cargo hax --version
printf 'hax-rust-engine: %s\n' "$(command -v hax-rust-engine)"
printf 'hax-engine: %s\n' "$(command -v hax-engine)"
lean --version
