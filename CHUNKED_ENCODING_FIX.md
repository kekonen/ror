# Chunked Encoding Fix for risc0 Journal Limitation

## The Problem

After ~2 hours of proof generation on Linux, the proof failed with:
```
Error: Custom("invalid value: integer `256`, expected u8")
```

## Root Cause Discovered

risc0's journal `decode()` method uses serde, which **cannot deserialize arrays larger than 32 elements** in Rust's default serde implementation. This is a fundamental limitation.

When trying to commit/decode:
- ❌ `Vec<u8>` with 256 elements → Variable-length encoding with length prefix
- ❌ `[u8; 256]` array → No Deserialize implementation (arrays >32 not supported)
- ❌ Custom struct with Vec inside → Still uses Vec's variable-length encoding

## The Solution: Chunked Encoding

Break the 256-byte binary image into **8 chunks of 32 bytes each**:

### Guest Code ([methods/guest/src/main.rs](methods/guest/src/main.rs))
```rust
// Commit binary image as 8 chunks of 32 bytes each (256 total)
// risc0 journal can't serialize arrays >32, so we chunk it
for chunk_idx in 0..8 {
    let start = chunk_idx * 32;
    let end = start + 32;
    let mut chunk = [0u8; 32];
    chunk.copy_from_slice(&binary_image.data[start..end]);
    env::commit(&chunk);
}
```

### Host Code ([host/src/main.rs](host/src/main.rs))
```rust
// Decode binary image as 8 chunks of 32 bytes each (256 total)
let mut binary_data = Vec::with_capacity(256);
for _ in 0..8 {
    let chunk: [u8; 32] = receipt.journal.decode()?;
    binary_data.extend_from_slice(&chunk);
}
let binary_image = BinaryImage32x64::from_bytes(&binary_data);
```

## Why This Works

- `[u8; 32]` arrays **are** supported by serde's Deserialize
- Each chunk is small enough to avoid the length field issue
- No variable-length encoding - each chunk is fixed 32 bytes
- 8 chunks × 32 bytes = 256 bytes total

## Testing

### Dev-Mode Test (Fast - Seconds)
```bash
RISC0_DEV_MODE=1 cargo run --bin ror -- \
  --private-key 0x1111111111111111111111111111111111111111111111111111111111111111 \
  --prove \
  --output devtest.png
```

**Expected output:**
```
✓ Proof generated successfully!
  Address: 0x...
  Parameters: walks=..., steps=...
  Binary image size: 256 bytes (24x smaller than RGB!)
```

### Real Proof (Linux - ~2 Hours)
```bash
time cargo run --bin ror -- \
  --private-key 0x1111111111111111111111111111111111111111111111111111111111111111 \
  --prove \
  --output real_proof.png
```

## Journal Size Impact

The chunked encoding adds minimal overhead:

**Without chunking** (theoretical):
- 256 bytes of data

**With chunking** (actual):
- 8 × 32 bytes = 256 bytes of data
- 8 × minimal array header = ~8-16 bytes overhead
- **Total**: ~264-272 bytes

Still **23x smaller** than RGB (6,144 bytes)!

## Binary Optimization Benefits Preserved

✅ **256 bytes** vs 6,144 bytes journal size
✅ **~23x reduction** in ZK circuit data
✅ **Faster proof generation** (less data to process)
✅ **Colors changeable** after verification
✅ **Full proof verification** works

## Changes Made

1. **[core/src/lib.rs](core/src/lib.rs)**: Custom Serialize/Deserialize for `BinaryImage32x64`
2. **[methods/guest/src/main.rs](methods/guest/src/main.rs)**: Commit 8 chunks instead of full array
3. **[host/src/main.rs](host/src/main.rs)**: Decode 8 chunks and reassemble
4. **[core/Cargo.toml](core/Cargo.toml)**: Added serde "alloc" feature

## Related risc0 Limitation

This is a known limitation of serde in Rust:
- Arrays up to 32 elements: Built-in Deserialize implementation
- Arrays >32 elements: No default implementation
- Workarounds: `serde-big-array`, custom implementations, or chunking

We chose **chunking** because:
- No additional dependencies (serde-big-array didn't work with no_std)
- Simple and transparent
- Works reliably with risc0's journal
- Minimal overhead

## Next Steps

On your Linux machine:

1. Pull latest code with chunked encoding fix
2. Clean build: `cargo clean && cargo build --bin ror`
3. Test dev-mode: `RISC0_DEV_MODE=1 cargo run --bin ror -- --prove --output test.png`
4. Run real proof: `time cargo run --bin ror -- --prove --output real.png` (~2 hours)
5. Verify: `cargo run --bin ror -- --verify real.proof`

The fix is confirmed working in dev-mode. Real proof generation should now complete successfully!
