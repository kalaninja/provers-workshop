//! Trusted imperative shells: the dispatcher and one handler per instruction.
//!
//! Everything in this module runs against the Solana runtime - account parsing,
//! PDA creation, clock reads, CPIs, persistence - and is outside Verus
//! verification (plain Rust outside `verus!{}`, external by construction).
//! Each handler keeps its decision logic in a verified `flows` core.
use pinocchio::{AccountView, Address, ProgramResult, error::ProgramError};
use ticketing_interface::{
    EventCreateArgs, TicketBuyArgs, TicketingInstruction,
    program::instruction::{InstructionArgs, NoArgs},
};

mod event_close;
mod event_create;
mod ticket_buy;

/// Dispatches raw instruction data to the matching ticketing instruction
/// handler.
///
/// # Errors
///
/// Returns [`ProgramError::IncorrectProgramId`] when invoked under an address
/// other than the declared program ID (`ticketing_interface::ID`),
/// [`ProgramError::InvalidInstructionData`] when `instruction_data` is empty,
/// its discriminator does not match a known instruction, or its argument bytes
/// are malformed, and propagates any error returned by the dispatched handler.
pub fn process_instruction(
    program_id: &Address,
    accounts: &mut [AccountView],
    instruction_data: &[u8],
) -> ProgramResult {
    if !ticketing_interface::check_id(program_id) {
        return Err(ProgramError::IncorrectProgramId);
    }

    macro_rules! dispatch {
        ($module:ident $(, $arg:expr)*) => {{
            #[cfg(feature = "logging")]
            pinocchio_log::log!("Instruction: {}", stringify!($module));
            $module::process(program_id, accounts $(, $arg)*)
        }};
    }

    let [discriminator, args @ ..] = instruction_data else {
        return Err(ProgramError::InvalidInstructionData);
    };

    match *discriminator {
        TicketingInstruction::EVENT_CREATE => {
            dispatch!(event_create, &EventCreateArgs::parse(args)?)
        },
        TicketingInstruction::TICKET_BUY => {
            dispatch!(ticket_buy, &TicketBuyArgs::parse(args)?)
        },
        TicketingInstruction::EVENT_CLOSE => {
            NoArgs::parse(args)?;
            dispatch!(event_close)
        },
        _ => Err(ProgramError::InvalidInstructionData),
    }
}

#[cfg(test)]
mod tests {
    use pinocchio::{Address, error::ProgramError};
    use ticketing_interface::ID;

    use super::process_instruction;

    #[test]
    fn rejects_empty_instruction_data() {
        let mut accounts = [];

        let result = process_instruction(&ID, &mut accounts, &[]);

        assert_eq!(result, Err(ProgramError::InvalidInstructionData));
    }

    #[test]
    fn rejects_unknown_instruction_discriminator() {
        let mut accounts = [];

        let result = process_instruction(&ID, &mut accounts, &[255]);

        assert_eq!(result, Err(ProgramError::InvalidInstructionData));
    }

    #[test]
    fn rejects_foreign_program_id() {
        let foreign = Address::from([0; 32]);
        let mut accounts = [];

        let result = process_instruction(&foreign, &mut accounts, &[0]);

        assert_eq!(result, Err(ProgramError::IncorrectProgramId));
    }
}
