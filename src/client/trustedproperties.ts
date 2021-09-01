/* eslint-disable @typescript-eslint/no-unsafe-assignment */
/* eslint-disable @typescript-eslint/no-unsafe-member-access */

import {
	Keypair,
	Connection,
	PublicKey,
	LAMPORTS_PER_SOL,
	SystemProgram,
	SYSVAR_RENT_PUBKEY,
	TransactionInstruction,
	Transaction,
	sendAndConfirmTransaction,
} from '@solana/web3.js';
import fs from 'mz/fs';
import path from 'path';
import * as borsh from 'borsh';
import BN from 'bn.js';

import {
	getPayer,
	getRpcUrl,
	newAccountWithLamports,
	createKeypairFromFile,
} from './utils';

/**
 * Connection to the network
 */
let connection: Connection;

/**
 * Keypair associated to the fees' payer
 */
let payer: Keypair;

/**
 * Trusted Properties's program id
 */
let programId: PublicKey;

/**
 * The public key of the Agreement PDA
 */
let agreementPubkey: PublicKey;

/**
 * The public key of the Escrow PDA
 */
let escrowPubkey: PublicKey;

/**
 * The public key of the property owner account
 */
let ownerPubkey: PublicKey;

/**
 * The public key of the tenant account
 */
let tenantPubkey: PublicKey;


/**
 * Path to program files
 */
const PROGRAM_PATH = path.resolve(__dirname, '../../dist/program');

/**
 * Path to program shared object file which should be deployed on chain.
 * This file is created when running `npm run build` (which builds the rRust code)
 */
const PROGRAM_SO_PATH = path.join(PROGRAM_PATH, 'trusted_properties_marketplace_solana_rust.so');

/**
 * Path to the keypair of the deployed program.
 * This file is created when running `npm run deploy`
 */
const PROGRAM_KEYPAIR_PATH = path.join(PROGRAM_PATH, 'trusted_properties_marketplace_solana_rust-keypair.json');

/**
 * The state of an Agreement account managed by the Trusted-Properties program
 */
class AgreementAccount {

	status: number = 0;
	owner_pubkey: PublicKey = new PublicKey(0);
	tenant_pubkey: PublicKey = new PublicKey(0);
	security_escrow_pubkey: PublicKey = new PublicKey(0);
	security_deposit: number = 0;
	rent_amount: number = 0;
	duration: number = 0;
	remaining_payments: number = 0;
	start_month: number = 0;
	start_year: number = 0;
	duration_extension_request: number = 0;

	constructor(fields: {
		status: number,
		owner_pubkey: PublicKey,
		tenant_pubkey: PublicKey,
		security_escrow_pubkey: PublicKey,
		security_deposit: number,
		rent_amount: number,
		duration: number,
		remaining_payments: number,
		start_month: number,
		start_year: number,
		duration_extension_request: number
	} | undefined = undefined) {

		if (fields) {
			this.status = fields.status;
			this.owner_pubkey = fields.owner_pubkey;
			this.tenant_pubkey = fields.tenant_pubkey;
			this.security_escrow_pubkey = fields.security_escrow_pubkey;
			this.security_deposit = fields.security_deposit;
			this.rent_amount = fields.rent_amount;
			this.duration = fields.duration;
			this.remaining_payments = fields.remaining_payments;
			this.start_month = fields.start_month;
			this.start_year = fields.start_year;
			this.duration_extension_request = fields.duration_extension_request;
		}
	}
}

/**
 * Borsh schema definition for Agreement accounts
 */
const AgreementSchema = new Map([
	[AgreementAccount, {kind: 'struct', fields: [
		['pub status', 'u8'],
		['owner_pubkey', 'string'],
		['tenant_pubkey', 'string'],
		['security_escrow_pubkey', 'string'],
		['security_deposit', 'u64'],
		['rent_amount', 'u64'],
		['duration', 'u8'],
		['remaining_payments', 'u8'],
		['start_month', 'u8'],
		['start_year', 'u16'],
		['duration_extension_request', 'u8']
	]}],
]);

/**
 * The expected size of each Agreement account.
 */
