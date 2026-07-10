set shell := ["bash", "-euo", "pipefail", "-c"]
set working-directory := ".."

rust_tooling_image := env_var_or_default("RUST_TOOLING_IMAGE", "workshop-rust-tooling:dev")
solana_tooling_image := env_var_or_default("SOLANA_TOOLING_IMAGE", "workshop-solana-tooling:dev")
node_tooling_image := env_var_or_default("NODE_TOOLING_IMAGE", "workshop-node-tooling:dev")
creusot_tooling_image := env_var_or_default("CREUSOT_TOOLING_IMAGE", "workshop-creusot-tooling:dev")
kani_tooling_image := env_var_or_default("KANI_TOOLING_IMAGE", "workshop-kani-tooling:dev")
verus_tooling_image := env_var_or_default("VERUS_TOOLING_IMAGE", "workshop-verus-tooling:dev")
