# Rorschach Solana Program

Zero-knowledge proof verification of Rorschach image generation on Solana using native Groth16 syscalls.

## Architecture

```
Private Key → Noir Circuit → Groth16 Proof (local) → Solana Verification → On-chain Record
```

### Why Solana?

- **Native Groth16**: Built-in `groth16::verify()` syscall - no custom verifier needed
- **Cost**: 1000x cheaper than Ethereum ($0.0005 vs $50-100 per verification)
- **Speed**: Sub-second finality vs 15 seconds
- **No external dependencies**: Local proof generation, Solana verification

## Project Structure

```
solana-program/
├── program/              # Solana program (smart contract)
│   └── src/lib.rs       # Groth16 verification logic
├── client/              # Rust CLI client
│   └── src/main.rs      # Submit proofs to Solana
├── build.sh             # Build script
└── README.md            # This file
```

## Prerequisites

### 1. Install Solana CLI

```bash
sh -c "$(curl -sSfL https://release.solana.com/stable/install)"
export PATH="$HOME/.local/share/solana/install/active_release/bin:$PATH"
solana --version
```

### 2. Install Node.js (for snarkjs)

```bash
# Ubuntu/Debian
curl -fsSL https://deb.nodesource.com/setup_18.x | sudo -E bash -
sudo apt-get install -y nodejs

node --version  # Should be v18+
```

### 3. Install snarkjs (for Groth16 proof generation)

```bash
npm install -g snarkjs
snarkjs --version
```

## Quick Start

### Step 1: Build the Program

```bash
cd solana-program
./build.sh
```

This creates:
- `target/deploy/rorschach_solana.so` - Solana program binary
- `target/release/rorschach-client` - CLI client

### Step 2: Start Local Validator

```bash
# In a separate terminal
solana-test-validator
```

### Step 3: Deploy Program

```bash
# Configure for local testing
solana config set --url localhost

# Deploy
solana program deploy target/deploy/rorschach_solana.so
```

This outputs a Program ID like: `Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS`

### Step 4: Generate Proof (from main project)

```bash
cd ..  # Back to project root

# Generate image and proof with Noir
cargo run --bin ror -- \
  --private-key 0x1111111111111111111111111111111111111111111111111111111111111111 \
  --prove \
  --output myimage.png

# This creates:
# - myimage.png - The image
# - circuits/target/circuits.gz - Witness file
```

### Step 5: Convert Proof to Groth16 (using snarkjs)

**Note**: This step requires implementing witness format conversion. For now, Barretenberg is the recommended path once fixed.

```bash
# TODO: Convert Noir witness to snarkjs format
# This is documented in ../core/src/snarkjs.rs
```

### Step 6: Initialize Verifying Key

```bash
cd solana-program

./target/release/rorschach-client init-vk \
  --vk-file path/to/verifying-key.bin \
  --program-id Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS
```

### Step 7: Submit Proof for Verification

```bash
./target/release/rorschach-client verify \
  --proof path/to/proof.bin \
  --public-inputs path/to/public-inputs.bin \
  --program-id Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS \
  --vk-account <VK_ACCOUNT_FROM_STEP_6>
```

## Program Instructions

### 1. InitializeVerifyingKey

Initialize the verifying key account (one-time setup).

**Accounts:**
- `[writable]` Verifying key account
- `[signer]` Authority
- `[]` System program

**Data:**
- `verifying_key: Vec<u8>` - Groth16 verifying key bytes

### 2. VerifyProof

Verify a Groth16 proof and create verification record.

**Accounts:**
- `[writable]` Verification record (PDA)
- `[signer]` User wallet
- `[]` Verifying key account
- `[]` System program
- `[]` Instructions sysvar

**Data:**
- `proof: Vec<u8>` - Groth16 proof
- `public_inputs: Vec<u8>` - Public inputs

**On-chain Record:**
```rust
struct VerificationRecord {
    user: Pubkey,           // Who submitted
    timestamp: i64,         // When verified
    public_inputs_hash: [u8; 32],  // Hash of inputs
    verified: bool,         // Always true if exists
}
```

## Development

### Running Tests

```bash
cd program
cargo test
```

### Deploying to Devnet

```bash
# Configure for devnet
solana config set --url devnet

# Airdrop SOL for testing
solana airdrop 2

# Deploy
solana program deploy target/deploy/rorschach_solana.so

# Note the program ID and update your client calls
```

### Deploying to Mainnet

```bash
# Configure for mainnet
solana config set --url mainnet-beta

# Deploy (costs ~0.1 SOL)
solana program deploy target/deploy/rorschach_solana.so
```

## Cost Analysis

| Operation | Lamports | SOL | USD (@ $100/SOL) |
|-----------|----------|-----|------------------|
| Deploy program | ~100M | 0.1 | $10 |
| Initialize VK | ~5M | 0.005 | $0.50 |
| Verify proof | ~500 | 0.000005 | $0.0005 |

**Comparison to Ethereum:**
- Ethereum verification: ~$50-100
- Solana verification: ~$0.0005
- **Savings: 100,000x cheaper!**

## Proof Generation

### Using Barretenberg (recommended when fixed)

```bash
cargo run --bin ror -- \
  --private-key $KEY \
  --prove-groth16 \
  --output image.png
```

### Using snarkjs (alternative)

See [../core/src/snarkjs.rs](../core/src/snarkjs.rs) for implementation details.

## Troubleshooting

### "Program failed to complete"

- Check account sizes are correct
- Ensure VK account is initialized
- Verify proof format matches expected structure

### "Proof verification failed"

- Ensure proof and public inputs match
- Check VK account contains correct key
- Verify witness was generated correctly

### "Insufficient funds"

```bash
# Devnet
solana airdrop 2

# Mainnet
# Transfer SOL to your wallet
```

## Resources

- [Solana Documentation](https://docs.solana.com)
- [Solana Cookbook](https://solanacookbook.com)
- [alt_bn128 Syscalls](https://docs.solana.com/developing/runtime-facilities/programs#alt_bn128)
- [snarkjs GitHub](https://github.com/iden3/snarkjs)

## Next Steps

1. **Implement NFT Minting**: Add Metaplex integration to mint NFTs upon verification
2. **Web Frontend**: Build web UI for proof submission
3. **Indexer**: Create indexer to query verification records
4. **Batch Verification**: Optimize for multiple proofs in one transaction

## License

Same as parent project
