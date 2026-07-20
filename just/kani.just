import 'common.just'

mod tooling

# Every recipe runs inside the lane's tooling image via the shared docker-run wrapper.
run := "MISE_ENV=kani ./scripts/docker/run-tooling.sh " + kani_tooling_image

# Show the pinned Kani version and its bundled CBMC backend (diagnostic)
version: tooling::build-kani
    {{ run }} cargo kani --version

# Wall-clock is bounded by the slowest harness. Re-verifies from scratch every run (no committed
# proof artifacts); `-p` scopes to the crate, terse output is CI-friendly and required by `-j`.
# The harnesses pin the `kissat` backend.
# Verify all Kani contracts and the dispatcher harness in parallel
verify: tooling::build-kani
    {{ run }} cargo kani -p ticketing-kani -Z function-contracts --output-format terse -j
