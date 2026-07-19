//! VeriFast verification model for the ticketing decision cores.
//!
//! This is **not** the shipping program - that lives in `../src` (a real Pinocchio program over
//! `ticketing-interface`, built to `ticketing_verifast.so`). VeriFast's Rust frontend cannot ingest
//! that crate's dependency graph (the external `pinocchio`/`bytemuck`/`ticketing-interface` types),
//! so this file is a self-contained model intended to reproduce the three cores' decisions -
//! `create_event`, `purchase`, `close_event` - verified against `//@` contracts by the built-in
//! Redux prover (`just verifast verify`). It is a standalone VeriFast target outside the crate's
//! module tree; `cargo` never compiles it.
//!
//! The model stays close to the shared state: `Event`/`Ticket` carry the spec-relevant fields, and
//! the wire scalars are modeled as `u64` - `Address` by set-ness (`!= 0`), the `NonZeroU64` fields
//! by an explicit `1 <= x <= u64::MAX` in the invariant, `EventName` by equality, and `EventStatus`
//! by a `u8` tag (0 = open, 1 = closed). This is the same u64-accounting bridge the Flux and Aeneas
//! lanes use for `NonZeroU64`.
//!
//! Two modeling choices carry an honest cost, recorded in `docs/verifast.md`:
//! - the cores are `unsafe fn`, which lets VeriFast verify pure functional `//@ req`/`//@ ens`
//!   contracts (like the other lanes' `requires`/`ensures`) without the ownership-token ceremony a
//!   safe `fn` returning a composite would require. The bodies contain no unsafe operations;
//! - the overflow-checked multiply (`checked_mul_u64`) is `assume_correct` - a trusted axiom, the
//!   analog of Creusot's `ax_checked_mul`. Everything else is proved, including the nonlinear
//!   `capacity * price <= u64::MAX` bound (by hand, via the prelude's `mul_mono_l`).
#![warn(missing_docs)]
#![warn(rustdoc::broken_intra_doc_links)]
// The decision cores are `unsafe fn` only to fit VeriFast's verification fragment (see the module
// docs above); their bodies contain no unsafe operations, so keep the pre-2024 implicit-unsafe-body
// behavior rather than wrapping semantically-empty `unsafe` blocks around internal calls.
#![allow(unsafe_op_in_unsafe_fn)]

/// Errors the decision cores can return, mirroring the subset of `pinocchio`'s
/// `ProgramError` the ticketing program uses.
pub enum ProgramError {
    /// An arithmetic operation would overflow `u64` (the worst-case event
    /// economics).
    ArithmeticOverflow,
    /// The account state rejected the operation (wrong organizer, sales closed,
    /// or sold out).
    InvalidAccountData,
}

/// The event account, modeled with the spec-relevant fields (the wire header
/// and reserved padding, irrelevant to the invariants, are dropped - as in the
/// hax lane's Lean model). The `NonZeroU64` `capacity`/`price_lamports` and the
/// `EventStatus` tag are modeled as `u64`/`u8`; their ranges are carried by
/// `event_invariant`.
pub struct Event {
    /// Slot when the event account was created.
    pub created_slot: u64,
    /// Slot when the event account was last updated.
    pub updated_slot: u64,
    /// Organizer authority (modeled `Address`; set iff non-zero).
    pub organizer: u64,
    /// Total tickets that can be sold (modeled `NonZeroU64`).
    pub capacity: u64,
    /// Price of one ticket in lamports (modeled `NonZeroU64`).
    pub price_lamports: u64,
    /// Tickets sold so far.
    pub sold: u64,
    /// Opaque event-name bytes, represented only by an equality tag.
    pub name: u64,
    /// Canonical event PDA bump.
    pub bump: u8,
    /// Lifecycle status tag: 0 = `SalesOpen`, 1 = `SalesClosed`.
    pub status: u8,
}

/// The per-buyer ticket account, modeled with its spec-relevant fields.
pub struct Ticket {
    /// Slot when the ticket account was created.
    pub created_slot: u64,
    /// Slot when the ticket account was last updated.
    pub updated_slot: u64,
    /// Event this ticket belongs to (modeled `Address`).
    pub event: u64,
    /// Buyer that paid for this ticket (modeled `Address`).
    pub buyer: u64,
    /// Admissions purchased (modeled `NonZeroU64`).
    pub quantity: u64,
    /// Lamports paid, `quantity * price_lamports` (modeled `NonZeroU64`).
    pub paid_lamports: u64,
    /// Canonical ticket PDA bump.
    pub bump: u8,
}

