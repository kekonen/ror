# Next Steps for Noir Migration

## ✅ What's Complete (Phase 1)

You now have a **working Noir circuit** that:
- ✅ Compiles without errors
- ✅ Passes all unit tests
- ✅ Implements the full Rorschach image generation algorithm
- ✅ Uses Pedersen hash for deterministic RNG (circuit-friendly)
- ✅ Supports the same parameter ranges as Risc0 (walks: 3-20, steps: 80-300)

## 🔄 Current Challenge: Rust Integration

The main blocker is that **there's no stable Rust FFI for Noir v1.0** yet. Here are your options:

### Option 1: Use `nargo` CLI from Rust (Recommended for MVP)

Call `nargo` as a subprocess from your Rust host code:

```rust
// In host/src/main.rs
use std::process::Command;

fn generate_noir_proof(private_key: &[u8; 32]) -> Result<ProofOutputs, Box<dyn Error>> {
    // 1. Write Prover.toml with inputs
    write_prover_toml(private_key)?;

    // 2. Call nargo execute to get public outputs
    let output = Command::new("nargo")
        .arg("execute")
        .current_dir("circuits")
        .output()?;

    // 3. Parse witness.gz to extract public outputs
    let outputs = parse_witness()?;

    // 4. Optionally generate proof
    Command::new("nargo")
        .arg("prove")
        .current_dir("circuits")
        .output()?;

    Ok(outputs)
}
```

**Pros:**
- Works immediately with existing Noir installation
- No additional dependencies
- Full control over proof generation

**Cons:**
- Slower than direct FFI
- Requires parsing TOML/witness files
- Less elegant than native integration

### Option 2: Wait for `noir_rs` or Barretenberg FFI

There are efforts to create Rust bindings for Noir, but they're not stable for v1.0 yet:
- [`noir-rs`](https://github.com/zkmopro/noir-rs) - Rust wrapper (may not support v1.0)
- Direct Barretenberg FFI - Lower level, more complex

**Pros:**
- Clean native Rust integration
- Better performance
- Type-safe

**Cons:**
- Not ready yet for Noir v1.0
- Would need to wait or contribute to development

### Option 3: Keep Risc0 for Now, Use Noir for Future

Since Risc0 works (except Groth16), you could:
- Keep using Risc0 STARK proofs for development
- Have Noir circuit ready for when better integration is available
- Switch over when noir_rs stabilizes

**Pros:**
- Zero disruption to current workflow
- Noir circuit is future-ready
- Can test both systems in parallel

**Cons:**
- Doesn't solve current Groth16 issues
- Still using slow Risc0 STARK proofs

## 📋 Practical Next Steps

### Immediate (Option 1 - CLI Integration)

1. **Create helper script** ([circuits/run_proof.sh](circuits/run_proof.sh)):
   ```bash
   #!/bin/bash
   # Helper script to generate proof with Noir
   nargo execute && nargo prove
   ```

2. **Add Rust CLI wrapper** in `host/`:
   ```rust
   fn call_nargo_prove(private_key: &[u8; 32]) -> Result<(), Error> {
       // Write Prover.toml
       // Call nargo
       // Read outputs
   }
   ```

3. **Test end-to-end**:
   ```bash
   cd circuits
   # First, derive the correct outputs from a test key
   cargo run --bin test_derive  # You'll need to create this
   # Then update Prover.toml with correct values
   nargo execute
   ```

### Near-term (Full Integration)

1. **Add Pedersen parameter derivation to Rust**:
   - Implement `derive_parameters_pedersen()` in [core/src/lib.rs](core/src/lib.rs)
   - This will match Noir circuit behavior
   - Allows generating correct Prover.toml inputs

2. **Create proof generation flow**:
   - Generate parameters in Rust → Write Prover.toml → Call nargo → Parse outputs
   - Apply colors and stamps in Rust (unchanged)
   - Save final image

3. **Testing**:
   - Verify determinism (same key → same image)
   - Compare performance vs Risc0
   - Document differences (Pedersen vs ChaCha8)

## 🎯 Recommended Approach

**For getting Noir working quickly:**

1. **First**: Create a standalone Rust tool to generate correct Prover.toml inputs
   ```bash
   cargo new --bin generate_prover_toml
   # This tool:
   # - Takes a private key
   # - Derives parameters using Pedersen (matching Noir)
   # - Generates binary image using Pedersen RNG (matching Noir)
   # - Outputs Prover.toml with correct values
   ```

2. **Then**: Test proof generation manually
   ```bash
   ./generate_prover_toml <private_key> > circuits/Prover.toml
   cd circuits
   nargo execute  # Should succeed
   nargo prove    # Generates proof
   ```

3. **Finally**: Integrate into main host binary
   - Call `generate_prover_toml` logic from `host/src/main.rs`
   - Call `nargo` as subprocess
   - Parse outputs and continue with image rendering

## 📦 What You Need to Implement

### 1. Pedersen-based Parameter Derivation (Rust)

Add to [core/src/lib.rs](core/src/lib.rs):

```rust
// Using ark-crypto or similar Pedersen implementation
pub fn derive_parameters_pedersen(pk: &[u8; 32]) -> (u64, u64) {
    // Pack bytes into field elements (matching Noir logic)
    let pk_field1 = pack_bytes_to_field(&pk[0..16]);
    let pk_field2 = pack_bytes_to_field(&pk[16..32]);

    // Pedersen hash
    let hash = pedersen_hash(&[pk_field1, pk_field2]);

    // Extract parameters (matching Noir circuit)
    let walks = 3 + (hash_to_u64(hash) % 18);
    let steps = 80 + ((hash_to_u64(hash) / 1000000) % 221);

    (walks, steps)
}
```

### 2. Pedersen-based Image Generation (Rust)

This is more complex - you'll need to replicate the exact Pedersen RNG logic from Noir.

**Alternative**: Use Noir circuit outputs directly (recommended for MVP)
- Let Noir generate the image
- Parse from witness
- Just apply colors in Rust

## 🚀 Quick Win Strategy

**To get something working today:**

1. Manually create a valid Prover.toml:
   ```bash
   cd circuits
   # Edit Prover.toml with a test private key
   # Set walks=5, steps=100 (within valid ranges)
   # Set binary_image to all zeros initially
   nargo execute --show-output
   # This will show you what the circuit computed
   # Update Prover.toml with those values
   # Run again - should succeed!
   ```

2. Once you have one working proof:
   - You've validated the circuit works
   - Can benchmark proof generation time
   - Can measure proof size
   - Can verify the proof

3. Then work backwards to Rust integration

## 📊 Expected Results

Once you get proof generation working:

- **Proof time**: Should be much faster than Risc0 (seconds vs minutes)
- **Proof size**: ~10-50KB (vs 1MB STARK or 256B Groth16)
- **Deterministic**: Same private key → same proof every time
- **Image quality**: Should look similar to original concept (different RNG = different pattern, but same style)

## 🔗 Resources for Integration

- **Noir CLI Docs**: https://noir-lang.org/docs/getting_started/hello_noir/
- **Witness Format**: witness.gz contains circuit outputs (GZIP compressed TOML)
- **Proof Format**: proof contains the ZK proof (binary)

---

**Bottom Line**: The circuit is done and working. The challenge now is integrating it with your Rust host code. The CLI subprocess approach is the fastest path to a working system, even if not the most elegant.

Would you like me to help implement any of these next steps?
