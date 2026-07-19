import 'common.just'

mod tooling

# Every recipe runs inside the lane's tooling image via the shared docker-run wrapper.
run := "MISE_ENV=verifast ./scripts/docker/run-tooling.sh " + verifast_tooling_image

# Show the pinned VeriFast version (diagnostic; the version line of the `-help` banner).
version: tooling::build-verifast
    {{ run }} sh -c 'verifast -help | head -n1'

# Verify the VeriFast lane: bin/verifast drives the pinned rustc nightly to MIR and discharges the
# separation-logic contracts on the three decision cores' self-contained model
# (`verification/model.rs`). VeriFast's Rust frontend cannot ingest the shipping program's dependency
# graph (pinocchio/bytemuck/ticketing-interface), so it proves a model intended to reproduce the
# shipping cores' decisions rather than the cores themselves - see docs/verifast.md. `-allow_assume`
# enables the lane's one trusted
# axiom, the `assume_correct` overflow-checked multiply (Creusot's `ax_checked_mul` analog).
# Re-verifies from source each run; no committed proof session to replay.
verify: tooling::build-verifast
    {{ run }} verifast -allow_assume crates/ticketing/verifast/verification/model.rs
