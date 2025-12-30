# On-Chain Verification Guide

## Overview

To verify Noir proofs on-chain, you need:
1. **Barretenberg** (`bb`) - Generates Groth16 proofs from Noir witnesses
2. **Solidity Verifier Contract** - Verifies proofs on Ethereum
3. **Public Inputs** - (walks, steps, binary_image) encoded for Solidity

## Prerequisites

### Install Barretenberg

```bash
# Option 1: Using bbup (recommended)
curl -L https://raw.githubusercontent.com/AztecProtocol/aztec-packages/master/barretenberg/cpp/installation/install | bash
bbup

# Option 2: From source
git clone https://github.com/AztecProtocol/aztec-packages.git
cd aztec-packages/barretenberg/cpp
cmake --preset default
cmake --build --preset default
```

Verify installation:
```bash
bb --version
```

## Workflow

### 1. Generate Witness (Already Working!)

```bash
cargo run --bin ror -- \
  --private-key 0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef \
  --prove \
  --output image.png
```

This creates:
- `circuits/target/circuits.json` - Compiled circuit
- `circuits/target/circuits.gz` - Witness file
- `image.witness.gz` - Copy of witness

### 2. Generate Groth16 Proof

```bash
cd circuits

# Generate proof using Barretenberg
bb prove \
  -b ./target/circuits.json \
  -w ./target/circuits.gz \
  -o ./proof

# This creates:
# - proof - The Groth16 proof (binary format)
```

### 3. Generate Solidity Verifier Contract

```bash
# Generate verifier contract
bb contract \
  -b ./target/circuits.json \
  -o ../contracts/Verifier.sol
```

This creates a Solidity contract that can verify your proofs on-chain.

### 4. Extract Public Inputs

The public inputs from your circuit are:
- `walks` (uint64)
- `steps` (uint64)
- `binary_image` ([u8; 256])

We need to encode these for Solidity verification.

## Implementation

### Step 1: Add Barretenberg Integration to Rust

Let me create a module to handle Barretenberg proof generation:

```rust
// File: core/src/barretenberg.rs

use std::process::Command;
use std::path::Path;
use std::fs;

/// Generate Groth16 proof using Barretenberg
pub fn generate_groth16_proof(
    circuits_dir: &str,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    println!("Generating Groth16 proof with Barretenberg...");

    let output = Command::new("bb")
        .arg("prove")
        .arg("-b")
        .arg(format!("{}/target/circuits.json", circuits_dir))
        .arg("-w")
        .arg(format!("{}/target/circuits.gz", circuits_dir))
        .arg("-o")
        .arg(format!("{}/proof", circuits_dir))
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("bb prove failed: {}", stderr).into());
    }

    // Read the generated proof
    let proof_path = format!("{}/proof", circuits_dir);
    let proof = fs::read(&proof_path)?;

    println!("✓ Groth16 proof generated ({} bytes)", proof.len());

    Ok(proof)
}

/// Generate Solidity verifier contract
pub fn generate_verifier_contract(
    circuits_dir: &str,
    output_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Generating Solidity verifier contract...");

    let output = Command::new("bb")
        .arg("contract")
        .arg("-b")
        .arg(format!("{}/target/circuits.json", circuits_dir))
        .arg("-o")
        .arg(output_path)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("bb contract failed: {}", stderr).into());
    }

    println!("✓ Verifier contract saved to: {}", output_path);

    Ok(())
}

/// Encode public inputs for Solidity verification
/// Returns ABI-encoded bytes ready for contract call
pub fn encode_public_inputs(
    walks: u64,
    steps: u64,
    binary_image: &[u8; 256],
) -> Vec<u8> {
    use alloy_sol_types::SolValue;

    // Define the public inputs struct matching Noir circuit outputs
    // In Solidity: function verify(uint64 walks, uint64 steps, bytes memory binaryImage)

    let mut encoded = Vec::new();

    // Encode walks as uint256 (Solidity doesn't have uint64)
    let walks_u256 = alloy_primitives::U256::from(walks);
    encoded.extend_from_slice(&walks_u256.abi_encode());

    // Encode steps as uint256
    let steps_u256 = alloy_primitives::U256::from(steps);
    encoded.extend_from_slice(&steps_u256.abi_encode());

    // Encode binary_image as bytes
    let image_bytes = alloy_primitives::Bytes::from(binary_image.to_vec());
    encoded.extend_from_slice(&image_bytes.abi_encode());

    encoded
}
```

