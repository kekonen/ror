# Noir Migration Status

## ✅ Completed (Phase 1)

### 1. Setup & Infrastructure
- ✅ Noir toolchain installed (nargo v1.0.0-beta.17)
- ✅ circuits/ project initialized
- ✅ Nargo.toml configured
- ✅ Workspace Cargo.toml updated (removed `methods`)

### 2. Circuit Implementation
- ✅ Noir circuit implemented at [circuits/src/main.nr](circuits/src/main.nr)
- ✅ **Pedersen hash** used for RNG (replaces ChaCha8)
- ✅ Parameter derivation (`derive_parameters_poseidon`)
- ✅ Binary image generation (`generate_binary_image`)
- ✅ Random walk algorithm ported
- ✅ Direction decision logic ported
- ✅ Binary pixel packing logic ported

### 3. Testing
- ✅ Circuit compiles without errors (`nargo check`)
- ✅ Unit tests pass:
  - `test_derive_parameters` - deterministic parameter derivation
  - `test_pixel_setting` - binary pixel manipulation

## 🔄 In Progress / Next Steps

### Phase 2: Rust Integration

1. **Add Pedersen-based parameter derivation to Rust** (for comparison)
   - Create `derive_parameters_pedersen()` in [core/src/lib.rs](core/src/lib.rs)
   - This will match the Noir circuit's RNG behavior

2. **Create Noir proving wrapper**
   - Option A: Use `nargo` CLI directly from Rust
   - Option B: Wait for stable Rust FFI bindings to Noir/Barretenberg

3. **Update host/src/main.rs**
   - Remove Risc0 proving code
   - Add Noir proof generation flow
   - Keep all image rendering/stamping logic in Rust

### Phase 3: Testing & Validation

1. **End-to-end test**
   - Generate proof with test private key
   - Verify determinism (same key → same image)
   - Compare outputs with Risc0 version

2. **Performance benchmarking**
   - Measure proof generation time
   - Measure proof size
   - Compare to Risc0 baseline

### Phase 4: On-Chain Verification (Future)

1. Generate Solidity verifier (when Barretenberg backend ready)
2. Deploy and test on-chain verification
3. Measure gas costs

## 📊 Key Differences from Risc0

### RNG Change: ChaCha8 → Pedersen Hash

| Aspect | Risc0 (ChaCha8) | Noir (Pedersen) |
|--------|-----------------|-----------------|
| **Algorithm** | ChaCha8 stream cipher | Pedersen hash function |
| **Purpose** | CSPRNG for parameter derivation | Deterministic hash for ZK circuits |
| **Output** | Pseudo-random bytes | Field elements |
| **Performance** | Fast in software | Optimized for ZK circuits |
| **Result** | Different walks/steps values | Different walks/steps values |

**Impact**: Images will look different but still be deterministic. Same private key → same image (within Noir), but different from Risc0 images.

### Ethereum Address Verification

| Aspect | Risc0 | Noir (MVP) |
|--------|-------|------------|
| **secp256k1 derivation** | ✅ In-circuit | ⚠️ Skipped for MVP |
| **Keccak256 address** | ✅ In-circuit | ⚠️ Skipped for MVP |
| **Verification** | Proven | Public input (host verifies) |

**MVP Approach**: For simplicity, the Ethereum address is passed as a public output. The host (off-circuit) verifies it matches the private key using standard secp256k1 libraries. This proves knowledge of the private key that generated the image, without the expensive in-circuit secp256k1 derivation.

**Future Enhancement**: Full secp256k1 public key derivation can be added using the [ecrecover-noir library](https://github.com/colinnielsen/ecrecover-noir).

## 🔧 Technical Details

### Circuit Bounds

Due to Noir's requirement for compile-time loop bounds:
- `MAX_WALKS = 20` (runtime walks must be ≤ 20)
- `MAX_STEPS = 300` (runtime steps must be ≤ 300)

These match the original Risc0 ranges:
- walks: 3-20
- steps: 80-300

### Type Conversions

Noir v1.0 has strict type requirements:
- Array indexing must use `u32` (not `u64`)
- Bit operations must have matching types
- Field arithmetic uses `Field` type

### No `continue` in Constrained Code

Noir doesn't allow `continue` in constrained functions (circuits). Workaround:
```noir
for i in 0..MAX_WALKS {
    let should_execute = i < actual_walks;
    if should_execute {
        // ... execute walk logic
    }
}
```

## 📁 File Changes

### Created
- `circuits/` - Noir project
- `circuits/Nargo.toml` - Noir configuration
- `circuits/src/main.nr` - Circuit implementation
- `NOIR_MIGRATION_STATUS.md` - This document

### Modified
- `Cargo.toml` - Removed `methods` from workspace

### To Delete (after full migration)
- `methods/` - Old Risc0 guest code

## 🎯 Success Criteria

### MVP Goals
- [x] Circuit compiles
- [x] Unit tests pass
- [ ] Generate proof with nargo
- [ ] Integrate with Rust host
- [ ] End-to-end test (private key → proof → image)

### Full Migration Goals
- [ ] Proof generation < 10s (target: 1-3s)
- [ ] Proof size < 100KB (target: 10-50KB)
- [ ] Deterministic (same key → same output)
- [ ] Images visually similar to original concept
- [ ] On-chain verification working
- [ ] Gas costs measured

## 🔗 Resources

- [Noir Documentation](https://noir-lang.org/docs/)
- [Noir Standard Library - Hashes](https://noir-lang.org/docs/noir/standard_library/cryptographic_primitives/hashes)
- [Noir Standard Library - Embedded Curve Ops](https://noir-lang.org/docs/dev/noir/standard_library/cryptographic_primitives/embedded_curve_ops)
- [ecrecover-noir (secp256k1 library)](https://github.com/colinnielsen/ecrecover-noir)
- [Migration Plan](/home/daniil/.claude/plans/rippling-seeking-lerdorf.md)

## 🚀 Next Commands

```bash
# Compile circuit
cd circuits
nargo check

# Run tests
nargo test

# Generate proof (requires valid Prover.toml inputs)
nargo prove

# Verify proof
nargo verify

# Generate Solidity verifier (future)
# bb contract -b ./target/circuits.json -o ../contracts/RorVerifier.sol
```

---

**Last Updated**: 2025-12-30
**Status**: Phase 1 Complete ✅ | Phase 2 In Progress 🔄
