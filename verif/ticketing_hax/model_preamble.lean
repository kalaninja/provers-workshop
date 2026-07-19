/- Trusted model of the opaque wire types (the trusted computing base).

   `scripts/hax/verify.sh` splices this file into the generated extraction, ahead of the first
   `namespace`. hax's `-i` clause deselects the foreign program crates (`ticketing-interface`,
   `solana-address`, `core`'s `NonZero`), so the generated module references their types without
   declaring them; this file models them:

   - `Address` is a `Nat` (a 32-byte account address as a number; sound over-approximation), so its
     equality is decidable and computable - no runtime-irrelevant `Classical` choice leaks into the
     extracted glue. `is_set` is the not-all-zero predicate every lane models.
   - `NonZeroU64` is the `u64` subtype `{x // x ≠ 0}`, so the positivity the wire type guarantees is
     carried by the model and `nzVal_pos` / `nzVal_nzMk` are *proved*, not axiomatized (a smaller
     trusted surface than the opaque-`NonZero` lanes). `nzMk`'s clamp arm is dead under the callers'
     `v ≠ 0` obligation.
   - `ProgramError` is the two variants the cores build.
   - `Event` / `Ticket` are logical structures over the modeled fields. The padded, self-referential
     header and the reserved bytes carry no invariant, so they are dropped (the same fields Aeneas
     leaves unmodeled).

   This is the hax analog of Aeneas's committed trusted axioms; the glue models in
   `crates/ticketing/hax/src/domain/glue.rs` (via `hax_lib::lean::replace`) are defined over it. -/

namespace solana_address
abbrev Address : Type := Nat
def is_set (a : Address) : Prop := a ≠ 0
end solana_address

