import 'common.just'

mod tooling

# Every recipe runs inside the lane's tooling image via the shared docker-run wrapper.
run := "MISE_ENV=creusot ./scripts/docker/run-tooling.sh " + creusot_tooling_image

# Show the baked Why3 platform and z3 versions (diagnostic). cargo-creusot itself exposes no
# version flag; the Creusot pin is the release tag in docker/creusot.Dockerfile.
version: tooling::build-creusot
    {{ run }} sh -c '/opt/creusot/data/creusot/bin/why3 --version && z3 --version'

# Guard: every flow core taking an account by `&mut` must carry its `is_legal_transition_from`
# clause (see each account's domain Ops). Host-side text scan, no tooling image; a dependency
# of `verify`/`replay` so CI and the pre-push hook run it too.
check-transitions:
    ./scripts/check-ticketing-transitions.sh

# Prove the program crate (Why3 + SMT). Which crates get translated is set by
# `[workspace.metadata.creusot] default-members`; `why3find.json` + the `verif/` session are at the repo root.
verify: check-transitions tooling::build-creusot
    {{ run }} cargo creusot

# Replay the committed proof sessions (verif/**/proof.json) without searching — deterministic, fast, and
# fails if any proof no longer holds. Used by the pre-push hook and CI; dev regenerates sessions via `verify`.
replay: check-transitions tooling::build-creusot
    {{ run }} cargo creusot --replay

# As `verify`, but open the Why3 IDE on any unproved goal (interactive; local only — never in CI).
verify-ide: tooling::build-creusot
    {{ run }} cargo creusot -i
