import 'common.just'

mod tooling

# Show the pinned Verus version (diagnostic).
version: tooling::build-verus
    MISE_ENV=verus ./scripts/docker/run-tooling.sh {{ verus_tooling_image }} cargo verus --version

# Verify the ticketing-verus crate (Verus -> bundled Z3). Re-verifies from scratch every run (no
# committed proof artifacts, like the Kani lane). The `verus` feature turns on the vstd import +
# spec surface; ordinary host/SBF builds leave it off and pull zero Verus code.
verify: tooling::build-verus
    MISE_ENV=verus ./scripts/docker/run-tooling.sh {{ verus_tooling_image }} cargo verus verify -p ticketing-verus --features verus
