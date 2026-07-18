import 'common.just'

mod tooling

# Every recipe runs inside the lane's tooling image via the shared docker-run wrapper.
run := "MISE_ENV=verifast ./scripts/docker/run-tooling.sh " + verifast_tooling_image

# Show the pinned VeriFast version (diagnostic; the version line of the `-help` banner).
version: tooling::build-verifast
    {{ run }} sh -c 'verifast -help | head -n1'

# Smoke-verify the VeriFast lane: bin/verifast drives the pinned rustc nightly to MIR and discharges
# the ticketing-verifast smoke crate's inline separation-logic contract (Redux), proving the Rust ->
# VeriFast pipeline end to end. Re-verifies from source each run; replaced by the real
# ticketing-verifast cores when they land.
verify: tooling::build-verifast
    {{ run }} verifast crates/ticketing/verifast/src/lib.rs