### Step 2: Create Custom Verifier Contract Wrapper

```solidity
// File: contracts/RorschachVerifier.sol

// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

// Import the auto-generated verifier
import "./Verifier.sol";

contract RorschachVerifier {
    UltraVerifier public verifier;

    struct ProofData {
        uint64 walks;
        uint64 steps;
        bytes binaryImage;
    }

    event ProofVerified(
        address indexed prover,
        uint64 walks,
        uint64 steps,
        bytes32 imageHash
    );

    constructor(address _verifier) {
        verifier = UltraVerifier(_verifier);
    }

    /// Verify a Rorschach proof on-chain
    /// @param proof The Groth16 proof bytes
    /// @param publicInputs The public inputs (walks, steps, binary_image)
    function verifyRorschach(
        bytes calldata proof,
        bytes calldata publicInputs
    ) external returns (bool) {
        // Verify the proof
        bool isValid = verifier.verify(proof, publicInputs);

        require(isValid, "Invalid proof");

        // Decode public inputs for event emission
        (uint64 walks, uint64 steps, bytes memory binaryImage) =
            abi.decode(publicInputs, (uint64, uint64, bytes));

        emit ProofVerified(
            msg.sender,
            walks,
            steps,
            keccak256(binaryImage)
        );

        return true;
    }

    /// Store verified image on-chain (optional, costs gas)
    mapping(bytes32 => ProofData) public verifiedImages;

    function verifyAndStore(
        bytes calldata proof,
        bytes calldata publicInputs
    ) external {
        bool isValid = verifyRorschach(proof, publicInputs);
        require(isValid, "Invalid proof");

        (uint64 walks, uint64 steps, bytes memory binaryImage) =
            abi.decode(publicInputs, (uint64, uint64, bytes));

        bytes32 imageHash = keccak256(binaryImage);

        verifiedImages[imageHash] = ProofData({
            walks: walks,
            steps: steps,
            binaryImage: binaryImage
        });
    }
}
```

### Step 3: Update CLI to Support Groth16

```rust
// Add to host/src/main.rs

use ror_core::barretenberg::{
    generate_groth16_proof,
    generate_verifier_contract,
    encode_public_inputs,
};

// Update the prove_groth16 handler:
if cli.prove_groth16 {
    // First, generate witness via Noir
    let (walks, steps, binary_image) = generate_noir_proof(&private_key)?;

    // Generate Groth16 proof via Barretenberg
    let proof = generate_groth16_proof("circuits")?;

    // Encode public inputs for Solidity
    let public_inputs = encode_public_inputs(walks, steps, &binary_image.data);

    // Save proof
    let proof_path = cli.output.with_extension("groth16.proof");
    fs::write(&proof_path, &proof)?;
    println!("  Groth16 proof saved to: {} ({} bytes)",
        proof_path.display(), proof.len());

    // Save public inputs
    let inputs_path = cli.output.with_extension("public_inputs");
    fs::write(&inputs_path, &public_inputs)?;
    println!("  Public inputs saved to: {}", inputs_path.display());

    // Optionally generate verifier contract
    if cli.generate_verifier {
        let verifier_path = "contracts/Verifier.sol";
        generate_verifier_contract("circuits", verifier_path)?;
    }

    // Generate image as usual...
    // ...

    Ok(())
}
```

## Usage

### Generate On-Chain Proof