/*@

// --- domain predicates (the formal specification, VeriFast's analog of Creusot's `#[logic]`) ---

fix address_is_set(a: u64) -> bool { a != 0 }

fix int_min(a: u64, b: u64) -> u64 { if a <= b { a } else { b } }

fix is_status_sales_open(e: Event) -> bool { e.status == 0 }
fix is_status_sales_closed(e: Event) -> bool { e.status == 1 }

// Event invariant: organizer set, non-zero bounded capacity/price, no oversell, bounded economics.
fix event_invariant(e: Event) -> bool {
    address_is_set(e.organizer) &&
    1 <= e.capacity && e.capacity <= u64::MAX &&
    1 <= e.price_lamports && e.price_lamports <= u64::MAX &&
    0 <= e.sold && e.sold <= e.capacity &&
    e.capacity * e.price_lamports <= u64::MAX
}

fix is_legal_event_status_transition(from: u8, to: u8) -> bool {
    from == to || (from == 0 && to == 1)
}

// Legal successor relation: immutable fields fixed, sold monotone, status step legal.
fix is_legal_transition_from(e: Event, prev: Event) -> bool {
    e.created_slot == prev.created_slot &&
    e.organizer == prev.organizer &&
    e.capacity == prev.capacity &&
    e.price_lamports == prev.price_lamports &&
    e.name == prev.name &&
    e.bump == prev.bump &&
    prev.sold <= e.sold &&
    is_legal_event_status_transition(prev.status, e.status)
}

// Frame for close_event: only `status` may change.
fix event_frame_except_status(e: Event, prev: Event) -> bool {
    e.created_slot == prev.created_slot &&
    e.updated_slot == prev.updated_slot &&
    e.organizer == prev.organizer &&
    e.capacity == prev.capacity &&
    e.price_lamports == prev.price_lamports &&
    e.sold == prev.sold &&
    e.name == prev.name &&
    e.bump == prev.bump
}

// Frame for purchase: only `sold` may change.
fix event_frame_except_sold(e: Event, prev: Event) -> bool {
    e.created_slot == prev.created_slot &&
    e.updated_slot == prev.updated_slot &&
    e.organizer == prev.organizer &&
    e.capacity == prev.capacity &&
    e.price_lamports == prev.price_lamports &&
    e.name == prev.name &&
    e.bump == prev.bump &&
    e.status == prev.status
}

// The complete initial event state written by event_new.
fix event_is_initial(e: Event, organizer: u64, capacity: u64, price_lamports: u64, name: u64, bump: u8, created_slot: u64) -> bool {
    e.sold == 0 &&
    e.created_slot == created_slot &&
    e.updated_slot == created_slot &&
    e.organizer == organizer &&
    e.capacity == capacity &&
    e.price_lamports == price_lamports &&
    e.name == name &&
    e.bump == bump &&
    event_invariant(e) &&
    is_status_sales_open(e)
}

fix ticket_invariant(t: Ticket) -> bool {
    address_is_set(t.event) && address_is_set(t.buyer)
}

fix ticket_is_initial(t: Ticket, event: u64, buyer: u64, paid_lamports: u64, quantity: u64, bump: u8, created_slot: u64) -> bool {
    t.event == event &&
    t.buyer == buyer &&
    t.paid_lamports == paid_lamports &&
    t.quantity == quantity &&
    t.bump == bump &&
    t.created_slot == created_slot &&
    t.updated_slot == created_slot &&
    ticket_invariant(t)
}

fix ticket_is_consistent_with(t: Ticket, event_addr: u64, e: Event) -> bool {
    t.event == event_addr && t.quantity <= e.sold
}

@*/

// Trusted (`assume_correct`): overflow-checked `NonZeroU64` multiply - the
// analog of Creusot's `ax_checked_mul` axiom. `None` exactly on `u64` overflow,
// else the product.
#[rustfmt::skip]
unsafe fn checked_mul_u64(a: u64, b: u64) -> Option<u64>
//@ req true;
//@ ens match result { Option::None => a * b > u64::MAX, Option::Some(r) => r == a * b && a * b <= u64::MAX };
//@ assume_correct
{
    a.checked_mul(b)
}

#[rustfmt::skip]
unsafe fn event_new(organizer: u64, capacity: u64, price_lamports: u64, name: u64, bump: u8, created_slot: u64) -> Event
//@ req address_is_set(organizer) == true &*& 1 <= capacity &*& 1 <= price_lamports &*& capacity * price_lamports <= u64::MAX;
//@ ens event_is_initial(result, organizer, capacity, price_lamports, name, bump, created_slot) == true;
{
    Event {
        created_slot,
        updated_slot: created_slot,
        organizer,
        capacity,
        price_lamports,
        sold: 0,
        name,
        bump,
        status: 0,
    }
}

