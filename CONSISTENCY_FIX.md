# Consistency Fix: Normal Mode Now Matches Proof Mode

## Problem

When using the same private key, the images generated in different modes were different:

```bash
# Normal mode (before fix)
cargo run --bin ror -- --private-key 0x111...111 --output normal.png
# Generated walks/steps using ChaCha8, image using ChaCha8

# Proof mode
cargo run --bin ror -- --private-key 0x111...111 --prove --output proof.png
# Generated walks/steps using Pedersen (Noir), image using Pedersen (Noir)

# Result: Different images! ❌
```

## Root Cause

Two different RNG algorithms were being used:

| Mode | Parameter Derivation | Image Generation |
|------|---------------------|------------------|
| **Normal** (old) | ChaCha8 RNG | ChaCha8 RNG |
| **Proof** | Pedersen hash (Noir) | Pedersen hash (Noir) |

This caused **different walks/steps values** and **different random walks** for the same private key.

## Solution

Now **both modes use the Noir circuit** as the source of truth:

```rust
// Normal mode (new)
let (walks, steps, binary_image) = if cli.walks.is_none() && cli.steps.is_none() {
    // Use Noir circuit to generate the canonical image
    let outputs = execute_noir_circuit(&private_key, "circuits")?;
    let binary = BinaryImage32x64::from_bytes(&outputs.binary_image);
    (outputs.walks, outputs.steps, binary)
} else {
    // If user manually specifies parameters, use Rust generation
    let walks = cli.walks.unwrap_or_else(|| derive_parameters(&private_key).0);
    let steps = cli.steps.unwrap_or_else(|| derive_parameters(&private_key).1);
    let binary = generate_rorschach_binary(&private_key, walks, steps);
    (walks, steps, binary)
};
```

### Key Changes

1. **Normal mode**: Calls `execute_noir_circuit()` to get walks, steps, and binary_image
2. **Proof mode**: Already calls `execute_noir_circuit()`
3. **Result**: Identical images! ✅

## Verification

```bash
# Generate in normal mode
cargo run --bin ror -- \
  --private-key 0x1111111111111111111111111111111111111111111111111111111111111111 \
  --output normal.png

# Generate in proof mode
cargo run --bin ror -- \
  --private-key 0x1111111111111111111111111111111111111111111111111111111111111111 \
  --prove \
  --output proof.png

# Compare
sha256sum normal.png proof.png
# e9ca7c35f7001822b42ee1569e9999366caa8ad6a7aa0488bb13290af5e33990  normal.png
# e9ca7c35f7001822b42ee1569e9999366caa8ad6a7aa0488bb13290af5e33990  proof.png
# ✅ MATCH!
```

## Behavior

### Default Mode (No Manual Parameters)

Both modes now produce **identical images**:

```bash
# Same image
--output image.png          → Uses Noir circuit
--prove --output image.png  → Uses Noir circuit
```

**For private key** `0x111...111`:
- Walks: 6
- Steps: 268
- Binary image: Same 256 bytes
- Final image: Identical (same SHA256 hash)

### Custom Parameters Mode

If you manually specify walks/steps, it uses Rust's ChaCha8:

```bash
# Custom parameters (uses Rust ChaCha8 RNG)
--walks 10 --steps 150 --output custom.png
```

This is intentional - custom parameters bypass the Noir circuit for faster generation.

## Performance Impact

**Before fix**:
- Normal mode: <1 second (Rust only)
- Proof mode: ~10 seconds (Noir circuit)

**After fix**:
- Normal mode: ~10 seconds (Noir circuit)
- Proof mode: ~10 seconds (Noir circuit)

**Trade-off**: Normal mode is now slower but produces verifiable, consistent images.

**Optimization**: If you want fast generation and don't care about consistency:
```bash
# Fast mode (still uses old ChaCha8 approach)
--walks 10 --steps 200 --output fast.png
```

## Why This Matters

### For Proofs

The whole point of ZK proofs is proving you generated a specific image from a specific private key. If normal mode produces a different image, then:
- ❌ You can't verify a "normal mode" image on-chain
- ❌ The proof doesn't match the visual output

Now:
- ✅ Normal mode image = Proof mode image
- ✅ You can generate visually, then prove later
- ✅ Or prove first, then just apply colors

### For Determinism

Same private key should always produce the same image, regardless of how you generate it:

- ✅ Generate today with `--output`
- ✅ Generate tomorrow with `--prove`
- ✅ Same image (can verify by comparing pixels or hashes)

## Migration Note

**Old behavior** (pre-fix):
- Private key `0x111...111` → ChaCha8 → Different walks/steps/image

**New behavior** (post-fix):
- Private key `0x111...111` → Pedersen (Noir) → walks=6, steps=268

If you generated images before this fix, they used ChaCha8. To regenerate with Noir:

```bash
# Regenerate all images to match Noir circuit
for key in $(cat old_keys.txt); do
  cargo run --bin ror -- --private-key $key --output images/${key}.png
done
```

## Summary

✅ **Fixed**: Normal mode and proof mode now produce identical images
✅ **Verified**: SHA256 hashes match
✅ **Trade-off**: Normal mode is slower but consistent
✅ **Opt-out**: Use custom `--walks/--steps` for fast Rust-only generation

**The Noir circuit is now the single source of truth for image generation!**
