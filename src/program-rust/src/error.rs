/// error.rs -> program specific errors

use thiserror::Error;

use solana_program::program_error::ProgramError;

#[derive(Error, Debug, Copy, Clone)]
pub enum TrustedPropertiesError {
	/// Invalid instruction
	#[error("Invalid Instruction")]
	InvalidInstruction,

	/// Incorrect amount (deposit or rent payment) as per the agreement
	#[error("Incorrect Payment Amount")]
	IncorrectPaymentAmount,

	/// Rent already paid in full
	#[error("Full Rent Already Paid")]
	RentAlreadyFullyPaid,

	/// Security amount already deposited
	#[error("Security Amount Already Deposited")]
	SecurityAlreadyDeposited,

	/// Rent agreement already terminated
	#[error("Rent Agreement Already Terminated")]
	RentAgreementTerminated,

	/// Invalid agreement status
	#[error("Invalid Agreement Status")]
	InvalidAgreementStatus,

	/// Invalid instruction parameter
	#[error("Invalid Instruction Parameter")]
	InvalidInstructionParameter,
}

impl From<TrustedPropertiesError> for ProgramError {
	fn from(e: TrustedPropertiesError) -> Self {
		ProgramError::Custom(e as u32)
	}
}

