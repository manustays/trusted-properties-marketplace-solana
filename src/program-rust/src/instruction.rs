/// instruction.rs -> program API, (de)serializing instruction data

use solana_program::{program_error::ProgramError, pubkey::Pubkey};
use std::convert::TryInto;

use crate::error::TrustedPropertiesError::InvalidInstruction;


#[derive(Debug)]
pub enum TrustedPropertiesInstruction {

	/// Initialize the rent contract (with agreed rent amount & duration) and persist initial state in the Rent Agreement account.
	///
	/// * Storing the owner & tenant public-keys ensures that future transactions happen between these parties only.
	///
	/// Accounts expected:
	/// 0. `[writable]` The Rent Agreement account (owned by program_id) created to manage the agreement state for owner & tenant.
	/// 1. `[writable]` The Security Deposit Escrow account (owned by program_id) created to store the tenant's security deposit.
	/// 2. `[]` Sysvar Rent Account to validate rent exemption (SYSVAR_RENT_PUBKEY)
	InitializeRentContract {
		owner_pubkey: Pubkey,
		tenant_pubkey: Pubkey,
		security_escrow_pubkey: Pubkey,
		security_deposit: u64,
		rent_amount: u64,
		duration: u8,
		start_month: u8,
		start_year: u16,
	},

	/// Pay the initial security_deposit amount (tenant -> owner)
	///
	/// Accounts expected:
	/// 0. `[writable]` The Rent Agreement account (owned by program_id) created to manage the agreement state for owner & tenant.
	/// 1. `[signer]` Tenant account (keypair)
	/// 2. `[writable]` The Security Deposit Escrow account (owned by program_id) created to store the tenant's security deposit.
	/// 3. `[]` System program account
	DepositSecurity { security_deposit_amount: u64 },

	/// Pay the rent (tenant -> owner)
	///
	/// Accounts expected:
	/// 0. `[writable]` The Rent Agreement account (owned by program_id) created to manage the agreement state for owner & tenant.
	/// 1. `[signer]` Tenant account (keypair)
	/// 2. `[]` Owner account (public key)
	/// 3. `[]` System program account
	PayRent { rent_amount: u64 },

	/// Terminate agreement early, violating the terms of agreement
	///
	/// Accounts expected:
	/// 0. `[writable]` The Rent Agreement account (owned by program_id) created to manage the agreement state for owner & tenant.
	TerminateEarly {},

	/// Request to extend the contract duration (by the Tenant).
	/// Contract duration can only be extended while the agreement is active.
	///
	/// Accounts expected:
	/// 0. `[writable]` The Rent Agreement account (owned by program_id) created to manage the agreement state for owner & tenant.
	/// 1. `[signer]` Tenant account (keypair)
	RequestContractDurationExtension { extension_duration: u8 },

	/// Confirm the extension of the contract duration (by the Owner).
	/// Contract duration can only be extended while the agreement is active.
	///
	/// Accounts expected:
	/// 0. `[writable]` The Rent Agreement account (owned by program_id) created to manage the agreement state for owner & tenant.
	/// 1. `[signer]` Owner account (keypair)
	ConfirmContractDurationExtension { extension_duration: u8 },
}

impl TrustedPropertiesInstruction {

	/// Unpacks a byte buffer into a [TrustedPropertiesInstruction]
	pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
		let (tag, rest) = input
			.split_first()
			.ok_or(InvalidInstruction)?;

		Ok(match tag {
			// Initialize Rent Agreement Contract
			0 => {
				let owner_pubkey: Pubkey = Pubkey::new(&rest[..32]);
				let tenant_pubkey: Pubkey = Pubkey::new(&rest[32..64]);
				let security_escrow_pubkey: Pubkey = Pubkey::new(&rest[64..96]);
				let security_deposit: u64 = Self::unpack_u64(&rest, 96)?;
				let rent_amount: u64 = Self::unpack_u64(&rest, 104)?;
				let duration: u8 = Self::unpack_u8(&rest, 112)?;
				let start_month: u8 = Self::unpack_u8(&rest, 113)?;
				let start_year: u16 = Self::unpack_u16(&rest, 114)?;

				Self::InitializeRentContract {
					owner_pubkey,
					tenant_pubkey,
					security_escrow_pubkey,
					security_deposit,
					rent_amount,
					duration,
					start_month,
					start_year,
				}
			}

			// Pay Initial Security Deposit (tenant to escrow)
			1 => {
				let security_deposit_amount: u64 = Self::unpack_u64(&rest, 0)?;
				Self::DepositSecurity { security_deposit_amount }
			}

			// Pay Rent (tenant to owner)
			2 => {
				let rent_amount: u64 = Self::unpack_u64(&rest, 0)?;
				Self::PayRent { rent_amount }
			}

			// Terminate the contract early
			3 => Self::TerminateEarly {},

			// Request to extend the contract duration (by Tenant).
			4 => {
				let extension_duration: u8 = Self::unpack_u8(&rest, 0)?;
				Self::RequestContractDurationExtension { extension_duration }
			}

			// Confirm extension of the contract duration (by Owner).
			5 => {
				let extension_duration: u8 = Self::unpack_u8(&rest, 0)?;
				Self::ConfirmContractDurationExtension { extension_duration }
			}

			// Default: Invalid instruction
			_ => return Err(InvalidInstruction.into()),
		})
	}

	// TODO: Is this a necessary step to slice only 1 byte? Find a more efficient solution!
	fn unpack_u8(input: &[u8], start: usize) -> Result<u8, ProgramError> {
		let value = input
			.get(start..8 + start)
			.and_then(|slice| slice.try_into().ok())
			.map(u8::from_le_bytes)
			.ok_or(InvalidInstruction)?;
		Ok(value)
	}

	fn unpack_u16(input: &[u8], start: usize) -> Result<u16, ProgramError> {
		let value = input
			.get(start..8 + start)
			.and_then(|slice| slice.try_into().ok())
			.map(u16::from_le_bytes)
			.ok_or(InvalidInstruction)?;
		Ok(value)
	}

	fn unpack_u64(input: &[u8], start: usize) -> Result<u64, ProgramError> {
		let value = input
			.get(start..8 + start)
			.and_then(|slice| slice.try_into().ok())
			.map(u64::from_le_bytes)
			.ok_or(InvalidInstruction)?;
		Ok(value)
	}
}
