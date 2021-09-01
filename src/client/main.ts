import {
  establishConnection,
  establishPayer,
  checkProgram,
  initContract,
//   reportGreetings,
} from './trustedproperties';

async function main() {
  console.log("Let's say hello to a Solana account...");

  // Establish connection to the cluster
  await establishConnection();

  // Determine who pays for the fees
  await establishPayer();

  // Check if the program has been deployed
  await checkProgram();

  // Initialize Agreement account
  await initContract();

  // Find out how many times that account has been greeted
//   await reportGreetings();

  console.log('Success');
}

main().then(
  () => process.exit(),
  err => {
    console.error(err);
    process.exit(-1);
  },
);
