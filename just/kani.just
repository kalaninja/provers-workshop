import 'common.just'

mod tooling

# Show the pinned Kani version and its bundled CBMC backend (diagnostic).
version: tooling::build-kani
    MISE_ENV=kani ./scripts/docker/run-tooling.sh {{ kani_tooling_image }} cargo kani --version

# Bounded model-check the ticketing-kani harnesses through CBMC. Re-verifies from scratch
# every run (no committed proof artifacts); `-p` scopes to the crate, terse output is
# CI-friendly. Add `--solver kissat` if the 64x64 multiplications bit-blast slowly (later).
verify: tooling::build-kani
    MISE_ENV=kani ./scripts/docker/run-tooling.sh {{ kani_tooling_image }} cargo kani -p ticketing-kani --output-format terse
