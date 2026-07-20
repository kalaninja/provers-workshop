import 'common.just'

mod tooling

# Every recipe runs inside the lane's tooling image via the shared docker-run wrapper.
run := "MISE_ENV=hax ./scripts/docker/run-tooling.sh " + hax_tooling_image

# Show the pinned hax frontend / Rust engine / Lean versions (diagnostic)
version: tooling::build-hax
    {{ run }} bash scripts/hax/version.sh

# hax extracts ticketing-hax and emits Lean under verif/ticketing_hax, then Lean compiles the
# generated module and its inline proofs directly against the image's prebuilt Hax/Qq oleans
# (Lake is editor-only because it re-clones Qq). Re-verifies from source on every run; generated
# Lean is intentionally not committed.
# Extract the hax crate to Lean and check its inline proofs from source
verify: tooling::build-hax
    {{ run }} bash scripts/hax/verify.sh
