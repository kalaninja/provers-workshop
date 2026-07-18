# Rust commands
mod rust 'just/rust.just'

# Solana program commands
mod solana 'just/solana.just'

# Formal verification commands (Creusot)
mod creusot 'just/creusot.just'

# Bounded model checking commands (Kani)
mod kani 'just/kani.just'

# SMT-checked Rust dialect commands (Verus)
mod verus 'just/verus.just'

# Refinement (liquid) types commands (Flux)
mod flux 'just/flux.just'

# Interactive-proof (Charon -> Aeneas -> Lean) commands (Aeneas)
mod aeneas 'just/aeneas.just'

# Translation-to-Lean commands (hax)
mod hax 'just/hax.just'

# Separation-logic commands (VeriFast)
mod verifast 'just/verifast.just'

# Tooling commands
mod tooling 'just/tooling.just'

# Lists available recipes
[default]
list:
    just --list

# Wire repo-tracked git hooks into this clone
init-hooks:
    git config core.hooksPath scripts/hooks
    @echo "Hooks installed via core.hooksPath -> scripts/hooks"

# Update the vendored agent skills to their latest versions (refreshes skills-lock.json)
update-skills:
    npx --yes skills update --project --yes
