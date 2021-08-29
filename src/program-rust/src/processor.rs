/// processor.rs -> program logic

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
	account_info::{next_account_info, AccountInfo},
	entrypoint::ProgramResult,
	msg,
	program::invoke,
	program_error::ProgramError,
	program_pack::IsInitialized,
	pubkey::Pubkey,
	system_instruction,
	sysvar::{rent::Rent, Sysvar},
};

use crate::{
	error::TrustedPropertiesError,
	instruction::TrustedPropertiesInstruction,
	state::{AgreementStatus, RentAgreementAccount},
};


pub struct Processor;

impl Processor {

	/// The entrypoint function to process the instructions.
	///
	/// @param program_id The public key of the account this program was loaded into.
	/// @param accounts Array of all accounts passed to the program.
	/// @param instruction_data The additional transaction parameters passed to the program as byte array.
	pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], instruction_data: &[u8]) -> ProgramResult {

		msg!("[TrustedProperties] Rust Program Entrypoint.");

		let instruction = TrustedPropertiesInstruction::unpack(instruction_data)?;
		match instruction {
			// Initialize the rent-contract
			TrustedPropertiesInstruction::InitializeRentContract {
				owner_pubkey,
				tenant_pubkey,
				security_escrow_pubkey,
				security_deposit,
				rent_amount,
				duration,
				start_month,
				start_year,
			} => Self::initialize_rent_contract(accounts, program_id, owner_pubkey, tenant_pubkey, security_escrow_pubkey, security_deposit, rent_amount, duration, start_month, start_year),

			// Pay first-time security_deposit amount (from tenant to escrow) & confirm the agreement
			TrustedPropertiesInstruction::DepositSecurity { security_deposit_amount } => Self::deposit_security(accounts, program_id, security_deposit_amount),

			// Pay rent from (tenant to owner)
			TrustedPropertiesInstruction::PayRent { rent_amount } => Self::pay_rent(accounts, program_id, rent_amount),

			// Terminate the contract early
			TrustedPropertiesInstruction::TerminateEarly {} => Self::terminate_early(accounts, program_id),

			// Request to extend the contract duration (by Tenant)
			TrustedPropertiesInstruction::RequestContractDurationExtension { extension_duration } => Self::extend_contract_duration_request(accounts, program_id, extension_duration),

			// Confirm to extend the contract duration (by Owner)
			TrustedPropertiesInstruction::ConfirmContractDurationExtension { extension_duration } => Self::extend_contract_duration_confirm(accounts, program_id, extension_duration),
		}
	}


	/// Initialize the Contract Account for the rent agreement
	fn initialize_rent_contract(
		accounts: &[AccountInfo],
		program_id: &Pubkey,
		owner_pubkey: Pubkey,
		tenant_pubkey: Pubkey,
		security_escrow_pubkey: Pubkey,
		security_deposit: u64,
		rent_amount: u64,
		duration: u8,
		start_month: u8,
		start_year: u16,
	) -> ProgramResult {

		let accounts_iter = &mut accounts.iter();

		let rent_agreement_account = next_account_info(accounts_iter)?;
		if rent_agreement_account.owner != program_id {
			msg!("[TrustedProperties] ERROR: Rent Agreement account must be owned by this program");
			return Err(ProgramError::IncorrectProgramId);
		}

		let solana_rent = &Rent::from_account_info(next_account_info(accounts_iter)?)?;
		// Make sure this account is rent exempt
		// Program owners can maintain a minimum amount of Lamports to keep the program rent-free.
		if !solana_rent.is_exempt(
			rent_agreement_account.lamports(),
			rent_agreement_account.data_len(),
		) {
			msg!("[TrustedProperties] ERROR: Rent Agreement account not rent exempt. Balance: {}", rent_agreement_account.lamports());
			return Err(ProgramError::AccountNotRentExempt);
		}

		// Initialize the Rent Agreement Account with the initial data
		// Note: the structure of the data state must match the `space` reserved when account created
		let rent_agreement_data = RentAgreementAccount::try_from_slice(&rent_agreement_account.data.borrow());

		if rent_agreement_data.is_err() {
			msg!("[TrustedProperties] ERROR: Rent Agreement account data size is incorrect: {}", rent_agreement_account.try_data_len()?);
			return Err(ProgramError::InvalidAccountData);
		}

		let mut rent_data = rent_agreement_data.unwrap();
		if rent_data.is_initialized() {
			msg!("[TrustedProperties] ERROR: Rent Agreement account already initialized");
			return Err(ProgramError::AccountAlreadyInitialized);
		}

		rent_data.status = AgreementStatus::DepositPending as u8;
		rent_data.owner_pubkey = owner_pubkey;
		rent_data.tenant_pubkey = tenant_pubkey;
		rent_data.security_escrow_pubkey = security_escrow_pubkey;
		rent_data.security_deposit = security_deposit;
		rent_data.rent_amount = rent_amount;
		rent_data.duration = duration;
		rent_data.remaining_payments = duration;
		rent_data.start_month = start_month;
		rent_data.start_year = start_year;
		rent_data.duration_extension_request = 0;
		rent_data.serialize(&mut &mut rent_agreement_account.data.borrow_mut()[..])?;

		msg!("[TrustedProperties] Rent Agreement account initialized successfully: {:?}", rent_data);

		Ok(())
	}


	/// Pay the rent (tenant -> owner)
	fn pay_rent(accounts: &[AccountInfo], program_id: &Pubkey, rent_amount: u64) -> ProgramResult {

		let accounts_iter = &mut accounts.iter();

		let rent_agreement_account = next_account_info(accounts_iter)?;
		if rent_agreement_account.owner != program_id {
			msg!("[TrustedProperties] Rent agreement account is not owned by this program");
			return Err(ProgramError::IncorrectProgramId);
		}

		let tenant_account = next_account_info(accounts_iter)?;
		let owner_account: &AccountInfo = next_account_info(accounts_iter)?;
		let system_program_account = next_account_info(accounts_iter)?;

		if !tenant_account.is_signer {
			return Err(ProgramError::MissingRequiredSignature);
		}

		if tenant_account.lamports() < rent_amount {
			return Err(ProgramError::InsufficientFunds);
		}

		// Transfer to self? Do nothing.
		if tenant_account.key == owner_account.key {
			return Ok(());
		}

		// Initialize the Rent Agreement Account with the initial data
		// Note: the structure of the data state must match the `space` the client used to create the account
		let rent_agreement_data = RentAgreementAccount::try_from_slice(&rent_agreement_account.data.borrow());

		if rent_agreement_data.is_err() {
			msg!("[TrustedProperties] Rent agreement account data size incorrect: {}", rent_agreement_account.try_data_len()?);
			return Err(ProgramError::InvalidAccountData);
		}

		let mut rent_data = rent_agreement_data.unwrap();
		if !rent_data.is_initialized() {
			msg!("[TrustedProperties] ERROR: Invalid agreement: Rent agreement account not initialized.");
			return Err(ProgramError::UninitializedAccount);
		}

		// Make sure we pay the same account used during the agreement initialization
		if rent_data.owner_pubkey != *owner_account.key {
			msg!("[TrustedProperties] ERROR: Owner's public-key (owner_pubkey) does not match the one used during agreement initialization");
			return Err(ProgramError::InvalidAccountData);
		}

		msg!("[TrustedProperties] Transferring {} lamports from tenant (current balance: {})", rent_amount, tenant_account.lamports());

		if rent_data.is_completed() {
			msg!("[TrustedProperties] ERROR: Rent already paid in full");
			return Err(TrustedPropertiesError::RentAlreadyFullyPaid.into());
		}

		if rent_data.is_terminated() {
			msg!("[TrustedProperties] ERROR: Rent agreement already terminated");
			return Err(TrustedPropertiesError::RentAgreementTerminated.into());
		}

		// TODO: Allow advance payment (transfer amount more than the monthly rent amount). This can go into the escrow account as advance deposit.
		if rent_data.rent_amount != rent_amount {
			msg!("[TrustedProperties] ERROR: Rent amount ({}) does not match the agreement amount ({})", rent_amount, rent_data.rent_amount);
			return Err(TrustedPropertiesError::IncorrectPaymentAmount.into());
		}

		// Create instruction to transfer the rent-amount (lamports) from tenant's account to the owner's account
		let instruction = system_instruction::transfer(&tenant_account.key, &owner_account.key, rent_amount);

		// Invoke the system program to transfer the rent-amount
		invoke(
			&instruction,
			&[
				system_program_account.clone(),
				owner_account.clone(),
				tenant_account.clone(),
			],
		)?;

		msg!("[TrustedProperties] Transfer completed. Remaining balance of the tenant: {}", tenant_account.lamports());

		// Decrement the number of payment
		rent_data.remaining_payments -= 1;
		if rent_data.remaining_payments == 0 {
			rent_data.status = AgreementStatus::Completed as u8;
		}
		rent_data.serialize(&mut &mut rent_agreement_account.data.borrow_mut()[..])?;

		Ok(())
	}


	/// Pay the initial security_deposit amount (tenant -> escrow)
	/// TODO: Revert the security_deposit to the tenant after agreement period
	/// TODO: 	or, make the last n payments from security_deposit escrow account.
	/// TODO:	Also, use this for penalty if the tenant terminates the agreement early.
	/// TODO: Merge with pay_rent function to create a common generic function.
	fn deposit_security(accounts: &[AccountInfo], program_id: &Pubkey, security_deposit_amount: u64) -> ProgramResult {

		let accounts_iter = &mut accounts.iter();

		let rent_agreement_account = next_account_info(accounts_iter)?;
		if rent_agreement_account.owner != program_id {
			msg!("[TrustedProperties] Rent agreement account is not owned by this program");
			return Err(ProgramError::IncorrectProgramId);
		}

		let tenant_account = next_account_info(accounts_iter)?;
		let escrow_account: &AccountInfo = next_account_info(accounts_iter)?;
		let system_program_account = next_account_info(accounts_iter)?;

		if !tenant_account.is_signer {
			return Err(ProgramError::MissingRequiredSignature);
		}

		if tenant_account.lamports() < security_deposit_amount {
			return Err(ProgramError::InsufficientFunds);
		}

		// Transfer to self? Do nothing.
		if tenant_account.key == escrow_account.key {
			return Ok(());
		}

		// Initialize the Rent Agreement Account with the initial data
		// Note: the structure of the data state must match the `space` the client used to create the account
		let rent_agreement_data = RentAgreementAccount::try_from_slice(&rent_agreement_account.data.borrow());

		if rent_agreement_data.is_err() {
			msg!("[TrustedProperties] Rent agreement account data size incorrect: {}", rent_agreement_account.try_data_len()?);
			return Err(ProgramError::InvalidAccountData);
		}

		let mut rent_data = rent_agreement_data.unwrap();
		if !rent_data.is_initialized() {
			msg!("[TrustedProperties] ERROR: Invalid agreement: Rent agreement account not initialized.");
			return Err(ProgramError::UninitializedAccount);
		}

		// Make sure we pay the same account used during the agreement initialization
		if rent_data.security_escrow_pubkey != *escrow_account.key {
			msg!("[TrustedProperties] ERROR: Escrow account's public-key does not match the one used during agreement initialization");
			return Err(ProgramError::InvalidAccountData);
		}

		msg!("[TrustedProperties] Transferring {} lamports from tenant (current balance: {}) to escrow", security_deposit_amount, tenant_account.lamports());

		if !rent_data.is_security_deposit_pending() {
			msg!("[TrustedProperties] ERROR: Security already deposited");
			return Err(TrustedPropertiesError::SecurityAlreadyDeposited.into());
		}

		// TODO: Allow advance payment (transfer amount more than the monthly rent amount)
		if security_deposit_amount != rent_data.security_deposit {
			msg!("[TrustedProperties] ERROR: Deposit amount ({}) does not match the agreed amount ({})", security_deposit_amount, rent_data.security_deposit);
			return Err(TrustedPropertiesError::IncorrectPaymentAmount.into());
		}

		// Create instruction to transfer the rent-amount (lamports) from tenant's account to the owner's account
		let instruction = system_instruction::transfer(&tenant_account.key, &escrow_account.key, security_deposit_amount);

		// Invoke the system program to transfer the security deposit amount to the escrow account
		invoke(
			&instruction,
			&[
				system_program_account.clone(),
				escrow_account.clone(),
				tenant_account.clone(),
			],
		)?;

		msg!("[TrustedProperties] Security deposit completed. Remaining balance of the tenant: {}", tenant_account.lamports());

		// Deposit payment done. Therefore, mark the agreement account as active.
		rent_data.status = AgreementStatus::Active as u8;
		rent_data.serialize(&mut &mut rent_agreement_account.data.borrow_mut()[..])?;

		Ok(())
	}


	/// Terminate the contract early
	fn terminate_early(accounts: &[AccountInfo], program_id: &Pubkey) -> ProgramResult {
		let accounts_iter = &mut accounts.iter();

		let rent_agreement_account = next_account_info(accounts_iter)?;
		if rent_agreement_account.owner != program_id {
			msg!("[TrustedProperties] Rent agreement account is not owned by this program");
			return Err(ProgramError::IncorrectProgramId);
		}

		let rent_agreement_data = RentAgreementAccount::try_from_slice(&rent_agreement_account.data.borrow());
		if rent_agreement_data.is_err() {
			msg!("[TrustedProperties] ERROR: Incorrect data size ({}) for the Rent agreement account", rent_agreement_account.try_data_len()?);
			return Err(ProgramError::InvalidAccountData);
		}

		let mut rent_data = rent_agreement_data.unwrap();
		if !rent_data.is_initialized() {
			msg!("[TrustedProperties] ERROR: Rent agreement account is not initialized");
			return Err(ProgramError::UninitializedAccount);
		}

		if rent_data.is_completed() {
			msg!("[TrustedProperties] ERROR: Full rent already paid");
			return Err(TrustedPropertiesError::RentAlreadyFullyPaid.into());
		}

		if rent_data.is_terminated() {
			msg!("[TrustedProperties] ERROR: Rent agreement already terminated");
			return Err(TrustedPropertiesError::RentAgreementTerminated.into());
		}

		rent_data.remaining_payments = 0;
		rent_data.status = AgreementStatus::Terminated as u8;
		rent_data.serialize(&mut &mut rent_agreement_account.data.borrow_mut()[..])?;

		Ok(())
	}


	/// Extend the contract duration.
	fn extend_contract_duration_request(accounts: &[AccountInfo], program_id: &Pubkey, extension_duration: u8) -> ProgramResult {

		let accounts_iter = &mut accounts.iter();

		let rent_agreement_account = next_account_info(accounts_iter)?;
		if rent_agreement_account.owner != program_id {
			msg!("[TrustedProperties] Rent agreement account is not owned by this program");
			return Err(ProgramError::IncorrectProgramId);
		}

		let tenant_account = next_account_info(accounts_iter)?;

		if !tenant_account.is_signer {
			msg!("[TrustedProperties] Tenant must sign the Duration Extension Request");
			return Err(ProgramError::MissingRequiredSignature);
		}

		let rent_agreement_data = RentAgreementAccount::try_from_slice(&rent_agreement_account.data.borrow());
		if rent_agreement_data.is_err() {
			msg!("[TrustedProperties] Rent agreement account data size incorrect: {}", rent_agreement_account.try_data_len()?);
			return Err(ProgramError::InvalidAccountData);
		}

		let mut rent_data = rent_agreement_data.unwrap();
		if !rent_data.is_initialized() {
			msg!("[TrustedProperties] ERROR: Invalid agreement: Rent agreement account not initialized.");
			return Err(ProgramError::UninitializedAccount);
		}

		if rent_data.status != AgreementStatus::Active as u8 {
			msg!("[TrustedProperties] ERROR: Agreement must be active to extend the duration");
			return Err(TrustedPropertiesError::InvalidAgreementStatus.into());
		}

		// Update the Agreement Duration Extension request
		rent_data.duration_extension_request = extension_duration;
		rent_data.serialize(&mut &mut rent_agreement_account.data.borrow_mut()[..])?;

		Ok(())
	}


	/// Confirm the extension of contract duration (by Owner).
	fn extend_contract_duration_confirm(accounts: &[AccountInfo], program_id: &Pubkey, extension_duration: u8) -> ProgramResult {

		let accounts_iter = &mut accounts.iter();

		let rent_agreement_account = next_account_info(accounts_iter)?;
		if rent_agreement_account.owner != program_id {
			msg!("[TrustedProperties] Rent agreement account is not owned by this program");
			return Err(ProgramError::IncorrectProgramId);
		}

		let owner_account = next_account_info(accounts_iter)?;

		if !owner_account.is_signer {
			msg!("[TrustedProperties] Owner must sign the Duration Extension Confirmation");
			return Err(ProgramError::MissingRequiredSignature);
		}

		let rent_agreement_data = RentAgreementAccount::try_from_slice(&rent_agreement_account.data.borrow());
		if rent_agreement_data.is_err() {
			msg!("[TrustedProperties] Rent agreement account data size incorrect: {}", rent_agreement_account.try_data_len()?);
			return Err(ProgramError::InvalidAccountData);
		}

		let mut rent_data = rent_agreement_data.unwrap();
		if !rent_data.is_initialized() {
			msg!("[TrustedProperties] ERROR: Invalid agreement: Rent agreement account not initialized.");
			return Err(ProgramError::UninitializedAccount);
		}

		if rent_data.status != AgreementStatus::Active as u8 {
			msg!("[TrustedProperties] ERROR: Agreement must be active to extend the duration");
			return Err(TrustedPropertiesError::InvalidAgreementStatus.into());
		}

		if rent_data.duration_extension_request != extension_duration {
			msg!("[TrustedProperties] ERROR: Extension duration ({}) does not match the requested one ({}).", extension_duration, rent_data.duration_extension_request);
			return Err(TrustedPropertiesError::InvalidInstructionParameter.into());
		}

		// Update the Agreement Duration Extension
		rent_data.duration += extension_duration;
		rent_data.remaining_payments += extension_duration;
		rent_data.duration_extension_request = 0;
		rent_data.serialize(&mut &mut rent_agreement_account.data.borrow_mut()[..])?;

		Ok(())
	}

}
