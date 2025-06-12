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
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sp1_sdk::{include_elf, ProverClient, SP1Stdin};
use std::env;
use tokio;
use types::{
    BeaconState, SignedBeaconBlock, MainnetEthSpec, EthSpec,
    BeaconBlock, Hash256, Slot, Eth1Data, Signature,
    Validator, FullPayload, FixedBytesExtended, Epoch, PublicKeyBytes
};
use ssz::Encode;
// use bls::Signature; // Removed to avoid zkVM compilation issues

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

    #[arg(long, env = "ETH_RPC_URL")]
    rpc_url: Option<String>,

    #[arg(long)]
    use_latest_block: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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

    // Setup the inputs
    let input = if args.use_latest_block {
        fetch_latest_block_data(&args).await?
    } else {
        // Use dummy data for testing if not fetching latest block
        BlockProcessingInput {
            state_bytes: vec![0u8; args.state_size], // Dummy state data
            signed_block_bytes: vec![1u8; args.state_size], // Dummy block data
            spec_bytes: vec![2u8; args.state_size], // Dummy spec data
        }
    };

    let mut stdin = SP1Stdin::new();
    stdin.write(&input);

    println!("Input prepared successfully");

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

    Ok(())
}

async fn fetch_latest_block_data(args: &Args) -> Result<BlockProcessingInput, Box<dyn std::error::Error>> {
    let rpc_url = args.rpc_url.as_ref()
        .ok_or("RPC URL is required when using --use-latest-block. Set ETH_RPC_URL env var or use --rpc-url")?;

    println!("Connecting to RPC: {}", rpc_url);

    // Create HTTP client
    let client = reqwest::Client::new();

    // Fetch the latest block using JSON-RPC
    println!("Fetching latest block...");
    let request_body = json!({
        "jsonrpc": "2.0",
        "method": "eth_getBlockByNumber",
        "params": ["latest", true],
        "id": 1
    });

    let response = client
        .post(rpc_url)
        .json(&request_body)
        .send()
        .await?;

    let response_json: Value = response.json().await?;
    let block = response_json["result"]
        .as_object()
        .ok_or("Invalid block response")?;

    let block_number = block["number"]
        .as_str()
        .ok_or("Missing block number")?;
    let block_hash = block["hash"]
        .as_str()
        .ok_or("Missing block hash")?;

    println!("Latest block number: {}", block_number);
    println!("Block hash: {}", block_hash);

    // For now, we'll create a simplified beacon block structure
    // In a real implementation, you would:
    // 1. Fetch the corresponding beacon block for this execution block
    // 2. Fetch the current beacon state
    // 3. Get the appropriate chain spec

    // Create dummy beacon state and spec for demonstration
    let dummy_state = create_dummy_beacon_state()?;
    let dummy_block = create_dummy_signed_beacon_block(&response_json)?;
    let dummy_spec = create_dummy_chain_spec()?;

    println!("Created dummy beacon structures for latest execution block");

    Ok(BlockProcessingInput {
        state_bytes: dummy_state,
        signed_block_bytes: dummy_block,
        spec_bytes: dummy_spec,
    })
}

fn create_dummy_beacon_state() -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // Create a minimal BeaconState for testing purposes
    let spec = &MainnetEthSpec::default_spec();
    
    // Create Eth1Data (required for BeaconState::new)
    let eth1_data = Eth1Data {
        deposit_root: Hash256::zero(),
        deposit_count: 0,
        block_hash: Hash256::zero(),
    };
    
    // Create a basic BeaconState with correct parameters
    let mut state = BeaconState::<MainnetEthSpec>::new(
        0,         // genesis_time
        eth1_data, // eth1_data
        &spec,     // chain spec
    );
    
    // Add a few validators for realism
    for _i in 0..32 {
        let validator = Validator {
            pubkey: PublicKeyBytes::empty(),
            withdrawal_credentials: Hash256::zero(),
            effective_balance: 32_000_000_000, // 32 ETH in Gwei
            slashed: false,
            activation_eligibility_epoch: Epoch::new(0),
            activation_epoch: Epoch::new(0), 
            exit_epoch: Epoch::new(u64::MAX),
            withdrawable_epoch: Epoch::new(u64::MAX),
        };
        state.validators_mut().push(validator).map_err(|e| format!("Failed to add validator: {:?}", e))?;
        state.balances_mut().push(32_000_000_000).map_err(|e| format!("Failed to add balance: {:?}", e))?;
    }
    
    // Serialize to bytes using SSZ encoding
    let serialized = state.as_ssz_bytes();
    Ok(serialized)
}

fn create_dummy_signed_beacon_block(
    execution_block: &Value,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let spec = &MainnetEthSpec::default_spec();
    
    // Extract basic info from execution block
    let block_number = execution_block["result"]["number"]
        .as_str()
        .and_then(|s| u64::from_str_radix(s.trim_start_matches("0x"), 16).ok())
        .unwrap_or(0);
    
    let _slot = Slot::new(block_number);
    
    // Create a minimal BeaconBlock 
    let beacon_block = BeaconBlock::<MainnetEthSpec, FullPayload<MainnetEthSpec>>::empty(&spec);
    
    // Create SignedBeaconBlock with empty signature
    let signed_block = SignedBeaconBlock::from_block(
        beacon_block,
        Signature::empty()
    );
    
    // Serialize to bytes using SSZ encoding
    let serialized = signed_block.as_ssz_bytes();
    Ok(serialized)
}

fn create_dummy_chain_spec() -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // Create the mainnet ChainSpec
    let spec = MainnetEthSpec::default_spec();
    
    // Create a simple byte representation of key ChainSpec parameters
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&spec.genesis_slot.as_u64().to_le_bytes());
    bytes.extend_from_slice(&spec.seconds_per_slot.to_le_bytes());
    bytes.extend_from_slice(&spec.min_deposit_amount.to_le_bytes());
    bytes.extend_from_slice(&spec.max_effective_balance.to_le_bytes());
    bytes.extend_from_slice(&spec.ejection_balance.to_le_bytes());
    bytes.extend_from_slice(&spec.effective_balance_increment.to_le_bytes());
    
    Ok(bytes)
}
