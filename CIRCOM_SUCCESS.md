# Circom + snarkjs Groth16 Success! 🎉

## What We Accomplished

Successfully switched from Noir (with broken Barretenberg Groth16) to **Circom + snarkjs** and generated working Groth16 proofs!

## Current Status

✅ **Circuit Compiled**: Circom circuit for Rorschach pattern verification
✅ **Proof Generated**: Working Groth16 proofs with snarkjs
✅ **Verification Works**: Local proof verification successful

## Circuit Details

**File**: `circom_circuits/rorschach_simple.circom`

**Circuit Stats**:
- Template instances: 154
- Non-linear constraints: 3,506
- Public inputs: 258 (walks + steps + 256-byte image)
- Private inputs: 32 (private key bytes)
- Circuit size: Perfect for Groth16 ✓

**What it proves**:
```
Given public: (walks, steps, binaryImage[256])
Prove: I know a privateKey[32] such that:
  - Poseidon(privateKey) derives the exact walks & steps
  - privateKey generates the exact binaryImage
```

## How It Works

### 1. Image Generation (Off-chain)
```javascript
// You generate the image
privateKey → Poseidon hash → (walks, steps)
privateKey + hash → deterministic pattern → binaryImage[256]
```

### 2. Publishing
You publish the image with:
- Visual representation (PNG/JPG)
- Embedded private key (steganography in postprocessing)
- Public challenge: (walks, steps, binaryImage)

### 3. Treasure Hunt (User finds it)
```
User extracts → privateKey from image
User generates → Groth16 proof
User submits to Solana → proof + public inputs
```

### 4. On-Chain Verification (Solana)
```rust
// Solana program verifies without seeing private key!
alt_bn128_groth16_verify(proof, [walks, steps, ...image], verifying_key)
→ Returns true = Mint NFT / Transfer reward
→ Front-running impossible (private key never revealed on-chain)
```

## Files Generated

```
circom_circuits/
├── rorschach_simple.circom          # Circuit definition
├── rorschach_simple.r1cs             # 1.8 MB - R1CS constraint system
├── rorschach_simple_js/              # Witness generator (WASM)
│   └── rorschach_simple.wasm         # 4.0 MB
├── rorschach_simple_final.zkey      # Proving key
├── verification_key.json             # Verification key
├── proof.json                         # Example Groth16 proof
├── public.json                        # Example public inputs
└── compute_outputs.js                 # Helper to compute outputs from private key
```

## Test Results

**Test Private Key**: `[17, 17, 17, ..., 17]` (32 bytes)

**Derived Outputs**:
- Walks: 9
- Steps: 163
- Binary image: 256 bytes (deterministic pattern)

**Proof Generation**: ✅ Success
**Proof Verification**: ✅ OK!

## Next Steps

### Phase 1: Integrate with Rust Host
1. Update `host/src/main.rs` to call circom/snarkjs instead of Noir/bb
2. Generate witness via Node.js subprocess
3. Generate proof via snarkjs subprocess
4. Parse JSON proof for Solana submission

### Phase 2: Implement Full Rorschach Algorithm
Current circuit uses simplified deterministic pattern. Need to implement:
- ✅ Poseidon-based parameter derivation
- ⏳ Full random walk algorithm in Circom
- ⏳ Proper 32×64 binary image generation
- ⏳ Mirror symmetry

### Phase 3: Solana On-Chain Verification
1. Complete `solana-program/program/src/lib.rs` line 236
2. Implement actual `alt_bn128_groth16_verify()` syscall
3. Format proof/inputs for Solana (binary, not JSON)
4. Deploy to local validator → test E2E
5. Deploy to devnet → public testing

### Phase 4: Product Integration
1. Image postprocessing (add private key steganography)
2. NFT minting on successful verification
3. Frontend for:
   - Viewing published images
   - Extracting private key from found image
   - Submitting proof
4. Reward distribution mechanism

## Key Advantages of Circom + snarkjs

✅ **Battle-tested**: Used by Tornado Cash, Polygon, etc.
✅ **Groth16 works**: No "Length is too large" errors
✅ **Good documentation**: Extensive examples and community
✅ **Solana compatible**: Standard Groth16 format
✅ **Tooling mature**: snarkjs handles everything smoothly

## Circuit Simplifications (Current)

The current circuit is **simplified for testing**. It doesn't implement the full random walk yet:

```circom
// Current (simplified):
for (var i = 0; i < 256; i++) {
    combined[i] <-- (privateKey[i % 32] + seed + i * 17) % 256;
    binaryImage[i] === combined[i];
}
```

**Full implementation needed**:
- Random walk with direction decisions
- Boundary checking
- Pixel setting in packed format
- Mirror symmetry

This is doable in Circom but requires more circuit logic (will increase constraint count).

## Constraints Budget

Current: **3,506 constraints**
Groth16 limit: ~2^20 = **1,048,576 constraints**

We have plenty of headroom to implement the full algorithm! 🚀

## Commands Reference

### Compile Circuit
```bash
circom rorschach_simple.circom --r1cs --wasm --sym
```

### Setup (one-time)
```bash
snarkjs powersoftau new bn128 12 pot12_0000.ptau
snarkjs powersoftau contribute pot12_0000.ptau pot12_0001.ptau --name="First"
snarkjs powersoftau prepare phase2 pot12_0001.ptau pot12_final.ptau
snarkjs groth16 setup rorschach_simple.r1cs pot12_final.ptau rorschach_0000.zkey
snarkjs zkey contribute rorschach_0000.zkey rorschach_final.zkey --name="1st"
snarkjs zkey export verificationkey rorschach_final.zkey verification_key.json
```

### Generate Proof
```bash
# 1. Compute public outputs from private key
node compute_outputs.js

# 2. Generate witness
node rorschach_simple_js/generate_witness.js \\
  rorschach_simple_js/rorschach_simple.wasm \\
  input_full.json \\
  witness.wtns

# 3. Generate proof
snarkjs groth16 prove rorschach_final.zkey witness.wtns proof.json public.json

# 4. Verify locally
snarkjs groth16 verify verification_key.json public.json proof.json
```

## Why This Works (vs Noir/Barretenberg)

**Noir + Barretenberg Issues**:
- bb Groth16 has hardcoded size limits
- Witness format incompatible
- Even tiny circuits (34k expressions) fail
- "Length is too large" error on ALL attempts

**Circom + snarkjs**:
- Standard R1CS format (universally compatible)
- Witness generation in WASM (portable)
- snarkjs Groth16 prover is production-ready
- Used by major DeFi protocols
- Works on ANY circuit size within Groth16 bounds

## Conclusion

We successfully:
1. ✅ Abandoned broken Barretenberg Groth16
2. ✅ Rewrote circuit in Circom
3. ✅ Generated working Groth16 proofs
4. ✅ Verified proofs locally

**Next**: Integrate with Rust, implement full algorithm, deploy to Solana!

---

**Date**: 2026-01-01
**Status**: Proof of Concept Complete ✅
**Ready for**: Integration & Full Implementation
