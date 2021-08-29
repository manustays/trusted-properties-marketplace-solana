/**
 * entrypoint.rs -> entrypoint to the program
 *
 * Program Flow:
 * 		1. entrypoint is called
 * 		2. entrypoint forwards args to processor
 * 		3. processor asks `instruction.rs` to decode the `instruction_data` argument from the entrypoint function
 * 		4. Using the decoded data, the processor will now decide which processing function to use to process the request
 * 		5. The processor may use state.rs to encode state into or decode the state of an account which has been passed into the entrypoint
 */

use solana_program::{
	account_info::AccountInfo, entrypoint, entrypoint::ProgramResult, pubkey::Pubkey
};

use crate::processor::Processor;

// * Using the entrypoint! macro to declare the process_instruction function the entrypoint to the program.
// * All calls go through the function declared as the entrypoint (`process_instruction`, in this case).
// * When called, a program is passed to its BPF Loader which processes the call.
// * Different BPF loaders may require different entrypoints.
entrypoint!(process_instruction);

// ## Accounts:
// 		* Accounts are used to store state (as Solana programs are stateless by default).
// 		* Each account can hold data & SOL.
// 		* Each account also has an owner and only the owner may debit the account and adjust its data.
// 		* Accounts can only be owned by programs. Eg: the `system_program`.
// 		* All accounts to be read or written to must be passed into the entrypoint function.
// 			* This allows the runtime to parallelise transactions.
// 			* Transactions can run in parallel that do not touch the same accounts
// 				or, touch the same accounts but only read and don't write.
pub fn process_instruction(
	program_id: &Pubkey,
	accounts: &[AccountInfo],
	instruction_data: &[u8],
) -> ProgramResult {
	Processor::process(program_id, accounts, instruction_data)
}