const AGREEMENT_SIZE = // 120;
	borsh.serialize(
		AgreementSchema,
		new AgreementAccount(),
	).length;


/**
 * The state of an Security-Escrow account managed by the Trusted-Properties program
 * /
class EscrowAccount {

	status: number = 0;
	agreement_pubkey: PublicKey = new PublicKey(0);
	owner_pubkey: PublicKey = new PublicKey(0);
	tenant_pubkey: PublicKey = new PublicKey(0);
	security_deposit: number = 0;
	remaining_deposit: number = 0;

	constructor(fields: {
		status: number,
		agreement_pubkey: PublicKey,
		owner_pubkey: PublicKey,
		tenant_pubkey: PublicKey,
		security_deposit: number,
		remaining_deposit: number
	} | undefined = undefined) {

		if (fields) {
			this.status = fields.status;
			this.agreement_pubkey = fields.agreement_pubkey;
			this.owner_pubkey = fields.owner_pubkey;
			this.tenant_pubkey = fields.tenant_pubkey;
			this.security_deposit = fields.security_deposit;
			this.remaining_deposit = fields.remaining_deposit;
		}
	}
} */

/**
 * Borsh schema definition for Escrow accounts
 * /
const EscrowSchema = new Map([
	[EscrowAccount, { kind: 'struct', fields: [
		['status', 'u8'],
		['agreement_pubkey', 'string'],
		['owner_pubkey', 'string'],
		['tenant_pubkey', 'string'],
		['security_deposit', 'u64'],
		['remaining_deposit', 'u64']
	] }],
]);
*/

/**
 * The expected size of each Escrow account.
 * /
const ESCROW_SIZE = borsh.serialize(
	EscrowSchema,
	new EscrowAccount(),
).length;
*/



/**
 * Establish a connection to the cluster
 */
export async function establishConnection(): Promise<void> {
	const rpcUrl = await getRpcUrl();
	connection = new Connection(rpcUrl, 'confirmed');
	const version = await connection.getVersion();
	console.log('Connection to cluster established:', rpcUrl, version);
}

/**
 * Establish the "Company" account to pay for everything
 */
export async function establishPayer(): Promise<void> {
	let fees = 0;
	let rentExemptForAgreement = 0;
	let rentExemptForEscrow = 0
	if (!payer) {
		const {feeCalculator} = await connection.getRecentBlockhash();

		// Calculate the cost to fund the greeter account
		rentExemptForAgreement = await connection.getMinimumBalanceForRentExemption(AGREEMENT_SIZE);
		// rentExemptForEscrow = await connection.getMinimumBalanceForRentExemption(ESCROW_SIZE);
		fees += (rentExemptForAgreement /* + rentExemptForEscrow */ );

		// Calculate the cost of sending transactions
		fees += feeCalculator.lamportsPerSignature * 100; // wag

		try {
			// Get payer from cli config
			payer = await getPayer();
		} catch (err) {
			// Fund a new payer via airdrop
			payer = await newAccountWithLamports(connection, fees);
		}
	}

	const lamports = await connection.getBalance(payer.publicKey);
	if (lamports < fees) {
		// This should only happen when using cli config keypair
		const sig = await connection.requestAirdrop(
			payer.publicKey,
			fees - lamports,
		);
		await connection.confirmTransaction(sig);
	}

	console.log(
		'Using the "Company" account',
		payer.publicKey?.toBase58(),
		'containing',
		lamports / LAMPORTS_PER_SOL,
		'SOL to pay for fees',
		{
			rentExemptForAgreement: rentExemptForAgreement,
			rentExemptForEscrow: rentExemptForEscrow
		}
	);
}

/**
 * Check if the hello world BPF program has been deployed
 */
