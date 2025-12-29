# Quick Start Guide

## Complete End-to-End Workflow

### Phase 1: Generate Groth16 Proof ✅

```bash
# Generate a Groth16 proof (takes ~2 hours first time)
./target/release/ror \
  --private-key 0x1111111111111111111111111111111111111111111111111111111111111111 \
  --prove-groth16 \
  --output my_proof.png

# Outputs:
# - my_proof.seal (256 bytes)
# - my_proof.journal (448 bytes)
# - my_proof.png (visual)
```

### Phase 2: Test with Fork ✅

```bash
cd contracts/foundry

# Test with real risc0 verifier on mainnet fork
forge test --match-contract Fork -vvv --fork-url mainnet

# Expected: 3/4 tests pass (4th needs GUEST_ID)
```

### Phase 3: Deploy to Testnet

```bash
# 1. Get Sepolia ETH from faucet
# https://sepoliafaucet.com

# 2. Set your private key in .env
cd contracts/foundry
nano .env  # Add your PRIVATE_KEY

# 3. Deploy
forge script script/Deploy.s.sol \
  --rpc-url sepolia \
  --broadcast \
  --verify

# 4. Save the deployed address
export RORSCHACH_VERIFIER=0x<address_from_output>
```

### Phase 4: Submit Proof On-Chain

```bash
# Submit your proof to the deployed contract
forge script script/SubmitProof.s.sol \
  --rpc-url sepolia \
  --broadcast
```

## Commands Reference

### Rust (Proof Generation)

```bash
# Build with Groth16 support
cargo build --release --features groth16

# Generate proof
./target/release/ror --private-key 0x... --prove-groth16 --output proof.png

# Generate with random key
./target/release/ror --generate-key --prove-groth16 --output proof.png
```

### Foundry (Smart Contracts)

```bash
cd contracts/foundry

# Compile
forge build

# Run unit tests (mock verifier)
forge test -vvv

# Run fork tests (real verifier)
forge test --match-contract Fork -vvv --fork-url mainnet

# Deploy to network
forge script script/Deploy.s.sol --rpc-url <network> --broadcast

# Submit proof
RORSCHACH_VERIFIER=0x... forge script script/SubmitProof.s.sol --rpc-url <network> --broadcast
```

## File Locations

**Proofs**: `/home/daniil/ror/`
- `test_groth16.seal`
- `test_groth16.journal`
- `test_groth16.png`

**Contracts**: `/home/daniil/ror/contracts/foundry/src/`
- `RorschachVerifier.sol`
- `ImageID.sol` (needs GUEST_ID update)
- `IRiscZeroVerifier.sol`

**Config**: `/home/daniil/ror/contracts/foundry/`
- `.env` (your settings)
- `foundry.toml`

## Current Status

✅ **Groth16 Proofs**: Working (256 byte seal + 448 byte journal)
✅ **Contracts**: Compiled and tested
✅ **Fork Tests**: 3/4 passing with real risc0 verifier
✅ **Deploy Scripts**: Ready for any network
⏸️ **ImageID**: Needs GUEST_ID (placeholder currently)

## Next Action

**Update ImageID.sol with correct GUEST_ID**, then you'll have full end-to-end verification working!

## Networks

| Network | RPC Alias | risc0 Verifier |
|---------|-----------|----------------|
| Mainnet | `mainnet` | `0x8EaB2D97Dfce405A1692a21b3ff3A172d593D319` |
| Sepolia | `sepolia` | `0x925d8331ddc0a1F0d96E68CF073DFE1d92b69187` |
| Local | `localhost` | Fork from mainnet |

## Documentation

- [`GROTH16_TEST_RESULTS.md`](GROTH16_TEST_RESULTS.md) - Phase 1 results
- [`FOUNDRY_SETUP_COMPLETE.md`](FOUNDRY_SETUP_COMPLETE.md) - Phase 2 summary
- [`FORK_TESTS_WORKING.md`](FORK_TESTS_WORKING.md) - Phase 3 details
- [`IMPLEMENTATION_PLAN.md`](IMPLEMENTATION_PLAN.md) - Original plan

## Help

```bash
# Rust
./target/release/ror --help

# Foundry
forge --help
forge test --help
forge script --help
```
