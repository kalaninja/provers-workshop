import 'common.just'

mod tooling

# Show the pinned Kani version and its bundled CBMC backend (diagnostic).
version: tooling::build-kani
    MISE_ENV=kani ./scripts/docker/run-tooling.sh {{ kani_tooling_image }} cargo kani --version

# Check the ticketing-kani harnesses through CBMC (`-Z function-contracts`): the three
# flow-core contracts plus the dispatcher byte-front proof, verified in parallel (`-j`,
# wall-clock is bounded by the slowest harness). Re-verifies from scratch every run (no
# committed proof artifacts); `-p` scopes to the crate, terse output is CI-friendly and
# required by `-j`. The harnesses pin the `kissat` backend.
verify: tooling::build-kani
    MISE_ENV=kani ./scripts/docker/run-tooling.sh {{ kani_tooling_image }} cargo kani -p ticketing-kani -Z function-contracts --output-format terse -j
