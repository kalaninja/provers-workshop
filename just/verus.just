import 'common.just'

mod tooling

# Every recipe runs inside the lane's tooling image via the shared docker-run wrapper.
run := "MISE_ENV=verus ./scripts/docker/run-tooling.sh " + verus_tooling_image

# `verus --version`, not `cargo verus --version`: the pinned cargo-verus takes no such flag.
# Show the pinned Verus version
version: tooling::build-verus
    {{ run }} verus --version

# verusfmt reaches inside `verus!{}` blocks, which rustfmt cannot. `--verus-only` leaves code
# outside the macros to `just rust fmt`: the repo's rustfmt.toml uses nightly-only options that
# the image's stable rustfmt would reject.
# Format the verifier-only source inside `verus!` blocks
fmt args='': tooling::build-verus
    {{ run }} sh -c 'find crates/ticketing/verus/src -name "*.rs" -print0 | xargs -0 verusfmt --verus-only {{ args }}'

# Check Verus-lane formatting (the CI/pre-push variant of `fmt`)
fmt-check: (fmt "--check")

# Cargo caches incremental verifier output, but the repository commits no proof artifact. The
# `verus` feature turns on the vstd import + spec surface; ordinary host/SBF builds leave it off
# and pull zero Verus code. The RUSTFLAGS cfg
# force-enables vstd's `std_specs/nonzero.rs` (the `NonZeroU64` model the specs rely on), which
# the published crate gates behind a feature its manifest does not declare; only vstd checks that
# cfg, and its own `verus_keep_ghost`-gated `#![feature(nonzero_internals)]` makes it compile
# under the driver.
# Verify the Verus decision cores and domain operations with bundled Z3
verify: tooling::build-verus
    {{ run }} env 'RUSTFLAGS=--cfg feature="nonzero_internals"' cargo verus verify -p ticketing-verus --features verus
