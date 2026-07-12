# Creusot lane

Lane 1 of [the prover roadmap](provers.md) - the baseline every later lane
is compared against. Status: **wired up**. Facts are point-in-time; last
reviewed 2026-07-12.

[Creusot](https://github.com/creusot-rs/creusot) translates Rust carrying
pearlite contracts into Why3; `why3find` discharges the verification
conditions with SMT solvers (alt-ergo, z3, cvc5, cvc4 - see `why3find.json`,
with `compute_specified` + `split_vc` tactics).

## What is wired

Contracts on the flow cores and domain ops
(`#[cfg_attr(creusot, requires(...)/ensures(...))]`), pure predicates under
`#[cfg(creusot)]`, trusted axioms for foreign types, 23 committed proof
sessions under `verif/`, `just creusot verify / replay / verify-ide`, and the
pre-push replay gate. The full walkthrough is in the README's "Verified cores"
section.

## Lessons that set the baseline

- Foreign types need an explicit trusted computing base: `NonZeroU64` and
  `Address` are modelled in `src/domain/` by `#[trusted]` shims
  (`ax_checked_mul`, `ax_eq`, ...) tied to opaque logic functions (`int`,
  `is_set`).
- The orphan rule pushes foreign-type contracts through crate-local extension
  traits; the `creusot_extension` proc-macro keeps trait and impl attributes
  from drifting. (The `extern_spec!` route is closed here: the std spec for
  `PartialEq::eq` speaks through `DeepModel`, which a foreign type cannot be
  given from this crate. Contrast with the Verus lane, whose
  `assume_specification` is a verifier-level registration the orphan rule does
  not touch - it pins the same equality axiom onto `<Address as
  PartialEq>::eq` itself; see [verus.md](verus.md).)
- Proof sessions replay deterministically: reviewable artifacts, no solver
  roulette in CI.
- Debugging a failed proof happens outside the source. An unproved goal is not
  a compiler diagnostic pointing at a Rust line - it is an unproved VC in a
  Why3 session, and understanding it means opening the Why3 IDE
  (`just creusot verify-ide`) and reading the goal as translated WhyML: a
  second language and mental model between the programmer and the failure.
  This is the flip side of the ensemble's automation - when the solvers close
  a goal silently the pipeline is invisible, when they do not, the diagnosis
  costs a context switch. The compensating ergonomic win: the exec code is
  plain Rust, so rustfmt, clippy, and rust-analyzer work everywhere.
  (Contrast: Verus reports the same failure as a rustc-style diagnostic with a
  span into the source, often with a suggested fix; see [verus.md](verus.md).)
