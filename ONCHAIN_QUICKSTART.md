# On-Chain Verification Quick Start

## Prerequisites

Install Barretenberg CLI (`bb`):

```bash
curl -L https://raw.githubusercontent.com/AztecProtocol/aztec-packages/master/barretenberg/cpp/installation/install | bash
bbup
```

Verify installation:
```bash
bb --version
```

## Generate On-Chain Proof

### Step 1: Generate Groth16 Proof + Verifier Contract

```bash
cargo run --bin ror -- \
  --private-key 0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef \
  --prove-groth16 \
  --generate-verifier \
  --output proof_image.png
```

**This creates**:
- `proof_image.png` - Visual output
- `proof_image.groth16.proof` - Groth16 proof for on-chain verification
- `proof_image.public_inputs` - Public inputs (ABI-encoded)
- `contracts/Verifier.sol` - Solidity verifier contract

**Expected output**:
```
Generating on-chain verifiable proof...

Generating ZK proof using Noir... (this may take a while)
✓ Circuit execution successful!
  Parameters: walks=13, steps=92
Generating Groth16 proof with Barretenberg...
  This may take 30-60 seconds...
✓ Groth16 proof generated (XXX bytes)
✓ Groth16 proof saved to: proof_image.groth16.proof (XXX bytes)
✓ Public inputs saved to: proof_image.public_inputs (384 bytes)
Generating Solidity verifier contract...
✓ Verifier contract saved to: contracts/Verifier.sol
✓ Solidity verifier contract generated!
  Deploy contracts/Verifier.sol to verify on-chain

📋 Next steps:
1. Deploy contracts/Verifier.sol to your network
2. Call verify() with:
   - proof: proof_image.groth16.proof
   - publicInputs: proof_image.public_inputs

See ONCHAIN_VERIFICATION.md for details.

✓ Image saved to: proof_image.png
```

## Deploy and Verify (Foundry)

### Step 2: Setup Foundry Project

```bash
# Initialize Foundry project (if not already done)
forge init --no-commit

# Copy generated contract
cp contracts/Verifier.sol src/
```

### Step 3: Deploy Verifier

```bash
# Deploy to local testnet (Anvil)
anvil  # In separate terminal

# Deploy contract
forge create src/Verifier.sol:UltraVerifier \
  --rpc-url http://localhost:8545 \
  --private-key 0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80

# Save the deployed address
export VERIFIER_ADDRESS=0x...
```

### Step 4: Verify Proof On-Chain

```bash
# Read proof and public inputs as hex
PROOF_HEX=$(xxd -p -c 0 proof_image.groth16.proof)
INPUTS_HEX=$(xxd -p -c 0 proof_image.public_inputs)

# Call verify function
cast send $VERIFIER_ADDRESS \
  "verify(bytes,bytes)" \
  0x$PROOF_HEX \
  0x$INPUTS_HEX \
  --rpc-url http://localhost:8545 \
  --private-key 0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80

# Or use call to test without sending transaction
cast call $VERIFIER_ADDRESS \
  "verify(bytes,bytes)(bool)" \
  0x$PROOF_HEX \
  0x$INPUTS_HEX \
  --rpc-url http://localhost:8545
```

## Deploy and Verify (Hardhat)

### Setup

```bash
npm init -y
npm install --save-dev hardhat @nomicfoundation/hardhat-toolbox
npx hardhat
```

### Deploy Script

```javascript
// scripts/deploy.js
const { ethers } = require("hardhat");
const fs = require("fs");

async function main() {
  // Deploy verifier
  const Verifier = await ethers.getContractFactory("UltraVerifier");
  const verifier = await Verifier.deploy();
  await verifier.waitForDeployment();

  console.log("Verifier deployed to:", await verifier.getAddress());

  // Read proof and inputs
  const proof = fs.readFileSync("proof_image.groth16.proof");
  const publicInputs = fs.readFileSync("proof_image.public_inputs");

  // Verify proof
  const tx = await verifier.verify(proof, publicInputs);
  console.log("Verification result:", tx);
}

main().catch(console.error);
```

Run:
```bash
npx hardhat run scripts/deploy.js --network localhost
```

## Testnet Deployment

### Sepolia (Ethereum Testnet)

```bash
# Set environment variables
export SEPOLIA_RPC_URL=https://sepolia.infura.io/v3/YOUR_KEY
export PRIVATE_KEY=0x...

# Deploy
forge create src/Verifier.sol:UltraVerifier \
  --rpc-url $SEPOLIA_RPC_URL \
  --private-key $PRIVATE_KEY \
  --verify \
  --etherscan-api-key $ETHERSCAN_API_KEY
```

### Base Sepolia

```bash
export BASE_SEPOLIA_RPC_URL=https://sepolia.base.org
export PRIVATE_KEY=0x...

forge create src/Verifier.sol:UltraVerifier \
  --rpc-url $BASE_SEPOLIA_RPC_URL \
  --private-key $PRIVATE_KEY \
  --verify \
  --verifier-url https://api-sepolia.basescan.org/api \
  --etherscan-api-key $BASESCAN_API_KEY
```

## Public Inputs Format

The public inputs are ABI-encoded as:

```solidity
(uint256 walks, uint256 steps, bytes memory binaryImage)
```

**Example for walks=13, steps=92**:
```
Offset  Value
------  -----
0x00    0x000000000000000000000000000000000000000000000000000000000000000d  // walks
0x20    0x000000000000000000000000000000000000000000000000000000000000005c  // steps
0x40    0x0000000000000000000000000000000000000000000000000000000000000060  // offset to bytes
0x60    0x0000000000000000000000000000000000000000000000000000000000000100  // length = 256
0x80    [256 bytes of binary image data]
```

Total size: 384 bytes (32 + 32 + 32 + 32 + 256)

## Gas Costs (Estimated)

| Operation | Gas Cost |
|-----------|----------|
| Deploy Verifier | ~1-2M gas |
| Verify Proof | ~300-500K gas |

## Troubleshooting

### "bb: command not found"

Install Barretenberg:
```bash
curl -L https://raw.githubusercontent.com/AztecProtocol/aztec-packages/master/barretenberg/cpp/installation/install | bash
bbup
```

### "Circuit file not found"

Make sure you ran `nargo compile` first:
```bash
cd circuits
nargo compile
```

### "Witness file not found"

Generate the witness with `--prove` first:
```bash
cargo run --bin ror -- --private-key 0x... --prove --output test.png
```

### Verification fails on-chain

1. Check that proof and public inputs match
2. Verify the contract was deployed correctly
3. Test locally with Anvil first
4. Check gas limits (increase if needed)

## Advanced: Custom Wrapper Contract

See [ONCHAIN_VERIFICATION.md](ONCHAIN_VERIFICATION.md#step-2-create-custom-verifier-contract-wrapper) for a wrapper contract that:
- Emits events
- Stores verified images
- Adds access control

## Next Steps

1. ✅ Generate Groth16 proof
2. ✅ Deploy verifier contract
3. ✅ Verify proof on-chain
4. Create custom wrapper with your business logic
5. Build frontend to interact with contracts
6. Consider batching multiple proofs for efficiency

## Summary

You now have:
- ✅ Working on-chain proof generation
- ✅ Solidity verifier contract
- ✅ Tools to deploy and verify

The full workflow is:
```
Private Key → Noir Circuit → Witness → Barretenberg → Groth16 Proof → On-Chain Verification ✅
```

All working end-to-end!
