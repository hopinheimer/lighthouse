//! An end-to-end example of using the SP1 SDK to generate a proof of beacon block processing
//! that can be executed or have a core proof generated.
//!
//! You can run this script using the following command:
//! ```shell
//! RUST_LOG=info cargo run --release -- --execute
//! ```
//! or
//! ```shell
//! RUST_LOG=info cargo run --release -- --prove
//! ```

use clap::Parser;
use serde::{Deserialize, Serialize};
use sp1_sdk::{include_elf, ProverClient, SP1Stdin};

/// The ELF (executable and linkable format) file for the Succinct RISC-V zkVM.
pub const BLOCK_PROCESSING_ELF: &[u8] = include_elf!("block-processing-program");

#[derive(Serialize, Deserialize)]
pub struct BlockProcessingInput {
    pub state_bytes: Vec<u8>,
    pub signed_block_bytes: Vec<u8>,
    pub spec_bytes: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
pub struct BlockProcessingOutput {
    pub success: bool,
    pub error_message: Option<String>,
    pub updated_state_bytes: Option<Vec<u8>>,
}

/// The arguments for the command.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long)]
    execute: bool,

    #[arg(long)]
    prove: bool,

    #[arg(long, default_value = "100")]
    state_size: usize,
}

fn main() {
    // Setup the logger.
    sp1_sdk::utils::setup_logger();
    dotenv::dotenv().ok();

    // Parse the command line arguments.
    let args = Args::parse();

    if args.execute == args.prove {
        eprintln!("Error: You must specify either --execute or --prove");
        std::process::exit(1);
    }

    // Setup the prover client.
    let client = ProverClient::from_env();

    // Setup the inputs with dummy data for testing.
    let input = BlockProcessingInput {
        state_bytes: vec![0u8; args.state_size], // Dummy state data
        signed_block_bytes: vec![1u8; args.state_size], // Dummy block data
        spec_bytes: vec![2u8; args.state_size], // Dummy spec data
    };

    let mut stdin = SP1Stdin::new();
    stdin.write(&input);

    println!("State size: {} bytes", args.state_size);

    if args.execute {
        // Execute the program
        let (mut output, report) = client.execute(BLOCK_PROCESSING_ELF, &stdin).run().unwrap();
        println!("Program executed successfully.");

        // Read the output.
        let result: BlockProcessingOutput = output.read();
        println!("Success: {}", result.success);
        if let Some(error) = &result.error_message {
            println!("Error: {}", error);
        }
        if let Some(state_bytes) = &result.updated_state_bytes {
            println!("Updated state size: {} bytes", state_bytes.len());
        }

        println!("Block processing completed successfully!");

        // Record the number of cycles executed.
        println!("Number of cycles: {}", report.total_instruction_count());
    } else {
        // Setup the program for proving.
        let (pk, vk) = client.setup(BLOCK_PROCESSING_ELF);

        // Generate the proof
        let proof = client
            .prove(&pk, &stdin)
            .run()
            .expect("failed to generate proof");

        println!("Successfully generated proof!");

        // Verify the proof.
        client.verify(&proof, &vk).expect("failed to verify proof");
        println!("Successfully verified proof!");
    }
}
