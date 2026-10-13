import 'common.just'

mod tooling

# Every recipe runs inside the lane's tooling image via the shared docker-run wrapper.
run := "MISE_ENV=flux ./scripts/docker/run-tooling.sh " + flux_tooling_image

# Show the pinned Flux version (diagnostic)
version: tooling::build-flux
    {{ run }} cargo flux --version

# Re-verifies from scratch every run (no committed proof artifacts, like the Kani/Verus lanes).
# `cargo flux` sets `--cfg flux` and only checks the crate because it opts in via
# `[package.metadata.flux] enabled = true`; the whole dependency graph is recompiled under Flux's
# pinned nightly in a dedicated, non-incremental profile.
# Refinement-check the Flux decision cores and domain operations
verify: tooling::build-flux
    {{ run }} env FLUXFLAGS="-Fstd-extern-specs=on -Fcheck-overflow=strict -Ldependency=/opt/flux" cargo flux -p ticketing-flux
