/- Hand-written proofs over the Aeneas-generated model of the `ticketing-aeneas` crate.
   This is the Aeneas lane's actual verification artifact (committed and re-checked by
   `just aeneas replay`); the generated model lives in `TicketingAeneas.lean`. For now it holds the
   one smoke proof that makes `just aeneas verify` exercise the whole toolchain end to end. -/
import TicketingAeneas

open Aeneas Aeneas.Std Result

namespace ticketing_aeneas

/-- Smoke proof: for `x ≤ 1000`, `smoke_triple` succeeds (no overflow) and returns `3 * x`.
    The bound makes it non-vacuous - drop it, or widen the crate body past `3 * x`, and the
    multiplication can overflow, so the proof fails. -/
theorem smoke_triple_spec (x : Std.U32) (h : x.val ≤ 1000) :
    smoke_triple x ⦃ y => y.val = 3 * x.val ⦄ := by
  unfold smoke_triple
  step
  scalar_tac

end ticketing_aeneas