/-- `omega` does not bridge `UInt64` disequality to its `toNat`; this does (used for the clamped
    quantity's positivity). -/
theorem toNat_pos_of_ne {a : u64} (h : a ≠ 0) : 0 < a.toNat := by
  have hlt : (0 : u64) < a := UInt64.pos_iff_ne_zero.mpr h
  simpa [UInt64.lt_iff_toNat_lt] using hlt

namespace core_models.num.nonzero
abbrev NonZero (_T : Type) : Type := { x : u64 // x ≠ 0 }
def nzVal (x : NonZero u64) : u64 := x.val
def nzMk (v : u64) : NonZero u64 := if h : v ≠ 0 then ⟨v, h⟩ else ⟨1, by decide⟩
theorem nzVal_pos (x : NonZero u64) : 0 < (nzVal x).toNat := by
  simp only [nzVal]
  have h : (0 : u64) < x.val := UInt64.pos_iff_ne_zero.mpr x.2
  simpa [UInt64.lt_iff_toNat_lt] using h
theorem nzVal_nzMk (v : u64) (h : v ≠ 0) : nzVal (nzMk v) = v := by
  simp [nzVal, nzMk, h]
/-- `nzVal (nzMk v) = v` keyed on `0 < v.toNat` - the positivity form the `purchase` composition
    carries for the clamped quantity and the pricing product (from `record_sale`/`payment_for`'s
    postconditions), so `grind` discharges the `nzMk` round-trips without re-deriving `v ≠ 0`. -/
theorem nzVal_nzMk_of_pos (v : u64) (h : 0 < v.toNat) : nzVal (nzMk v) = v := by
  apply nzVal_nzMk
  intro hv
  subst hv
  simp at h
end core_models.num.nonzero

namespace ticketing_interface.wire.bounded_string
opaque BoundedString : usize → Type
end ticketing_interface.wire.bounded_string

namespace solana_program_error
inductive ProgramError where
  | InvalidAccountData
  | ArithmeticOverflow
deriving DecidableEq
end solana_program_error

namespace ticketing_interface.state.event
structure Event where
  organizer : solana_address.Address
  capacity : u64
  price : u64
  sold : u64
  closed : Bool
  name : ticketing_interface.wire.bounded_string.BoundedString (64 : usize)
  createdSlot : u64
  updatedSlot : u64
  bump : u8
end ticketing_interface.state.event

namespace ticketing_interface.state.ticket
structure Ticket where
  event : solana_address.Address
  buyer : solana_address.Address
  quantity : core_models.num.nonzero.NonZero u64
  paid : core_models.num.nonzero.NonZero u64
  bump : u8
  createdSlot : u64
  updatedSlot : u64
end ticketing_interface.state.ticket

namespace core_models.num.Impl_9
def MAX : u64 := 18446744073709551615
end core_models.num.Impl_9

/- Domain predicates over the logical `Event`, shared by the per-core Hoare-triple theorems
   (`hax_lib::lean::after`). Mirrors the Creusot reference lane's `Event::invariant` and
   `is_legal_transition_from`: the organizer is set, `sold` never exceeds `capacity`, and the
   worst-case payment `capacity * price` fits in `u64`; a legal successor pins every immutable
   field, grows `sold` monotonically, and only steps the status open -> closed (or leaves it). -/
def EventInv (e : ticketing_interface.state.event.Event) : Prop :=
  solana_address.is_set e.organizer ∧
  0 < e.capacity.toNat ∧ 0 < e.price.toNat ∧
  e.sold.toNat ≤ e.capacity.toNat ∧
  e.capacity.toNat * e.price.toNat ≤ 18446744073709551615

/-- The ticket invariant, mirroring the Creusot lane's `Ticket::invariant`: the event and buyer
    references are set. -/
def TicketInv (t : ticketing_interface.state.ticket.Ticket) : Prop :=
  solana_address.is_set t.event ∧ solana_address.is_set t.buyer

def EventLegalTransition
    (after before : ticketing_interface.state.event.Event) : Prop :=
  after.organizer = before.organizer ∧ after.capacity = before.capacity ∧
  after.price = before.price ∧ after.name = before.name ∧
  after.createdSlot = before.createdSlot ∧ after.bump = before.bump ∧
  before.sold.toNat ≤ after.sold.toNat ∧
  (after.closed = before.closed ∨ (before.closed = false ∧ after.closed = true))

/- The nonlinear monotonicity the pricing proofs need: a factor bounded by `capacity` keeps the
   product under the invariant's `capacity * price ≤ u64::MAX` ceiling, so it fits in `u64`
   (`< 2 ^ 64`) and `checked_mul` does not overflow. Proved from `Nat.mul_le_mul` (where Flux trusts
   a `mul_le_mono`), matching `UInt64.mulOverflow_iff`'s `≥ 2 ^ 64` shape for a direct contradiction. -/
theorem mul_no_overflow {a cap b : Nat} (h : a ≤ cap) (hb : cap * b ≤ 18446744073709551615) :
    a * b < 2 ^ 64 := by
  have h1 : a * b ≤ 18446744073709551615 := Nat.le_trans (Nat.mul_le_mul h (Nat.le_refl b)) hb
  have h2 : (18446744073709551615 : Nat) < 2 ^ 64 := by decide
  omega

/- The `event_create` overflow gate is stated with integer division (`c > u64::MAX / p`) rather than
   `checked_mul`, so these two lemmas connect the gate to the multiplicative `capacity * price`
   bound the invariant carries - the `Ok` side (`c ≤ MAX / p`) fits, the `Err` side (`MAX / p < c`)
   overflows. Both proved from `core`'s `Nat` division facts, not axiomatized. -/
theorem le_div_mul_le {c p : Nat} (h : c ≤ 18446744073709551615 / p) :
    c * p ≤ 18446744073709551615 :=
  Nat.le_trans (Nat.mul_le_mul h (Nat.le_refl p)) (Nat.div_mul_le_self 18446744073709551615 p)

theorem div_lt_mul_gt {c p : Nat} (hp : 0 < p) (h : 18446744073709551615 / p < c) :
    18446744073709551615 < c * p :=
  (Nat.div_lt_iff_lt_mul hp).mp h

/- Same monotonicity as `mul_no_overflow`, phrased as the `≤ u64::MAX` bound the ticket-buy pricing
   postconditions carry (a sold quantity `≤ capacity` keeps `quantity * price ≤ u64::MAX`). -/
theorem mul_le_max {a cap b : Nat} (h : a ≤ cap) (hb : cap * b ≤ 18446744073709551615) :
    a * b ≤ 18446744073709551615 :=
  Nat.le_trans (Nat.mul_le_mul h (Nat.le_refl b)) hb