```bash
# 1. Generate witness and Groth16 proof
cargo run --bin ror -- \
  --private-key 0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef \
  --prove-groth16 \
  --generate-verifier \
  --output image.png

# Creates:
# - image.png - Visual output
# - image.groth16.proof - Proof for on-chain verification
# - image.public_inputs - Public inputs (ABI-encoded)
# - contracts/Verifier.sol - Solidity verifier contract
```

### Deploy and Verify

```bash
# 2. Deploy verifier contract (using Foundry)
cd contracts
forge create Verifier --rpc-url $RPC_URL --private-key $DEPLOYER_KEY

# 3. Deploy wrapper contract
forge create RorschachVerifier \
  --constructor-args <VERIFIER_ADDRESS> \
  --rpc-url $RPC_URL \
  --private-key $DEPLOYER_KEY

# 4. Verify proof on-chain
cast send $RORSCHACH_VERIFIER \
  "verifyRorschach(bytes,bytes)" \
  $(cat image.groth16.proof | xxd -p -c 0) \
  $(cat image.public_inputs | xxd -p -c 0) \
  --rpc-url $RPC_URL \
  --private-key $PROVER_KEY
```

### Using Hardhat

```javascript
// scripts/verify.js
const { ethers } = require("hardhat");
const fs = require("fs");

async function main() {
  // Load proof and public inputs
  const proof = fs.readFileSync("image.groth16.proof");
  const publicInputs = fs.readFileSync("image.public_inputs");

  // Get contract
  const verifier = await ethers.getContractAt(
    "RorschachVerifier",
    process.env.VERIFIER_ADDRESS
  );

  // Verify on-chain
  const tx = await verifier.verifyRorschach(proof, publicInputs);
  await tx.wait();

  console.log("✓ Proof verified on-chain!");
}

main().catch(console.error);
```

## Public Inputs Format

Your Noir circuit returns:
```noir
(u64, u64, [u8; 256])
// (walks, steps, binary_image)
```

These get encoded as:
```solidity
(uint256, uint256, bytes)
```

### Example Public Inputs

For the test key that produces `walks=13, steps=92`:

```
Offset  Data
------  ----
0x00    0x000000000000000000000000000000000000000000000000000000000000000d  // walks = 13
0x20    0x000000000000000000000000000000000000000000000000000000000000005c  // steps = 92
0x40    0x0000000000000000000000000000000000000000000000000000000000000060  // offset to bytes
0x60    0x0000000000000000000000000000000000000000000000000000000000000100  // length = 256
0x80    0x0000000000000000000000000000000000000000000000000000000000000000  // binary_image[0..32]
...     ... (256 bytes of image data) ...
```

## Gas Costs (Estimated)

- Deploy Verifier: ~1-2M gas
- Deploy RorschachVerifier: ~500K gas
- Verify proof: ~300-500K gas
- Verify + store: ~400-600K gas (+ image size costs)

**Tip**: For cheaper verification, only emit events and let indexers store the full image data off-chain.

## Testing

### Local Testing with Foundry

```solidity
// test/RorschachVerifier.t.sol
pragma solidity ^0.8.0;

import "forge-std/Test.sol";
import "../contracts/RorschachVerifier.sol";

contract RorschachVerifierTest is Test {
    RorschachVerifier public verifier;

    function setUp() public {
        // Deploy verifier
        address ultraVerifier = deployCode("Verifier.sol");
        verifier = new RorschachVerifier(ultraVerifier);
    }

    function testVerifyValidProof() public {
        bytes memory proof = vm.readFileBinary("../image.groth16.proof");
        bytes memory inputs = vm.readFileBinary("../image.public_inputs");

        bool isValid = verifier.verifyRorschach(proof, inputs);
        assertTrue(isValid);
    }
}
```

Run tests:
```bash
forge test -vvv
```

## Next Steps

1. **Add `barretenberg.rs` module** to core/src/
2. **Update CLI** to support `--prove-groth16` properly
3. **Create contracts/** directory with Solidity contracts
4. **Test locally** with Foundry/Hardhat
5. **Deploy to testnet** (Sepolia/Base Sepolia)
6. **Verify on-chain** with actual proofs

Would you like me to implement these integrations now?
