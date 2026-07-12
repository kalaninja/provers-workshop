import 'common.just'

mod tooling

# Show the pinned Verus version (diagnostic). `verus --version`, not `cargo verus --version`:
# the pinned rolling cargo-verus takes no such flag.
version: tooling::build-verus
    MISE_ENV=verus ./scripts/docker/run-tooling.sh {{ verus_tooling_image }} verus --version

# Format the Verus lane with pinned verusfmt - it formats inside `verus!{}` blocks, which rustfmt
# cannot reach. `--verus-only` leaves code outside the macros to `just rust fmt`: the repo's
# rustfmt.toml uses nightly-only options that the image's stable rustfmt would reject.
fmt: tooling::build-verus
    MISE_ENV=verus ./scripts/docker/run-tooling.sh {{ verus_tooling_image }} sh -c 'find crates/ticketing/verus/src -name "*.rs" -print0 | xargs -0 verusfmt --verus-only'

# Check Verus-lane formatting (the CI/pre-push variant of `fmt`).
fmt-check: tooling::build-verus
    MISE_ENV=verus ./scripts/docker/run-tooling.sh {{ verus_tooling_image }} sh -c 'find crates/ticketing/verus/src -name "*.rs" -print0 | xargs -0 verusfmt --verus-only --check'

# Verify the ticketing-verus crate (Verus -> bundled Z3). Re-verifies from scratch every run (no
# committed proof artifacts, like the Kani lane). The `verus` feature turns on the vstd import +
# spec surface; ordinary host/SBF builds leave it off and pull zero Verus code. The RUSTFLAGS cfg
# force-enables vstd's `std_specs/nonzero.rs` (the `NonZeroU64` model the specs rely on), which
# the published crate gates behind a feature its manifest does not declare; only vstd checks that
# cfg, and its own `verus_keep_ghost`-gated `#![feature(nonzero_internals)]` makes it compile
# under the driver.
verify: tooling::build-verus
    MISE_ENV=verus ./scripts/docker/run-tooling.sh {{ verus_tooling_image }} env 'RUSTFLAGS=--cfg feature="nonzero_internals"' cargo verus verify -p ticketing-verus --features verus
