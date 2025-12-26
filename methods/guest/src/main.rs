#![no_main]
#![no_std]

extern crate alloc;

use risc0_zkvm::guest::env;
use ror_core::{derive_parameters, generate_rorschach_half, Pixel};
use k256::ecdsa::{SigningKey, VerifyingKey};
use sha3::{Digest, Keccak256};

risc0_zkvm::guest::entry!(main);

fn main() {
    // Read private key from host (kept private)
    let private_key: [u8; 32] = env::read();

    // Derive public key using secp256k1
    let signing_key = SigningKey::from_bytes(&private_key.into())
        .expect("Invalid private key");
    let verifying_key = VerifyingKey::from(&signing_key);
    let public_key_bytes = verifying_key.to_encoded_point(false);
    let public_key_uncompressed = public_key_bytes.as_bytes();

    // Derive Ethereum address (last 20 bytes of keccak256(public_key))
    let mut hasher = Keccak256::new();
    hasher.update(&public_key_uncompressed[1..]); // Skip the 0x04 prefix
    let hash = hasher.finalize();
    let address: [u8; 20] = hash[12..32].try_into().unwrap();

    // Derive generation parameters from private key
    let (walks, steps) = derive_parameters(&private_key);

    // Define colors (hardcoded for now, could be public inputs)
    let foreground = Pixel::new(255, 217, 102);
    let background = Pixel::new(255, 0, 129);

    // Generate the Rorschach half-canvas deterministically
    let image = generate_rorschach_half(&private_key, walks, steps, foreground, background);

    // Commit public outputs to the journal
    env::commit(&address);
    env::commit(&walks);
    env::commit(&steps);
    env::commit(&image.to_bytes());
}
