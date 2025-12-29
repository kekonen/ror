# Groth16 Proof Verification Status

## Current Issue

**Selector Mismatch**: The Docker prover generates proofs with selector `0x1f1d6202`, but risc0-ethereum v3.0/v3.0.1 expects selector `0x73c457ba`.

## Root Cause

The selector is a 4-byte hash of (CONTROL_ROOT + BN254_CONTROL_ID + Groth16_VK). Our Docker prover image uses a different Groth16 verification key than what's published in risc0-ethereum releases.

- **Docker Prover**: `risczero/risc0-groth16-prover:v2025-04-03.1`
- **risc0 Version**: v3.0
- **risc0-ethereum Tried**: v3.0.0, v3.0.1, release-3.0 branch
- **All produce selector**: `0x73c457ba` (doesn't match our proof's `0x1f1d6202`)

## What Works

✅ STARK proof generation
✅ Groth16 proof generation via Docker
✅ Journal encoding/decoding
✅ Contract deployment
✅ IMAGE_ID extraction
✅ Control ID extraction
✅ All contract tests pass with mock data

## What Doesn't Work

❌ On-chain Groth16 proof verification (selector mismatch)

## Files Generated

Successfully generated proofs (but can't verify on-chain yet):
- `test_groth16_clean.seal` (256 bytes Groth16 proof)
- `test_groth16_clean.journal` (416 bytes journal data)
- `test_groth16_clean.groth16.proof` (full receipt with metadata)
- `test_groth16_clean.png` (visualized Rorschach image)

## Potential Solutions

### Option 1: Use Unreleased risc0-ethereum Version
The Docker image `v2025-04-03.1` suggests it's from a future/unreleased version. Need to:
- Check risc0-ethereum main/development branches
- Or extract verification key directly from Docker container

### Option 2: Use Different Docker Prover Version
Try older/different Docker prover images that match released risc0-ethereum versions:
```bash
# Check available Docker images
docker search risczero/risc0-groth16-prover
```

### Option 3: Extract Verification Key from Docker
The Docker container contains the Groth16 verification key. Could potentially:
1. Extract the verification key from the container
2. Deploy a custom verifier contract with that key

### Option 4: Use Bonsai Proving Service
Instead of local Docker proving, use Bonsai (risc0's cloud proving service) which should have matching verification keys.

## Testing Without On-Chain Verification

For development/testing purposes, the codebase includes:
- Mock verifier tests (contracts/foundry/test/RorschachVerifier.t.sol)
- These validate all logic except cryptographic verification

## Next Steps

1. **Immediate**: Test with Bonsai proving service (if available)
2. **Short-term**: Contact risc0 team about Docker image / risc0-ethereum version alignment
3. **Long-term**: Wait for risc0-ethereum release matching the Docker prover

## Contract Addresses (when deployed)

Currently deployed to local/fork for testing:
- RiscZeroGroth16Verifier: Deployed with v3.0 ControlID
- RorschachVerifier: Deployed with IMAGE_ID `0xc116a8105dd1141806aa804bf8cca46d74a725bf8ef46006ef634a61e357f582`

## Technical Details

### Control IDs (v3.0)
```solidity
CONTROL_ROOT = 0xa54dc85ac99f851c92d7c96d7318af41dbe7c0194edfcc37eb4d422a998c1f56
BN254_CONTROL_ID = 0x04446e66d300eb7fb45c9726bb53c793dda407a62e9601618bb43c5c14657ac0
```

### Proof Metadata
```
Selector in proof: 0x1f1d6202
Expected by v3.0: 0x73c457ba
```

### Proof Generation Command
```bash
cargo run --release --features groth16 -- \
  --private-key 0x... \
  -o output.png \
  --prove-groth16
```

## References

- risc0 zkVM: https://github.com/risc0/risc0
- risc0-ethereum: https://github.com/risc0/risc0-ethereum
- Docker prover: https://hub.docker.com/r/risczero/risc0-groth16-prover
