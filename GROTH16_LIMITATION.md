# Groth16 Circuit Size Limitation

## Issue

The current Noir circuit (83MB compiled) is **too large for Groth16** proof generation:

```
Error: "Length is too large"
```

This is a known Groth16 limitation - it can only handle circuits up to ~50MB.

## Why So Large?

Current circuit parameters:
- `MAX_WALKS = 20`
- `MAX_STEPS = 300`
- Total loop iterations: 20 × 300 = 6,000 iterations
- Compiled circuit: 83MB

## Solutions

### Option 1: Reduce Circuit Size (Quick Fix)

Reduce the constants in [circuits/src/main.nr](circuits/src/main.nr:92-93):

```noir
// Current (too large for Groth16)
let MAX_WALKS: u64 = 20;  // line 92
let MAX_STEPS: u64 = 300; // line 116

// Optimized (should work with Groth16)
let MAX_WALKS: u64 = 10;
let MAX_STEPS: u64 = 150;
```

**Trade-off**: Limits the maximum complexity of generated images.

**Steps**:
```bash
# 1. Edit circuits/src/main.nr
nano circuits/src/main.nr  # Change MAX_WALKS to 10, MAX_STEPS to 150

# 2. Recompile circuit
cd circuits
nargo compile
cd ..

# 3. Try Groth16 again
cargo run --bin ror -- \
  --private-key 0x1111111111111111111111111111111111111111111111111111111111111111 \
  --prove-groth16 --generate-verifier --output proof_image.png
```

### Option 2: Use UltraPlonk/Honk Backend (Production Solution)

UltraPlonk (Honk) is the modern proving system for Noir with no circuit size limits.

**Status**: ⚠️ Not yet implemented in this project

**Advantages**:
- No circuit size limits
- Faster proving time
- Native Noir support
- Better for production

**Implementation needed**:
1. Update `barretenberg.rs` to use `bb prove_ultra_honk`
2. Update verifier contract generation
3. Use UltraPlonk verifier in Solidity

**Barretenberg commands**:
```bash
# UltraPlonk proof generation (works with large circuits)
bb prove_ultra_honk -b ./circuits/target/circuits.json -w ./circuits/target/circuits.gz -o ./proof

# Generate UltraPlonk verifier contract
bb contract_ultra_honk -b ./circuits/target/circuits.json -o ./contracts/UltraPlonkVerifier.sol
```

### Option 3: Use Witness-Only Mode (Current Workaround)

For testing without on-chain verification:

```bash
# Generate witness only (always works)
cargo run --bin ror -- \
  --private-key 0x1111111111111111111111111111111111111111111111111111111111111111 \
  --prove \
  --output test.png
```

This creates:
- `test.png` - Image
- `test.witness.gz` - Noir witness (can be verified off-chain)

## Recommended Approach

**For immediate testing**: Use Option 1 (reduce circuit size)
**For production**: Implement Option 2 (UltraPlonk)

## Circuit Size Comparison

| MAX_WALKS | MAX_STEPS | Iterations | Estimated Size | Groth16 Support |
|-----------|-----------|------------|----------------|-----------------|
| 20 | 300 | 6,000 | ~83MB | ❌ Too large |
| 15 | 200 | 3,000 | ~40MB | ⚠️ Borderline |
| 10 | 150 | 1,500 | ~20MB | ✅ Should work |
| 10 | 100 | 1,000 | ~15MB | ✅ Comfortable |

## Testing Without Groth16

You can still test the full stack without Groth16:

1. **Generate witness** with `--prove`
2. **Test Foundry contracts** with mock data
3. **Deploy when UltraPlonk is implemented**

## Future Work

- [ ] Implement UltraPlonk backend in `barretenberg.rs`
- [ ] Add `--prove-ultraplonk` flag
- [ ] Generate UltraPlonk verifier contracts
- [ ] Update Foundry tests for UltraPlonk
- [ ] Benchmark proving times (UltraPlonk vs Groth16)

## Resources

- [Noir Proving Backends](https://noir-lang.org/docs/reference/proving_backends)
- [Barretenberg Documentation](https://github.com/AztecProtocol/barretenberg)
- [UltraPlonk Paper](https://eprint.iacr.org/2019/953)

---

**Current Status**: Circuit too large for Groth16. Use reduced parameters or implement UltraPlonk.

**Last Updated**: 2025-12-30
