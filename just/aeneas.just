import 'common.just'

mod tooling

# Every recipe runs inside the lane's tooling image via the shared docker-run wrapper.
run := "MISE_ENV=aeneas ./scripts/docker/run-tooling.sh " + aeneas_tooling_image

# Show the pinned Charon / Aeneas / Lean versions (diagnostic)
version: tooling::build-aeneas
    {{ run }} bash scripts/aeneas/version.sh

# Full pipeline: Charon extracts ticketing-aeneas to an LLBC model, Aeneas translates it to Lean under
# verif/ticketing_aeneas, and lean-check compiles the committed proofs against the freshly regenerated
# code (plain `lean` against the image's prebuilt Aeneas + mathlib oleans - not `lake build`, which
# would re-clone mathlib). This regenerates the committed Lean (commit the diff); `replay` is the
# cheap re-check.
# Extract the Aeneas crate, regenerate Lean, and check the hand-written proofs
verify: tooling::build-aeneas
    {{ run }} bash scripts/aeneas/verify.sh

# Deterministic, fast, and the pre-push gate (mirrors `just creusot replay`): lean-check over the
# committed generated Lean + hand-written proofs, reusing the image's prebuilt packages.
# Replay the committed Aeneas Lean model and proofs without extraction
replay: tooling::build-aeneas
    {{ run }} bash scripts/aeneas/replay.sh
