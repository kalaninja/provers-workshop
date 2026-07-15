/- Hand-written proofs over the Aeneas-generated model of the `ticketing-aeneas` crate
   (`TicketingAeneas.lean`). This is the Aeneas lane's verification artifact, re-checked by
   `just aeneas replay`.

   The wire types (`Event`/`Ticket`/`Address`/`NonZeroU64`) and the field-level glue
   (`domain::glue::*`) are opaque in the extracted model, so we give them trusted specs here - the
   declared trusted computing base, the analog of Flux's `#[extern_spec]`s and Creusot's `#[trusted]`
   shims. Everything else (the `flows`/`domain::event` decision logic) is a real extracted
   definition, and its contracts are proved - including the nonlinear `capacity * price <= u64::MAX`
   bound and the `payment = quantity * price` product (which Flux only *trusts*), and the end-to-end
   `purchase` composition. The contracts mirror the Creusot reference lane's
   (`crates/ticketing/creusot`): exact clamp, exact `sold` delta, initial-state and on-error-unchanged
   characterizations. -/
import TicketingAeneas

open Aeneas Aeneas.Std Result

namespace ticketing_aeneas

/-! ## Abbreviations for the opaque wire types -/

abbrev Ev := ticketing_interface.state.event.Event
abbrev Tk := ticketing_interface.state.ticket.Ticket
abbrev Addr := solana_address.Address
abbrev PErr := solana_program_error.ProgramError
abbrev NZU64 := core.num.nonzero.NonZero Std.U64 core.num.niche_types.NonZeroU64Inner

noncomputable instance : DecidableEq Addr := Classical.decEq _

/-! ## Trusted model of the opaque wire types (the TCB)

Logical projections of an opaque `Event`/`Ticket`, and the specs relating the glue
accessors/setters/constructors to them. Nothing about the wire values is known except through
these. -/

axiom evOrganizer : Ev → Addr
axiom evCreatedSlot : Ev → U64
axiom evUpdatedSlot : Ev → U64
axiom evCapacity : Ev → U64
axiom evPrice : Ev → U64
axiom evSold : Ev → U64
axiom evBump : Ev → U8
axiom evClosed : Ev → Bool

/-- "Address is set" - the uninterpreted not-all-zero predicate (Creusot/Flux/Verus all model it
opaquely). -/
axiom addrIsSet : Addr → Prop

/-- `NonZeroU64` as a positive `u64`. -/
axiom nzVal : NZU64 → U64
axiom nzMk : U64 → NZU64
axiom nzVal_pos (x : NZU64) : 0 < (nzVal x).val
/-- The `nz_from` bridge is the identity on positive values - its `None` arm is dead under the
callers' `v > 0` obligation (which they prove before calling it). -/
axiom nzVal_nzMk (v : U64) (h : 0 < v.val) : nzVal (nzMk v) = v

/-- Ticket projections. -/
axiom tkEvent : Tk → Addr
axiom tkBuyer : Tk → Addr
axiom tkPaid : Tk → NZU64
axiom tkQuantity : Tk → NZU64
axiom tkBump : Tk → U8
axiom tkCreatedSlot : Tk → U64
axiom tkUpdatedSlot : Tk → U64

/-- The event invariant carried for the account's whole life: organizer set, capacity and price
positive, no oversell, and worst-case payment bounded (`capacity * price <= u64::MAX`). -/
def Event.inv (e : Ev) : Prop :=
  addrIsSet (evOrganizer e) ∧
  0 < (evCapacity e).val ∧ 0 < (evPrice e).val ∧
  (evSold e).val ≤ (evCapacity e).val ∧
  (evCapacity e).val * (evPrice e).val ≤ U64.max

-- Accessors: pure reads of the projections.
@[simp] axiom get_capacity_spec (e : Ev) : domain.glue.get_capacity e = ok (evCapacity e)
@[simp] axiom get_price_spec (e : Ev) : domain.glue.get_price e = ok (evPrice e)
@[simp] axiom get_sold_spec (e : Ev) : domain.glue.get_sold e = ok (evSold e)
@[simp] axiom is_open_spec (e : Ev) : domain.glue.is_open e = ok (!evClosed e)
@[simp] axiom organizer_eq_spec (e : Ev) (who : Addr) :
    domain.glue.organizer_eq e who = ok (decide (evOrganizer e = who))
