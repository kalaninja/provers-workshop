#!/usr/bin/env bash
# Show every independently pinned moving part of the hax -> Lean lane.
set -euo pipefail

cargo hax --version
printf 'hax-rust-engine 0.3.7: %s\n' "$(command -v hax-rust-engine)"
printf 'hax-engine 0.3.7: %s\n' "$(command -v hax-engine)"
lean --version
