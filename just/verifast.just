import 'common.just'

mod tooling

# Smoke-verify the VeriFast lane: bin/verifast drives rustc +nightly-2025-11-25 to MIR and discharges
# the ticketing-verifast smoke crate's inline separation-logic contract (Redux), proving the Rust ->
# VeriFast pipeline end to end. Re-verifies from source each run; replaced by the real
# ticketing-verifast cores when they land.
verify: tooling::build-verifast
    MISE_ENV=verifast ./scripts/docker/run-tooling.sh {{ verifast_tooling_image }} verifast crates/ticketing/verifast/src/lib.rs