@[simp] axiom nz_get_spec (x : NZU64) : domain.glue.nz_get x = ok (nzVal x)
@[simp] axiom nz_from_spec (v : U64) : domain.glue.nz_from v = ok (nzMk v)

-- Setters: frame + update, preserving the invariant. `set_sold` carries the no-oversell guard
-- (`v <= capacity`), the load-bearing precondition that keeps `sold <= capacity`.
@[step] axiom set_sold_spec (e : Ev) (v : U64) (hv : v.val ≤ (evCapacity e).val) :
    domain.glue.set_sold e v ⦃ e' =>
      (evSold e').val = v.val ∧ evCapacity e' = evCapacity e ∧ evPrice e' = evPrice e ∧
      evOrganizer e' = evOrganizer e ∧ evClosed e' = evClosed e ∧
      evCreatedSlot e' = evCreatedSlot e ∧ evUpdatedSlot e' = evUpdatedSlot e ∧
      evBump e' = evBump e ⦄

@[step] axiom set_status_closed_spec (e : Ev) :
    domain.glue.set_status_closed e ⦃ e' =>
      evClosed e' = true ∧ evCapacity e' = evCapacity e ∧ evPrice e' = evPrice e ∧
      evOrganizer e' = evOrganizer e ∧ evSold e' = evSold e ∧
      evCreatedSlot e' = evCreatedSlot e ∧ evUpdatedSlot e' = evUpdatedSlot e ∧
      evBump e' = evBump e ⦄

-- Constructors: the fresh-event / fresh-ticket postconditions (the analog of the Creusot lane's
-- `is_initial` predicates).
@[step] axiom event_new_spec (organizer : Addr) (c p : U64) (name1 : ticketing_interface.wire.bounded_string.BoundedString 64#usize) (bump : U8) (slot : U64) :
    domain.glue.event_new organizer c p name1 bump slot ⦃ e =>
      evOrganizer e = organizer ∧ evCapacity e = c ∧ evPrice e = p ∧
      (evSold e).val = 0 ∧ evClosed e = false ∧
      evCreatedSlot e = slot ∧ evUpdatedSlot e = slot ∧ evBump e = bump ⦄

@[step] axiom ticket_new_spec (event buyer : Addr) (paid quantity : NZU64) (bump : U8) (slot : U64) :
    domain.glue.ticket_new event buyer paid quantity bump slot ⦃ t =>
      tkEvent t = event ∧ tkBuyer t = buyer ∧ tkPaid t = paid ∧ tkQuantity t = quantity ∧
      tkBump t = bump ∧ tkCreatedSlot t = slot ∧ tkUpdatedSlot t = slot ⦄

/-! ## Proved contracts (the extracted decision cores) -/

/-- `payment_for`: for `quantity <= capacity`, the payment is exactly `quantity * price` and does
not overflow. The nonlinear bound is *proved* here (from the invariant + monotonicity), where Flux
trusts a `mul_le_mono` lemma. -/
@[step]
theorem payment_for_spec (e : Ev) (quantity : U64)
    (hinv : Event.inv e) (hq : quantity.val ≤ (evCapacity e).val) :
    domain.event.payment_for e quantity ⦃ p => p.val = quantity.val * (evPrice e).val ⦄ := by
  unfold domain.event.payment_for
  obtain ⟨_, _, _, _, hcap⟩ := hinv
  simp only [get_price_spec]
  step
  · -- no overflow: quantity * price ≤ capacity * price ≤ u64::MAX
    have : quantity.val * (evPrice e).val ≤ (evCapacity e).val * (evPrice e).val :=
      Nat.mul_le_mul_right _ hq
    omega
  · omega

/-- `create_event`: `Ok` yields an initial event satisfying the invariant (the invariant's base
case); the error is exactly `capacity * price` overflow. The overflow check runs on `u64`
(`checked_mul`, a modeled op), so the bound is proved. -/
theorem create_event_spec (organizer : Addr) (capacity price_lamports : NZU64)
    (name1 : ticketing_interface.wire.bounded_string.BoundedString 64#usize) (bump : U8) (slot : U64)
    (hset : addrIsSet organizer) :
    flows.event_create.create_event organizer capacity price_lamports name1 bump slot ⦃ res =>
      (match res with
      | .Ok e =>
        Event.inv e ∧
        evOrganizer e = organizer ∧ evCapacity e = nzVal capacity ∧
        evPrice e = nzVal price_lamports ∧ (evSold e).val = 0 ∧ evClosed e = false ∧
        evCreatedSlot e = slot ∧ evUpdatedSlot e = slot ∧ evBump e = bump
      | .Err e => e = .ArithmeticOverflow ∧
        U64.max < (nzVal capacity).val * (nzVal price_lamports).val) ⦄ := by
  unfold flows.event_create.create_event
  have hmul := U64.checked_mul_bv_spec (nzVal capacity) (nzVal price_lamports)
  have hc := nzVal_pos capacity
  have hp := nzVal_pos price_lamports
  simp only [nz_get_spec, bind_ok, bind_tc_ok, lift]
  cases hEq : U64.checked_mul (nzVal capacity) (nzVal price_lamports) with
  | none =>
    rw [hEq] at hmul
    try simp only [WP.spec_ok]
    exact ⟨trivial, hmul⟩
  | some z =>
    rw [hEq] at hmul
    obtain ⟨hbound, _, _⟩ := hmul
    simp only [bind_ok, bind_tc_ok]
    step as ⟨e, ho, hcap, hpr, hs, hcl, hcs, hus, hb⟩
    try simp only [WP.spec_ok]
    refine ⟨⟨?_, ?_, ?_, ?_, ?_⟩, ho, hcap, hpr, hs, hcl, hcs, hus, hb⟩
    · rw [ho]; exact hset
    · rw [hcap]; exact hc
    · rw [hpr]; exact hp
    · rw [hcap]; omega
    · rw [hcap, hpr]; exact hbound

/-- `record_sale`: preserves the invariant (so `sold <= capacity` is maintained - no overselling),
changes only `sold`, succeeds exactly when sales are open with headroom left, returns the exact
positive clamp `min(requested, capacity - sold)`, grows `sold` by exactly that quantity, and on
error leaves the event unchanged. -/
@[step]
theorem record_sale_spec (event : Ev) (requested : NZU64) (hinv : Event.inv event) :
    domain.event.record_sale event requested ⦃ (res, event') =>
      Event.inv event' ∧
      evCapacity event' = evCapacity event ∧ evPrice event' = evPrice event ∧
      evOrganizer event' = evOrganizer event ∧ evClosed event' = evClosed event ∧
      (evSold event).val ≤ (evSold event').val ∧
      (match res with
      | .Ok actual =>
        evClosed event = false ∧ 0 < actual.val ∧
        actual.val = min (nzVal requested).val ((evCapacity event).val - (evSold event).val) ∧
        (evSold event').val = (evSold event).val + actual.val ∧
        actual.val ≤ (evCapacity event).val
      | .Err _ =>
        event' = event ∧
        (evClosed event = true ∨ (evSold event).val = (evCapacity event).val)) ⦄ := by
  unfold domain.event.record_sale
  obtain ⟨hset, hcap, hprice, hsold, hbound⟩ := hinv
  have hnz := nzVal_pos requested
  simp only [is_open_spec, get_capacity_spec, get_sold_spec, nz_get_spec, bind_ok, bind_tc_ok]
  cases hopen : evClosed event with
  | true =>
    simp only [Bool.not_true, reduceIte, if_false, Bool.false_eq_true, bind_ok, bind_tc_ok,
      WP.spec_ok]
    refine ⟨⟨hset, hcap, hprice, hsold, hbound⟩, ?_, ?_, ?_, ?_, ?_, ?_, ?_⟩ <;> simp_all
  | false =>
    simp only [Bool.not_false, reduceIte, if_true, bind_ok, bind_tc_ok]
    step as ⟨remaining⟩ <;> try scalar_tac
    rename_i hrem
    split
    · -- remaining = 0: Err (sold at capacity), event unchanged
      try simp only [WP.spec_ok]
      rename_i hzero
      have hsc : (evSold event).val = (evCapacity event).val := by scalar_tac
      refine ⟨⟨hset, hcap, hprice, hsold, hbound⟩, ?_, ?_, ?_, ?_, ?_, ?_, ?_⟩ <;> simp_all
    · rename_i hne
      rw [show (if remaining ≤ nzVal requested then Result.ok remaining
                else Result.ok (nzVal requested))
            = Result.ok (if remaining ≤ nzVal requested then remaining else nzVal requested) from by
          split <;> rfl]
      simp only [bind_ok, bind_tc_ok]
      have hle : (if remaining ≤ nzVal requested then remaining else nzVal requested).val
                  ≤ remaining.val := by split <;> scalar_tac
      have hpos : 0 < (if remaining ≤ nzVal requested then remaining else nzVal requested).val := by
        split <;> scalar_tac
      have hmin : (if remaining ≤ nzVal requested then remaining else nzVal requested).val
                  = min (nzVal requested).val remaining.val := by split <;> scalar_tac
      step as ⟨i⟩ <;> try scalar_tac
      step as ⟨event1⟩ <;> try scalar_tac
      simp_all [Event.inv] <;> first | omega | scalar_tac

/-- `close_sales`: preserves the invariant, changes only `status` (`open -> closed`), succeeds
exactly when sales were open, and on error leaves the event unchanged. -/
@[step]
theorem close_sales_spec (event : Ev) (hinv : Event.inv event) :
    domain.event.close_sales event ⦃ (res, event') =>
      Event.inv event' ∧
      evCapacity event' = evCapacity event ∧ evPrice event' = evPrice event ∧
      evOrganizer event' = evOrganizer event ∧ evSold event' = evSold event ∧
      (match res with
      | .Ok _ => evClosed event = false ∧ evClosed event' = true
      | .Err _ => event' = event ∧ evClosed event = true) ⦄ := by
  unfold domain.event.close_sales
  obtain ⟨hset, hcap, hprice, hsold, hbound⟩ := hinv
  simp only [is_open_spec, bind_ok, bind_tc_ok]
  cases hopen : evClosed event with
  | true =>
    simp only [Bool.not_true, reduceIte, if_false, Bool.false_eq_true, bind_ok, bind_tc_ok,
      WP.spec_ok]
    refine ⟨⟨hset, hcap, hprice, hsold, hbound⟩, ?_, ?_, ?_, ?_, ?_, ?_⟩ <;> simp_all
  | false =>
    simp only [Bool.not_false, reduceIte, if_true, bind_ok, bind_tc_ok]
    step as ⟨event1⟩ <;> try scalar_tac
    simp_all [Event.inv] <;> first | omega | scalar_tac

/-- `close_event`: succeeds exactly when the caller is the organizer and sales are open; then it
moves `open -> closed`, preserving the invariant and changing only `status`; on error the event is
unchanged. -/
theorem close_event_spec (event : Ev) (organizer : Addr) (hinv : Event.inv event) :
    flows.event_close.close_event event organizer ⦃ (res, event') =>
      Event.inv event' ∧
      evCapacity event' = evCapacity event ∧ evPrice event' = evPrice event ∧
      evOrganizer event' = evOrganizer event ∧ evSold event' = evSold event ∧
      (match res with
      | .Ok _ => evOrganizer event = organizer ∧ evClosed event = false ∧ evClosed event' = true
      | .Err _ => event' = event ∧ (evOrganizer event ≠ organizer ∨ evClosed event = true)) ⦄ := by
  unfold flows.event_close.close_event
  have hinv' := hinv
  obtain ⟨hset, hcap, hprice, hsold, hbound⟩ := hinv
  simp only [organizer_eq_spec, bind_ok, bind_tc_ok]
  by_cases horg : evOrganizer event = organizer
  · simp only [horg, decide_true, reduceIte, bind_ok, bind_tc_ok]
    step
    all_goals first | exact hinv' | (simp_all [Event.inv]) | omega | scalar_tac | tauto | trivial
  · simp only [horg, decide_false, Bool.false_eq_true, reduceIte, if_false, bind_ok, bind_tc_ok,
      WP.spec_ok]
    exact ⟨hinv', rfl, rfl, rfl, rfl, rfl, Or.inl horg⟩

/-- `purchase`: the end-to-end ticket-buy contract, mirroring the Creusot reference lane's.
Succeeds exactly under organizer authorization + open sales + headroom; preserves the invariant;
only `sold` changes and it grows by exactly the ticket's clamped quantity
`min(requested, capacity - sold)` (positive); the ticket pins the event/buyer/payment/bump/slots;
the payment is exactly `quantity * price` (no overflow); on error the event is unchanged. -/
theorem purchase_spec (event : Ev) (organizer event_addr buyer : Addr) (requested : NZU64)
    (bump : U8) (slot : U64) (hinv : Event.inv event) :
    flows.ticket_buy.purchase event organizer event_addr buyer requested bump slot ⦃ (res, event') =>
      Event.inv event' ∧
      evCapacity event' = evCapacity event ∧ evPrice event' = evPrice event ∧
      evOrganizer event' = evOrganizer event ∧ evClosed event' = evClosed event ∧
      (evSold event).val ≤ (evSold event').val ∧
      (match res with
      | .Ok (payment, ticket) =>
        evOrganizer event = organizer ∧ evClosed event = false ∧
        tkEvent ticket = event_addr ∧ tkBuyer ticket = buyer ∧ tkPaid ticket = payment ∧
        tkBump ticket = bump ∧ tkCreatedSlot ticket = slot ∧ tkUpdatedSlot ticket = slot ∧
        0 < (nzVal (tkQuantity ticket)).val ∧
        (nzVal (tkQuantity ticket)).val
          = min (nzVal requested).val ((evCapacity event).val - (evSold event).val) ∧
        (evSold event').val = (evSold event).val + (nzVal (tkQuantity ticket)).val ∧
        (nzVal payment).val = (nzVal (tkQuantity ticket)).val * (evPrice event).val
      | .Err _ => event' = event) ⦄ := by
  unfold flows.ticket_buy.purchase
  have hinv' := hinv
  simp only [organizer_eq_spec, bind_ok, bind_tc_ok]
  by_cases horg : evOrganizer event = organizer
  · simp only [horg, decide_true, reduceIte, bind_ok, bind_tc_ok]
    step as ⟨r1, event1, hinv1, hcap1, hprice1, horg1, hclosed1, hmono, hmatch⟩
    cases r1 with
    | Ok actual =>
      obtain ⟨hopen, hapos, hamin, hdelta, hacap⟩ := hmatch
      have hacap1 : actual.val ≤ (evCapacity event1).val := by rw [hcap1]; exact hacap
      have hp1 : 0 < (evPrice event1).val := hinv1.2.2.1
      step as ⟨i, hi⟩
      try simp only [nz_from_spec, bind_ok, bind_tc_ok]
      step as ⟨ticket, htEv, htBuyer, htPaid, htQ, htB, htCS, htUS⟩
      try simp only [WP.spec_ok]
      have hipos : 0 < i.val := by rw [hi]; exact Nat.mul_pos hapos hp1
      have hpay := nzVal_nzMk i hipos
      have hqv := nzVal_nzMk actual hapos
      refine ⟨hinv1, hcap1, hprice1, horg1.trans horg, hclosed1, hmono, hopen,
        htEv, htBuyer, htPaid, htB, htCS, htUS, ?_, ?_, ?_, ?_⟩ <;>
        simp_all
    | Err e =>
      obtain ⟨heq, -⟩ := hmatch
      try simp only [WP.spec_ok]
      exact ⟨hinv1, hcap1, hprice1, horg1.trans horg, hclosed1, hmono, heq⟩
  · simp only [horg, decide_false, Bool.false_eq_true, reduceIte, if_false, bind_ok, bind_tc_ok,
      WP.spec_ok]
    exact ⟨hinv', rfl, rfl, rfl, rfl, Nat.le_refl _, rfl⟩

end ticketing_aeneas
