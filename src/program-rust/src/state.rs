/// state.rs -> program objects, (de)serializing state

use solana_program::{
	program_pack::{IsInitialized, Sealed},
	pubkey::Pubkey,
};
use borsh::{BorshDeserialize, BorshSerialize};


/* ==========================================================================
					Account State: Rent Agreement
============================================================================= */

/// Renting state stored in the Agreement Account
/// Recording the owner & tenant public keys to ensure that future transactions happen between these parties only.
#[derive(BorshSerialize, BorshDeserialize, Debug)]				// Traits to (de)serialize & debug
pub struct RentAgreementAccount {

	/// Agreement status (active, complete, terminated, etc)
	pub status: u8,

	/// Property owner account's public-key
	pub owner_pubkey: Pubkey,

	/// Tenant account's public-key
	pub tenant_pubkey: Pubkey,

	/// Security-deposit escrow account's public-key
	pub security_escrow_pubkey: Pubkey,

	/// Minimum security deposit (in Lamports) to be made by the tenant before the contract begins
	pub security_deposit: u64,

	/// Rent amount per month (in Lamports)
	pub rent_amount: u64,

	/// Duration of the agreement (in months)
	pub duration: u8,

	/// Count of monthly payments due
	pub remaining_payments: u8,

	/// Contract start month (1-12)
	pub start_month: u8,

	/// Contract start year (eg: 2021)
	pub start_year: u16,

	/// Duration (in months) for contract extension requested by Tenant
	pub duration_extension_request: u8
}


/* ==========================================================================
				Account State: Security Deposit Escrow
============================================================================= */

/// The Security Deposit Escrow Account State
/// Used to store the security-deposit amount from the tenant
#[derive(BorshSerialize, BorshDeserialize, Debug)]				// Traits to (de)serialize & debug
pub struct SecurityEscrowAccount {

	/// Agreement status (active, complete, terminated, etc)
	pub status: u8,

	/// Agreement account pubkey
	pub agreement_pubkey: Pubkey,

	/// Property owner account
	pub owner_pubkey: Pubkey,

	/// Tenant account
	pub tenant_pubkey: Pubkey,

	/// Minimum security-deposit amount to be maintained
	pub security_deposit: u64,

	/// Currently remaining security deposit amount in the escrow
	pub remaining_deposit: u64,
}


impl Sealed for RentAgreementAccount {}
impl Sealed for SecurityEscrowAccount {}


/// Is the `Agreement Account` initialized?
impl IsInitialized for RentAgreementAccount {
	fn is_initialized(&self) -> bool {
		self.status != AgreementStatus::Uninitialized as u8
	}
}

impl RentAgreementAccount {

	/// Is initial security_deposit pending by the tenant?
	pub fn is_security_deposit_pending(&self) -> bool {
		self.status == AgreementStatus::DepositPending as u8
	}

	/// Is the rent-agreement complete (i.e, all payments done for the agreed duration)?
	pub fn is_completed(&self) -> bool {
		self.status == AgreementStatus::Completed as u8
	}

	/// Is the rent-agreement terminated?
	pub fn is_terminated(&self) -> bool {
		self.status == AgreementStatus::Terminated as u8
	}

	// Get rent-agreement status as String
	// pub fn get_status(&self) -> String {
	// 	match self.status {
	// 		AgreementStatus::Uninitialized => String::from("Hello, world!")
	// 	}
	// }
}


#[derive(Copy, Clone)]
pub enum AgreementStatus {
	Uninitialized = 0,
	DepositPending,
	Active,
	Completed,
	Terminated,
}
