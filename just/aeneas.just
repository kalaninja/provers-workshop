import 'common.just'

mod tooling

# Show the pinned Charon / Aeneas / Lean versions (diagnostic).
version: tooling::build-aeneas
    MISE_ENV=aeneas ./scripts/docker/run-tooling.sh {{ aeneas_tooling_image }} bash scripts/aeneas/version.sh

# Full pipeline: Charon extracts ticketing-aeneas to an LLBC model, Aeneas translates it to Lean under
# verif/ticketing_aeneas, and lean-check compiles the committed proofs against the freshly regenerated
# code (plain `lean` against the image's prebuilt Aeneas + mathlib oleans - not `lake build`, which
# would re-clone mathlib). This regenerates the committed Lean (commit the diff); `replay` is the
# cheap re-check.
verify: tooling::build-aeneas
    MISE_ENV=aeneas ./scripts/docker/run-tooling.sh {{ aeneas_tooling_image }} bash scripts/aeneas/verify.sh

# Replay the committed Lean proofs without re-running Charon/Aeneas: deterministic, fast, and the
# pre-push gate (mirrors `just creusot replay`). Just lean-check over the committed generated Lean +
# hand-written proofs, reusing the image's prebuilt packages.
replay: tooling::build-aeneas
    MISE_ENV=aeneas ./scripts/docker/run-tooling.sh {{ aeneas_tooling_image }} bash scripts/aeneas/replay.sh