/// Verified decision core for `event_create`: build the initial event, rejecting economics whose
/// worst-case payment (`capacity * price_lamports`) would overflow `u64` - the bound the event
/// invariant then carries for the account's whole life.
///
/// # Errors
///
/// Returns `ProgramError::ArithmeticOverflow` when `capacity * price_lamports` overflows `u64`.
///
/// # Safety
///
/// Marked `unsafe` only so VeriFast verifies it in its ownership-token-free fragment; the body has
/// no unsafe operations and the function imposes no runtime safety requirements on the caller.
#[rustfmt::skip]
pub unsafe fn create_event(organizer: u64, capacity: u64, price_lamports: u64, name: u64, bump: u8, created_slot: u64) -> Result<Event, ProgramError>
//@ req address_is_set(organizer) == true &*& 1 <= capacity &*& 1 <= price_lamports;
//@ ens match result { Result::Ok(e) => event_is_initial(e, organizer, capacity, price_lamports, name, bump, created_slot) == true, Result::Err(err) => capacity * price_lamports > u64::MAX };
{
    match checked_mul_u64(capacity, price_lamports) {
        None => Err(ProgramError::ArithmeticOverflow),
        Some(_) => Ok(event_new(organizer, capacity, price_lamports, name, bump, created_slot)),
    }
}

/// Verified decision core for `event_close`: authorize the organizer, then move `SalesOpen ->
/// SalesClosed`. Returns the (possibly unchanged) event and the outcome; on error the event is
/// returned untouched.
///
/// # Safety
///
/// Marked `unsafe` only so VeriFast verifies it in its ownership-token-free fragment; the body has
/// no unsafe operations and the function imposes no runtime safety requirements on the caller.
#[rustfmt::skip]
pub unsafe fn close_event(event: Event, organizer: u64) -> (Event, Result<(), ProgramError>)
//@ req event_invariant(event) == true;
//@ ens event_invariant(result.0) == true &*& is_legal_transition_from(result.0, event) == true &*& match result.1 { Result::Ok(u) => event.organizer == organizer && is_status_sales_open(event) == true && is_status_sales_closed(result.0) == true && event_frame_except_status(result.0, event) == true, Result::Err(err) => (event.organizer != organizer || is_status_sales_open(event) == false) && result.0 == event };
{
    if event.organizer != organizer {
        return (event, Err(ProgramError::InvalidAccountData));
    }
    if event.status != 0 {
        return (event, Err(ProgramError::InvalidAccountData));
    }
    let closed = Event { status: 1, ..event };
    (closed, Ok(()))
}

/// Verified decision core for `ticket_buy`: authorize the organizer, clamp the sale to remaining
/// capacity, price it (overflow-free), and build the buyer's ticket. Returns the updated event and
/// the `(payment, ticket)` pair; on error the event is returned untouched.
///
/// # Safety
///
/// Marked `unsafe` only so VeriFast verifies it in its ownership-token-free fragment; the body has
/// no unsafe operations and the function imposes no runtime safety requirements on the caller.
#[rustfmt::skip]
pub unsafe fn purchase(event: Event, organizer: u64, event_addr: u64, buyer: u64, requested: u64, bump: u8, created_slot: u64) -> (Event, Result<(u64, Ticket), ProgramError>)
//@ req event_invariant(event) == true &*& address_is_set(event_addr) == true &*& address_is_set(buyer) == true &*& 1 <= requested;
//@ ens event_invariant(result.0) == true &*& is_legal_transition_from(result.0, event) == true &*& match result.1 { Result::Ok(pt) => is_status_sales_open(event) == true && event.organizer == organizer && ticket_is_initial(pt.1, event_addr, buyer, pt.0, pt.1.quantity, bump, created_slot) == true && ticket_is_consistent_with(pt.1, event_addr, result.0) == true && result.0.sold == event.sold + pt.1.quantity && pt.1.quantity == int_min(requested, event.capacity - event.sold) && pt.0 == pt.1.quantity * event.price_lamports && event_frame_except_sold(result.0, event) == true, Result::Err(err) => result.0 == event };
{
    if event.organizer != organizer {
        return (event, Err(ProgramError::InvalidAccountData));
    }
    if event.status != 0 {
        return (event, Err(ProgramError::InvalidAccountData));
    }
    let cap = event.capacity;
    let sold0 = event.sold;
    let price = event.price_lamports;
    let remaining = cap - sold0;
    if remaining == 0 {
        return (event, Err(ProgramError::InvalidAccountData));
    }
    let actual = if remaining <= requested { remaining } else { requested };
    // The payment cannot overflow: actual <= capacity and capacity * price <= u64::MAX, so
    // 0 <= actual * price <= capacity * price <= u64::MAX (VeriFast's nonlinear arithmetic needs
    // both monotonicity steps by hand).
    //@ mul_mono_l(0, actual, price);
    //@ mul_mono_l(actual, cap, price);
    let payment = actual * price;
    let new_sold = sold0 + actual;
    let updated = Event { sold: new_sold, ..event };
    let ticket = Ticket {
        created_slot,
        updated_slot: created_slot,
        event: event_addr,
        buyer,
        quantity: actual,
        paid_lamports: payment,
        bump,
    };
    (updated, Ok((payment, ticket)))
}
