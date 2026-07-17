import 'common.just'

mod tooling

# Show the pinned hax frontend / Rust engine / Lean versions (diagnostic).
version: tooling::build-hax
    MISE_ENV=hax ./scripts/docker/run-tooling.sh {{ hax_tooling_image }} bash scripts/hax/version.sh

# Full smoke pipeline: hax extracts ticketing-hax and emits Lean under verif/ticketing_hax, then Lean
# compiles the generated module and its inline proof directly against the image's prebuilt Hax/Qq
# oleans (Lake is editor-only because it re-clones Qq). Re-verifies from source on every run;
# generated Lean is intentionally not committed.
verify: tooling::build-hax
    MISE_ENV=hax ./scripts/docker/run-tooling.sh {{ hax_tooling_image }} bash scripts/hax/verify.sh
