set shell := ["bash", "-euo", "pipefail", "-c"]
set working-directory := ".."

rust_tooling_image := env_var_or_default("RUST_TOOLING_IMAGE", "workshop-rust-tooling:dev")
solana_tooling_image := env_var_or_default("SOLANA_TOOLING_IMAGE", "workshop-solana-tooling:dev")
node_tooling_image := env_var_or_default("NODE_TOOLING_IMAGE", "workshop-node-tooling:dev")
creusot_tooling_image := env_var_or_default("CREUSOT_TOOLING_IMAGE", "workshop-creusot-tooling:dev")
kani_tooling_image := env_var_or_default("KANI_TOOLING_IMAGE", "workshop-kani-tooling:dev")
verus_tooling_image := env_var_or_default("VERUS_TOOLING_IMAGE", "workshop-verus-tooling:dev")
flux_tooling_image := env_var_or_default("FLUX_TOOLING_IMAGE", "workshop-flux-tooling:dev")
aeneas_tooling_image := env_var_or_default("AENEAS_TOOLING_IMAGE", "workshop-aeneas-tooling:dev")
hax_tooling_image := env_var_or_default("HAX_TOOLING_IMAGE", "workshop-hax-tooling:dev")

# Internal shared bases consumed by the split prover Dockerfiles.
tooling_base_bookworm_image := env_var_or_default("TOOLING_BASE_BOOKWORM_IMAGE", "workshop-tooling-base-bookworm:dev")
tooling_base_trixie_image := env_var_or_default("TOOLING_BASE_TRIXIE_IMAGE", "workshop-tooling-base-trixie:dev")
