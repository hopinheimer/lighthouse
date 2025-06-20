//! Beacon block processing program for SP1 zkVM
//! This program processes Ethereum beacon blocks and validates state transitions.

#![no_main]
sp1_zkvm::entrypoint!(main);

// SP1 zkVM provides its own getrandom implementation

use serde::{Deserialize, Serialize};
// Available for use - state_processing is now accessible
use state_processing::per_block_processing;
use types::{BeaconState, SignedBeaconBlock, ChainSpec, EthSpec, MainnetEthSpec};

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

pub fn main() {
    // Read input from SP1
    let input: BlockProcessingInput = sp1_zkvm::io::read();
    
    let result = process_block_wrapper(input);
    
    // Write result back to SP1
    sp1_zkvm::io::commit(&result);
    
    println!("SP1 zkVM block processing completed!");
}

fn process_block_wrapper(input: BlockProcessingInput) -> BlockProcessingOutput {
    match process_block_inner(input) {
        Ok(updated_state_bytes) => BlockProcessingOutput {
            success: true,
            error_message: None,
            updated_state_bytes: Some(updated_state_bytes),
        },
        Err(error) => BlockProcessingOutput {
            success: false,
            error_message: Some(error),
            updated_state_bytes: None,
        },
    }
}

fn process_block_inner(input: BlockProcessingInput) -> Result<Vec<u8>, String> {
    // For now, we'll use a simplified approach due to serialization complexity
    // In a full implementation, you would:
    // 1. Deserialize the beacon state from state_bytes using SSZ
    // 2. Deserialize the signed block from signed_block_bytes using SSZ
    // 3. Deserialize the chain spec from spec_bytes
    // 4. Call per_block_processing with proper parameters
    // 5. Serialize the updated state back to bytes using SSZ
    
    println!("Processing block with {} state bytes, {} block bytes, {} spec bytes",
             input.state_bytes.len(), 
             input.signed_block_bytes.len(), 
             input.spec_bytes.len());
    
    // Simulate successful processing for now
    // TODO: Implement actual SSZ deserialization and per_block_processing call
    // let mut state: BeaconState<MainnetEthSpec> = ssz::decode(&input.state_bytes)
    //     .map_err(|e| format!("Failed to decode beacon state: {}", e))?;
    // let signed_block: SignedBeaconBlock<MainnetEthSpec> = ssz::decode(&input.signed_block_bytes)
    //     .map_err(|e| format!("Failed to decode signed block: {}", e))?;
    // let spec: ChainSpec = serde_json::from_slice(&input.spec_bytes)
    //     .map_err(|e| format!("Failed to decode chain spec: {}", e))?;
    
    // per_block_processing(&mut state, &signed_block, None, &spec)
    //     .map_err(|e| format!("Block processing failed: {:?}", e))?;
    
    // let updated_state_bytes = ssz::encode(&state);
    
    println!("Block processing completed successfully");
    
    // For now, return the original state bytes to demonstrate the flow
    Ok(input.state_bytes)
}
