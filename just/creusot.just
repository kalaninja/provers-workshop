import 'common.just'

mod tooling

# Every recipe runs inside the lane's tooling image via the shared docker-run wrapper.
run := "MISE_ENV=creusot ./scripts/docker/run-tooling.sh " + creusot_tooling_image

# cargo-creusot itself exposes no version flag; the Creusot pin is the release tag in
# docker/creusot.Dockerfile.
# Show the pinned Why3 platform and Z3 versions
version: tooling::build-creusot
    {{ run }} sh -c '/opt/creusot/data/creusot/bin/why3 --version && z3 --version'

# Host-side text scan, no tooling image; a dependency of `verify`/`replay` so CI and the pre-push
# hook run it too (see each account's domain Ops for `is_legal_transition_from`).
# Check Creusot mutable cores for the required legal-transition contract
check-transitions:
    ./scripts/check-ticketing-transitions.sh

# Which crates get translated is set by `[workspace.metadata.creusot] default-members`;
# `why3find.json` and the `verif/` sessions are at the repo root.
# Prove the Creusot program and update its Why3 sessions
verify: check-transitions tooling::build-creusot
    {{ run }} cargo creusot

# No solver search: fails if any committed session (verif/**/proof.json) no longer holds. Used by
# the pre-push hook and CI; dev regenerates sessions via `verify`.
# Replay the committed Creusot proof sessions deterministically
replay: check-transitions tooling::build-creusot
    {{ run }} cargo creusot --replay

# As `verify`, but open the Why3 IDE on any unproved goal (interactive; local only — never in CI)
verify-ide: tooling::build-creusot
    {{ run }} cargo creusot -i
