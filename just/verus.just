import 'common.just'

mod tooling

# Show the pinned Verus version (diagnostic).
version: tooling::build-verus
    MISE_ENV=verus ./scripts/docker/run-tooling.sh {{ verus_tooling_image }} cargo verus --version

# Verify the ticketing-verus crate (Verus -> bundled Z3). Re-verifies from scratch every run (no
# committed proof artifacts, like the Kani lane). The `verus` feature turns on the vstd import +
# spec surface; ordinary host/SBF builds leave it off and pull zero Verus code. The RUSTFLAGS cfg
# force-enables vstd's `std_specs/nonzero.rs` (the `NonZeroU64` model the specs rely on), which
# the published crate gates behind a feature its manifest does not declare; only vstd checks that
# cfg, and its own `verus_keep_ghost`-gated `#![feature(nonzero_internals)]` makes it compile
# under the driver.
verify: tooling::build-verus
    MISE_ENV=verus ./scripts/docker/run-tooling.sh {{ verus_tooling_image }} env 'RUSTFLAGS=--cfg feature="nonzero_internals"' cargo verus verify -p ticketing-verus --features verus
