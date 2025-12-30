# Commands Quick Reference

Complete reference for all Rorschach operations - setup, generation, proving, and testing.

## Table of Contents
- [Setup](#setup)
- [Image Generation](#image-generation)
- [Proof Generation](#proof-generation)
- [On-Chain Verification](#on-chain-verification)
- [Testing](#testing)
- [Development](#development)

---

## Setup

### Prerequisites

Install the required tools:

```bash
# 1. Rust (if not installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 2. Nargo (Noir compiler)
curl -L https://raw.githubusercontent.com/noir-lang/noirup/main/install | bash
noirup

# 3. Barretenberg (for Groth16 proofs)
curl -L https://raw.githubusercontent.com/AztecProtocol/aztec-packages/master/barretenberg/cpp/installation/install | bash
bbup

# 4. Foundry (for Solidity testing)
curl -L https://foundry.paradigm.xyz | bash
foundryup
```

Verify installations:
```bash
rustc --version    # Should be 1.70+
nargo --version    # Should be 1.0.0-beta.17+
bb --version       # Should show Barretenberg version
forge --version    # Should show Foundry version
```

### First-Time Setup

```bash
# Clone and build
git clone <your-repo>
cd ror

# Compile Noir circuit (takes ~6 minutes, creates 83MB circuits.json)
cd circuits
nargo compile
cd ..

# Build Rust project
cargo build --release

# Verify everything works
cargo test
```

**Expected output:**
```
Compiling circuits v0.1.0 (/home/user/ror/circuits)
Finished `release` profile [optimized] target(s) in 6m 23s
```

---

## Image Generation

### Normal Mode (No Proof)

**Basic generation** (uses Noir circuit for consistency):
```bash
cargo run --bin ror -- \
  --private-key 0x1111111111111111111111111111111111111111111111111111111111111111 \
  --output image.png
```

**Expected output:**
```
Generating ZK proof using Noir... (this may take a while)
✓ Circuit execution successful!
  Parameters: walks=6, steps=268
✓ Image saved to: image.png
```

**Custom colors:**
```bash
cargo run --bin ror -- \
  --private-key 0x1111...1111 \
  --color 255,100,50 \
  --background 20,20,40 \
  --output custom_colors.png
```

**Custom parameters** (bypasses Noir circuit, faster but not provable):
```bash
cargo run --bin ror -- \
  --private-key 0x1111...1111 \
  --walks 10 \
  --steps 200 \
  --output fast_mode.png
```

### All CLI Options

```bash
cargo run --bin ror -- --help
```

**Available flags:**
- `--private-key <HEX>` - 32-byte private key (required)
- `--output <PATH>` - Output PNG path (required)
- `--color <R,G,B>` - Foreground color (default: 255,100,50)
- `--background <R,G,B>` - Background color (default: 10,10,20)
- `--walks <N>` - Manual walks parameter (bypasses Noir)
- `--steps <N>` - Manual steps parameter (bypasses Noir)
- `--prove` - Generate Noir witness
- `--prove-groth16` - Generate Groth16 proof
- `--generate-verifier` - Generate Solidity verifier contract
- `--debug <LEVEL>` - Debug verbosity (0-2)

---

## Proof Generation

### Noir Witness (Off-Chain Proof)

```bash
cargo run --bin ror -- \
  --private-key 0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef \
  --prove \
  --output proof_test.png
```

**Creates:**
- `proof_test.png` - Visual output
- `proof_test.witness.gz` - Noir witness (12MB)
- `circuits/target/circuits.gz` - Original witness

**Expected output:**
```
Generating ZK proof using Noir... (this may take a while)
✓ Circuit execution successful!
  Parameters: walks=13, steps=92
✓ Proof generated successfully!
  Witness saved to: proof_test.witness.gz
  Image saved to: proof_test.png
```

### Groth16 Proof (On-Chain Verification)

**Prerequisites:** Barretenberg installed (`bb --version`)

```bash
cargo run --bin ror -- \
  --private-key 0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef \
  --prove-groth16 \
  --generate-verifier \
  --output proof_image.png
```

**Creates:**
- `proof_image.png` - Visual output
- `proof_image.groth16.proof` - Groth16 proof (binary)
- `proof_image.public_inputs` - ABI-encoded public inputs (384 bytes)
- `contracts/Verifier.sol` - Solidity verifier contract

**Expected output:**
```
Generating on-chain verifiable proof...

Generating ZK proof using Noir... (this may take a while)
✓ Circuit execution successful!
  Parameters: walks=13, steps=92
Generating Groth16 proof with Barretenberg...
  This may take 30-60 seconds...
✓ Groth16 proof generated (XXX bytes)
✓ Public inputs saved to: proof_image.public_inputs (384 bytes)
Generating Solidity verifier contract...
✓ Verifier contract saved to: contracts/Verifier.sol

📋 Next steps:
1. Deploy contracts/Verifier.sol to your network
2. Call verify() with proof and publicInputs

See ONCHAIN_VERIFICATION.md for details.
```

---

## On-Chain Verification

### Deploy Verifier Contract

**Using Foundry:**

```bash
# Copy generated verifier to Foundry project
cp contracts/Verifier.sol contracts/foundry/src/

# Deploy to local testnet (Anvil)
anvil &  # In separate terminal

# Deploy
forge create contracts/foundry/src/Verifier.sol:UltraVerifier \
  --rpc-url http://localhost:8545 \
  --private-key 0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80

# Save the deployed address
export VERIFIER_ADDRESS=0x...
```

**Deploy to testnet (Sepolia):**

```bash
# Set environment
export SEPOLIA_RPC_URL=https://sepolia.infura.io/v3/YOUR_KEY
export PRIVATE_KEY=0x...

# Deploy
forge create contracts/foundry/src/Verifier.sol:UltraVerifier \
  --rpc-url $SEPOLIA_RPC_URL \
  --private-key $PRIVATE_KEY \
  --verify \
  --etherscan-api-key $ETHERSCAN_API_KEY
```

### Verify Proof On-Chain

```bash
# Read proof and public inputs as hex
PROOF_HEX=$(xxd -p -c 0 proof_image.groth16.proof)
INPUTS_HEX=$(xxd -p -c 0 proof_image.public_inputs)

# Call verify function
cast call $VERIFIER_ADDRESS \
  "verify(bytes,bytes)(bool)" \
  0x$PROOF_HEX \
  0x$INPUTS_HEX \
  --rpc-url http://localhost:8545

# Expected: true (0x01)
```

---

## Testing

### Rust Tests

```bash
# Run all tests
cargo test

# Run specific module
cargo test noir::

# Run with output
cargo test -- --nocapture

# Run integration test
cargo run --bin test_noir_integration
```

**Expected output:**
```
running 10 tests
test binary_image::tests::test_binary_packing ... ok
test derive::tests::test_derive_parameters ... ok
test noir::tests::test_generate_prover_toml ... ok
test noir::tests::test_parse_nargo_output ... ok
...
test result: ok. 10 passed
```

### Circuit Tests

```bash
cd circuits

# Run Noir tests
nargo test

# Run specific test
nargo test test_derive_parameters
```

**Expected output:**
```
[circuits] Testing test_derive_parameters ... ok
[circuits] Testing test_pixel_setting ... ok
[circuits] 2 tests passed
```

### Solidity Tests

**Prerequisites:** Barretenberg installed, proof files generated

```bash
cd contracts/foundry

# Copy generated verifier
cp ../../contracts/Verifier.sol src/

# Run all tests
forge test -vv

# Run specific test
forge test --match-test testVerifyValidProof -vvv

# Run with gas report
forge test --gas-report

# Run fork tests (requires RPC URL)
forge test --match-path "test/*.fork.t.sol" \
  --fork-url $SEPOLIA_RPC_URL -vv
```

**Expected output:**
```
Running 3 tests for test/NoirVerifier.t.sol:NoirVerifierTest
[PASS] testVerifyValidProof() (gas: 234567)
[PASS] testRejectInvalidProof() (gas: 123456)
[PASS] testRejectInvalidPublicInputs() (gas: 123456)
Test result: ok. 3 passed
```

---

## Development

### Circuit Development

```bash
cd circuits

# Compile circuit (takes ~6 minutes)
nargo compile

# Run tests
nargo test

# Execute with specific inputs (creates Prover.toml first)
nargo execute

# Check circuit info
nargo info

# Format code
nargo fmt
```

### Debugging

```bash
# Debug level 1: Basic info
cargo run --bin ror -- --private-key 0x... --debug 1 --output test.png

# Debug level 2: Verbose output
cargo run --bin ror -- --private-key 0x... --debug 2 --output test.png
```

### Generate Multiple Images

```bash
# Batch generation script
for i in {1..10}; do
  KEY=$(printf "0x%064d" $i)
  cargo run --bin ror -- \
    --private-key $KEY \
    --output images/image_$i.png
done
```

### Benchmarking

```bash
# Time normal generation
time cargo run --release --bin ror -- \
  --private-key 0x1111...1111 \
  --output bench.png

# Time with proof
time cargo run --release --bin ror -- \
  --private-key 0x1111...1111 \
  --prove \
  --output bench_proof.png
```

**Expected times:**
- Normal mode (with Noir): ~10 seconds
- With Noir witness: ~10 seconds
- With Groth16: ~40-60 seconds (includes proving time)

---

## Common Workflows

### Workflow 1: Generate Provable Image

```bash
# 1. Generate with proof
cargo run --release --bin ror -- \
  --private-key 0x1234...cdef \
  --prove-groth16 \
  --generate-verifier \
  --output my_image.png

# 2. Deploy verifier
cp contracts/Verifier.sol contracts/foundry/src/
cd contracts/foundry
forge create src/Verifier.sol:UltraVerifier \
  --rpc-url $SEPOLIA_RPC_URL \
  --private-key $PRIVATE_KEY

# 3. Verify on-chain
PROOF_HEX=$(xxd -p -c 0 ../../my_image.groth16.proof)
INPUTS_HEX=$(xxd -p -c 0 ../../my_image.public_inputs)
cast call $VERIFIER_ADDRESS \
  "verify(bytes,bytes)(bool)" \
  0x$PROOF_HEX 0x$INPUTS_HEX \
  --rpc-url $SEPOLIA_RPC_URL
```

### Workflow 2: Quick Testing

```bash
# 1. Generate test image
cargo run --bin ror -- \
  --private-key 0x1111...1111 \
  --output test.png

# 2. Run tests
cargo test
cd circuits && nargo test && cd ..

# 3. Generate proof for testing
cargo run --bin ror -- \
  --private-key 0x1111...1111 \
  --prove-groth16 \
  --output proof_image.png

# 4. Run Solidity tests
cd contracts/foundry
forge test -vv
```

### Workflow 3: Production Deployment

```bash
# 1. Generate production proof
cargo run --release --bin ror -- \
  --private-key $PRODUCTION_KEY \
  --prove-groth16 \
  --generate-verifier \
  --output production.png

# 2. Test locally
cd contracts/foundry
forge test -vv

# 3. Deploy to mainnet
forge create src/Verifier.sol:UltraVerifier \
  --rpc-url $ETH_RPC_URL \
  --private-key $DEPLOYER_KEY \
  --verify \
  --etherscan-api-key $ETHERSCAN_API_KEY

# 4. Verify proof on mainnet
cast call $MAINNET_VERIFIER \
  "verify(bytes,bytes)(bool)" \
  0x$PROOF_HEX 0x$INPUTS_HEX \
  --rpc-url $ETH_RPC_URL
```

---

## Troubleshooting

### "bb: command not found"

**Install Barretenberg:**
```bash
curl -L https://raw.githubusercontent.com/AztecProtocol/aztec-packages/master/barretenberg/cpp/installation/install | bash
bbup
bb --version
```

### "nargo: command not found"

**Install Nargo:**
```bash
curl -L https://raw.githubusercontent.com/noir-lang/noirup/main/install | bash
noirup
nargo --version
```

### "Circuit file not found"

**Compile the circuit:**
```bash
cd circuits
nargo compile
ls -lh target/circuits.json  # Should be ~83MB
```

### "Witness file not found"

**Generate a witness first:**
```bash
cargo run --bin ror -- \
  --private-key 0x1111...1111 \
  --prove \
  --output test.png
```

### Verification fails on-chain

**Check:**
1. Proof and public inputs files exist
2. Correct verifier contract deployed
3. Files match (same private key used)
4. Gas limit sufficient (try --gas-limit 1000000)

```bash
# Test locally first
anvil &
forge test -vv
```

### Different images in normal vs proof mode

This is now fixed! Both modes use Noir circuit. If you see different images:
```bash
# Delete old images and regenerate
rm -f *.png
cargo run --bin ror -- --private-key 0x1111...1111 --output test1.png
cargo run --bin ror -- --private-key 0x1111...1111 --prove --output test2.png

# Verify they match
sha256sum test1.png test2.png
# Should be identical
```

---

## Environment Variables

Create `.env` file in project root:

```bash
# RPC URLs
SEPOLIA_RPC_URL=https://sepolia.infura.io/v3/YOUR_KEY
BASE_SEPOLIA_RPC_URL=https://sepolia.base.org
ETH_RPC_URL=https://mainnet.infura.io/v3/YOUR_KEY

# Private keys (DO NOT COMMIT)
PRIVATE_KEY=0x...
DEPLOYER_KEY=0x...

# Etherscan API keys
ETHERSCAN_API_KEY=your_key
BASESCAN_API_KEY=your_key

# Deployed addresses
VERIFIER_ADDRESS=0x...
```

Load variables:
```bash
source .env
# or
export $(cat .env | xargs)
```

---

## Performance Notes

| Operation | Time | Output Size |
|-----------|------|-------------|
| Circuit compilation | ~6 min | 83MB circuits.json |
| Normal generation | ~10 sec | 6KB PNG |
| Noir witness | ~10 sec | 12MB .gz |
| Groth16 proof | ~40-60 sec | ~1-2KB proof |
| Verifier deployment | ~30 sec | ~1-2M gas |
| Proof verification | <1 sec | ~300-500K gas |

---

## Further Documentation

- [CONSISTENCY_FIX.md](CONSISTENCY_FIX.md) - Why normal and proof modes now match
- [ONCHAIN_VERIFICATION.md](ONCHAIN_VERIFICATION.md) - Detailed on-chain verification guide
- [ONCHAIN_QUICKSTART.md](ONCHAIN_QUICKSTART.md) - Quick start for on-chain proofs
- [MIGRATION_COMPLETE.md](MIGRATION_COMPLETE.md) - Risc0 → Noir migration details
- [README.md](README.md) - Project overview and architecture

---

**Last updated:** 2025-12-30
