# Phase 2 Status: Rust Integration with Noir

## ✅ What's Complete

1. **Pedersen-based parameter derivation in Rust** ([core/src/lib.rs:207-235](core/src/lib.rs#L207-L235))
   - Function: `derive_parameters_pedersen()`
   - Uses arkworks crypto library (ark-crypto-primitives)
   - Successfully compiles and runs
   - Produces values in correct ranges (walks: 3-20, steps: 80-300)
   - Is deterministic (same key → same output)

2. **Test tool** ([test_pedersen.rs](test_pedersen.rs))
   - Verifies Pedersen derivation works
   - Tests determinism
   - For test key `[0x12, 0x34, ...]`: walks=3, steps=154

## ⚠️ Current Blocker

**Problem**: Rust Pedersen implementation doesn't match Noir's `std::hash::pedersen_hash`

- **Noir circuit** ([circuits/src/main.nr:66](circuits/src/main.nr#L66)) uses `std::hash::pedersen_hash(pk_fields)`
- **Rust code** uses arkworks' `PedersenHash::evaluate()`
- These produce **different hash values** for the same input
- Result: Circuit assertion fails because `derived_walks != walks`

**Root cause**: Different Pedersen hash implementations use different curve parameters:
- Noir uses Grumpkin curve (embedded in BN254)
- Arkworks defaults may differ

## 🎯 Solution Paths

### Option A: Match Implementations (Complex, Not Recommended)
Research exact Pedersen parameters Noir uses and configure arkworks to match. This is technically possible but time-consuming.

### Option B: Let Noir Do Everything (Recommended for MVP)
Use Noir for all ZK computation, Rust only for post-processing:

```
Private Key
     ↓
Noir Circuit (via nargo)
     ↓
Witness file (contains walks, steps, binary_image)
     ↓
Parse witness in Rust
     ↓
Apply colors & stamps in Rust
     ↓
Final image + proof
```

**Benefits**:
- No need to match implementations
- Noir circuit remains the source of truth
- Rust handles only non-ZK tasks (colors, stamps, file I/O)
- Aligns with NEXT_STEPS.md recommendation (lines 189-193)

## 📋 Next Steps (Option B)

### 1. Understand Noir Witness Format
When you run `nargo execute`, it generates `target/witness.gz`:
- Contains all circuit outputs (walks, steps, binary_image)
- Format: GZIP-compressed TOML or binary
- Need to parse this in Rust

### 2. Create Noir Execution Wrapper
Rust function that:
```rust
fn execute_noir_circuit(private_key: &[u8; 32]) -> Result<CircuitOutputs, Error> {
    // 1. Write minimal Prover.toml with just private_key
    write_prover_toml(private_key)?;

    // 2. Call nargo execute
    Command::new("nargo")
        .arg("execute")
        .current_dir("circuits")
        .output()?;

    // 3. Parse witness.gz
    let outputs = parse_witness("circuits/target/witness.gz")?;

    Ok(outputs)
}
```

### 3. Modify Circuit Design (Optional)
Current circuit design requires walks/steps/binary_image as public inputs that must match computed values. This creates the chicken-and-egg problem.

**Better design for MVP**:
```noir
fn main(private_key: [u8; 32]) -> pub (u64, u64, [u8; 256]) {
    let (walks, steps) = derive_parameters_pedersen(private_key);
    let binary_image = generate_binary_image(private_key, walks, steps);
    (walks, steps, binary_image)
}
```

This returns computed values as outputs rather than requiring them as matching inputs.

### 4. Alternative: Accept the Mismatch
For development/testing, we could:
- Use Rust's Pedersen to generate test images (different from Noir)
- Use Noir's Pedersen for actual proofs
- Document that they produce different images (both valid, just different)

This is acceptable if the goal is just to prove "I know a private key that generates THIS image" rather than "images must match between Rust and Noir."

## 🔧 Recommended Immediate Actions

1. **Simplify Prover.toml** - Only require private_key as input
2. **Modify circuit** - Return computed values instead of asserting against inputs
3. **Create witness parser** - Rust tool to read Noir's witness file
4. **Build nargo wrapper** - Rust function to call nargo and parse results
5. **Test end-to-end** - private_key → nargo → witness → Rust → final image

## 📝 Code Changes Needed

### circuits/src/main.nr
```noir
// Simplified design - return outputs instead of asserting
fn main(
    private_key: [u8; 32],
    ethereum_address: pub [u8; 20],  // Still for future use
) -> pub (u64, u64, [u8; 256]) {
    let _ = ethereum_address;  // Not verified yet (MVP)

    let (walks, steps) = derive_parameters_poseidon(private_key);
    let binary_image = generate_binary_image(private_key, walks, steps);

    (walks, steps, binary_image)
}
```

### circuits/Prover.toml
```toml
# Minimal - only private input needed
private_key = [
    "0x12", "0x34", "0x56", "0x78", "0x90", "0xab", "0xcd", "0xef",
    # ... 32 bytes total
]

# No need for ethereum_address yet (MVP)
# No need for walks/steps/binary_image - circuit computes these
```

### host/src/main.rs (new function)
```rust
use std::process::Command;
use std::fs;

fn generate_with_noir(private_key: &[u8; 32]) -> Result<ProofOutputs, Error> {
    // Write Prover.toml
    write_prover_toml_minimal(private_key)?;

    // Execute circuit
    let output = Command::new("nargo")
        .arg("execute")
        .arg("--show-output")  // Shows return values
        .current_dir("circuits")
        .output()?;

    // Parse output (nargo prints return values to stdout)
    let stdout = String::from_utf8(output.stdout)?;
    let (walks, steps, binary_image) = parse_nargo_output(&stdout)?;

    // Optionally generate proof
    Command::new("nargo")
        .arg("prove")
        .current_dir("circuits")
        .output()?;

    Ok(ProofOutputs {
        address: [0; 20],  // TODO: Derive off-circuit
        walks,
        steps,
        binary_chunks: binary_image,  // Convert format
    })
}
```

## 🎓 Key Insights

1. **ZK circuits are the source of truth** - Rust should call Noir, not replicate it
2. **Matching crypto implementations is hard** - Different libraries, different parameters
3. **MVP philosophy** - Get it working with CLI calls, optimize later with FFI
4. **Circuit design matters** - Return values vs. assertion-based design affects usability

## 📊 Trade-offs

| Approach | Pros | Cons |
|----------|------|------|
| **Match implementations** | Rust can pre-compute values | Complex, time-consuming |
| **Let Noir do everything** | Simple, reliable, fast to implement | Requires calling nargo CLI |
| **Accept mismatch** | Can develop independently | Confusing to have two different RNGs |

**Recommendation**: Let Noir do everything (Option B) for MVP, consider matching implementations later if needed for performance or UX.

---

**Status**: Ready to implement Option B - modify circuit to return values, build nargo wrapper
**Blocked on**: Circuit design change (return values instead of assertions)
**Next step**: Update circuits/src/main.nr to return computed values