export async function checkProgram(): Promise<void> {
	// Read program id from keypair file
	try {
		const programKeypair = await createKeypairFromFile(PROGRAM_KEYPAIR_PATH);
		programId = programKeypair.publicKey;
	} catch (err) {
		const errMsg = (err as Error).message;
		throw new Error(
		`Failed to read program keypair at '${PROGRAM_KEYPAIR_PATH}' due to error: ${errMsg}. Program may need to be deployed with \`npm run deploy\``,
		);
	}

	// Check if the program has been deployed
	const programInfo = await connection.getAccountInfo(programId);
	if (programInfo === null) {
		if (fs.existsSync(PROGRAM_SO_PATH)) {
			throw new Error(
				'Program needs to be deployed with `npm run deploy`',
			);
		} else {
			throw new Error('Program needs to be built and deployed');
		}
	} else if (!programInfo.executable) {
		throw new Error(`Program is not executable`);
	}
	console.log(`Using program ${programId?.toBase58()}`);

	// TODO: Get from app user
	ownerPubkey = new PublicKey("EJBCNzigNdKMiCueXNaYn3CccMJqmB8DLPZKgfeTWQrh");
	tenantPubkey = new PublicKey("EGikG1URSBTi3Dc2AHwTV4HBDWu1KmbcD63rTbYWv9Cu");


	// Derive the address (public key) of a Agreement account from the program so that it's easy to find later.
	const AGREEMENT_SEED = 'agreement' + ownerPubkey.toString() + tenantPubkey.toString();
	agreementPubkey = await PublicKey.createWithSeed(
		payer.publicKey,
		AGREEMENT_SEED,
		programId,
	);

  	// Check if the Agreement account has already been created
	const agreementAccount = await connection.getAccountInfo(agreementPubkey);
	if (agreementAccount === null) {
		console.log(
			'Creating new Agreement Contract account: ',
			agreementPubkey.toBase58()
		);
		const lamports = await connection.getMinimumBalanceForRentExemption(
			AGREEMENT_SIZE,
		);

		// Create Agreement account
		const transaction = new Transaction().add(
			SystemProgram.createAccountWithSeed({
				fromPubkey: payer.publicKey,
				basePubkey: payer.publicKey,
				seed: AGREEMENT_SEED,
				newAccountPubkey: agreementPubkey,
				lamports,
				space: AGREEMENT_SIZE,
				programId,
			}),
		);
		await sendAndConfirmTransaction(connection, transaction, [payer]);
	}
}


/**
 * Initialize Rent Agreement Contract
 */
export async function initContract(): Promise<void> {

	console.log('Initializing rent agreement contract at address: ', agreementPubkey.toBase58());

	const instruction_index = 0;

	// TODO: TESTING
	const deposit = 1000;
	const rentAmount = 500;
	const duration = 11;
	const start_month = 1;
	const start_year = 2022;

	const instruction: TransactionInstruction = new TransactionInstruction({
		keys: [
			{ pubkey: agreementPubkey, isSigner: false, isWritable: true },
			{ pubkey: agreementPubkey, isSigner: false, isWritable: true },			// TODO: escrowPubkey
			{ pubkey: SYSVAR_RENT_PUBKEY, isSigner: false, isWritable: false },
		],
		programId,
		data:
		// Buffer.alloc(0)
		Buffer.from(Uint8Array.of(instruction_index,
			...Array.from(ownerPubkey.toBytes()),
			...Array.from(tenantPubkey.toBytes()),
			...Array.from(agreementPubkey.toBytes()),			// DEBUG: SHOULD BE: escrowPubkey
			...new BN(deposit).toArray("le", 64),
			...new BN(rentAmount).toArray("le", 64),
			...new BN(duration).toArray("le", 8),
			...new BN(start_month).toArray("le", 8),
			...new BN(start_year).toArray("le", 16),
		))
	});
	await sendAndConfirmTransaction(
		connection,
		new Transaction().add(instruction),
		[payer],
	);
}

/**
 * Report the number of times the greeted account has been said hello to
 * /
export async function reportGreetings(): Promise<void> {
  const accountInfo = await connection.getAccountInfo(agreementPubkey);
  if (accountInfo === null) {
    throw 'Error: cannot find the greeted account';
  }
  const greeting = borsh.deserialize(
    AgreementSchema,
    AgreementAccount,
    accountInfo.data,
  );
  console.log(
    agreementPubkey.toBase58(),
    'has been greeted',
    greeting.counter,
    'time(s)',
  );
}
*/