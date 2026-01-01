// SP1 Guest Program Entry Point
// This code runs inside the zkVM and generates the ZK proof

#![no_main]
sp1_zkvm::entrypoint!(main);

use rorschach_program::generate_complete;

/// Main entry point for the SP1 zkVM
///
/// Reads private key as private input, generates Rorschach pattern,
/// and commits public outputs to the proof.
pub fn main() {
    // Read private key from stdin (private input - not revealed in proof)
    let private_key: [u8; 32] = sp1_zkvm::io::read();

    // Generate Rorschach pattern
    let output = generate_complete(&private_key);

    // Commit public outputs
    // These will be verified on-chain but don't reveal the private key

    // 1. Address (20 bytes) - derived from private key
    sp1_zkvm::io::commit_slice(&output.address);

    // 2. Walks parameter (8 bytes)
    sp1_zkvm::io::commit(&output.walks);

    // 3. Steps parameter (8 bytes)
    sp1_zkvm::io::commit(&output.steps);

    // 4. Image hash (32 bytes) - Keccak256 of binary image
    sp1_zkvm::io::commit_slice(&output.image_hash);

    // 5. Full image data (256 bytes) - for on-chain storage/display
    sp1_zkvm::io::commit_slice(&output.image_data);

    // Total public output: 20 + 8 + 8 + 32 + 256 = 324 bytes
}
