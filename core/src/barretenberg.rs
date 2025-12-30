// Barretenberg integration for Groth16 proof generation
// Requires `bb` CLI to be installed

#[cfg(feature = "std")]
use std::process::Command;
#[cfg(feature = "std")]
use std::fs;

/// Generate Groth16 proof using Barretenberg
///
/// Requires:
/// - Compiled circuit: circuits/target/circuits.json
/// - Witness file: circuits/target/circuits.gz
///
/// Creates:
/// - circuits/proof - Groth16 proof binary
#[cfg(feature = "std")]
pub fn generate_groth16_proof(circuits_dir: &str) -> Result<Vec<u8>, String> {
    println!("Generating Groth16 proof with Barretenberg...");
    println!("  This may take 30-60 seconds...");

    // Check if bb is installed
    let bb_check = Command::new("bb").arg("--version").output();
    if bb_check.is_err() {
        return Err(
            "Barretenberg CLI (bb) not found. Install it with:\n\
             curl -L https://raw.githubusercontent.com/AztecProtocol/aztec-packages/master/barretenberg/cpp/installation/install | bash\n\
             bbup".to_string()
        );
    }

    let circuit_path = format!("{}/target/circuits.json", circuits_dir);
    let witness_path = format!("{}/target/circuits.gz", circuits_dir);
    let proof_path = format!("{}/proof", circuits_dir);

    // Verify input files exist
    if !std::path::Path::new(&circuit_path).exists() {
        return Err(format!("Circuit file not found: {}", circuit_path));
    }
    if !std::path::Path::new(&witness_path).exists() {
        return Err(format!("Witness file not found: {}", witness_path));
    }

    // Generate proof
    let output = Command::new("bb")
        .arg("prove")
        .arg("-b")
        .arg(&circuit_path)
        .arg("-w")
        .arg(&witness_path)
        .arg("-o")
        .arg(&proof_path)
        .output()
        .map_err(|e| format!("Failed to execute bb: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!(
            "bb prove failed:\nstdout: {}\nstderr: {}",
            stdout, stderr
        ));
    }

    // Read the generated proof
    let proof = fs::read(&proof_path)
        .map_err(|e| format!("Failed to read proof file: {}", e))?;

    println!("✓ Groth16 proof generated ({} bytes)", proof.len());

    Ok(proof)
}

/// Generate Solidity verifier contract using Barretenberg
///
/// Creates a Solidity contract that can verify proofs on-chain
#[cfg(feature = "std")]
pub fn generate_verifier_contract(
    circuits_dir: &str,
    output_path: &str,
) -> Result<(), String> {
    println!("Generating Solidity verifier contract...");

    let circuit_path = format!("{}/target/circuits.json", circuits_dir);

    if !std::path::Path::new(&circuit_path).exists() {
        return Err(format!("Circuit file not found: {}", circuit_path));
    }

    let output = Command::new("bb")
        .arg("contract")
        .arg("-b")
        .arg(&circuit_path)
        .arg("-o")
        .arg(output_path)
        .output()
        .map_err(|e| format!("Failed to execute bb: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!(
            "bb contract failed:\nstdout: {}\nstderr: {}",
            stdout, stderr
        ));
    }

    println!("✓ Verifier contract saved to: {}", output_path);
    println!("  Deploy this contract to verify proofs on-chain");

    Ok(())
}

/// Encode public inputs for Solidity verification
///
/// The Noir circuit returns: (u64, u64, [u8; 256])
/// We need to encode as: (uint256, uint256, bytes) for Solidity
///
/// Format:
/// - walks: uint256 (32 bytes, right-aligned)
/// - steps: uint256 (32 bytes, right-aligned)
/// - binary_image: bytes (dynamic length, ABI-encoded)
pub fn encode_public_inputs_simple(
    walks: u64,
    steps: u64,
    binary_image: &[u8; 256],
) -> Vec<u8> {
    let mut encoded = Vec::new();

    // Encode walks as uint256 (32 bytes, big-endian)
    let mut walks_bytes = [0u8; 32];
    walks_bytes[24..32].copy_from_slice(&walks.to_be_bytes());
    encoded.extend_from_slice(&walks_bytes);

    // Encode steps as uint256 (32 bytes, big-endian)
    let mut steps_bytes = [0u8; 32];
    steps_bytes[24..32].copy_from_slice(&steps.to_be_bytes());
    encoded.extend_from_slice(&steps_bytes);

    // For bytes, we need: offset, length, data
    // Since we're at position 0x40 (after walks and steps):

    // Offset to where bytes data starts (after this word) = 32 bytes = 0x20
    encoded.extend_from_slice(&[0u8; 28]);
    encoded.extend_from_slice(&[0, 0, 0, 0x20]); // offset = 32

    // Length of bytes array = 256
    encoded.extend_from_slice(&[0u8; 28]);
    encoded.extend_from_slice(&[0, 0, 0x01, 0x00]); // length = 256

    // Actual bytes data (256 bytes)
    encoded.extend_from_slice(binary_image);

    encoded
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_encode_public_inputs() {
        let walks = 13u64;
        let steps = 92u64;
        let binary_image = [0u8; 256];

        let encoded = encode_public_inputs_simple(walks, steps, &binary_image);

        // Should be: 32 (walks) + 32 (steps) + 32 (offset) + 32 (length) + 256 (data) = 384 bytes
        assert_eq!(encoded.len(), 384);

        // Check walks encoding
        assert_eq!(&encoded[24..32], &[0, 0, 0, 0, 0, 0, 0, 13]);

        // Check steps encoding
        assert_eq!(&encoded[56..64], &[0, 0, 0, 0, 0, 0, 0, 92]);

        // Check length encoding
        assert_eq!(&encoded[92..96], &[0, 0, 0x01, 0x00]); // 256
    }
}
