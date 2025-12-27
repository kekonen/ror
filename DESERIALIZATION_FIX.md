# Deserialization Error Fix

## The Problem

After ~2 hours of proof generation on Linux amd64, the proof failed with:

```
Error: Custom("invalid value: integer `256`, expected u8")
```

## Root Cause

The error occurred because we were serializing the binary image incorrectly:

**Before (WRONG):**
```rust
// Guest code
env::commit(&binary_image.to_bytes()); // Commits Vec<u8>

// Host code
let binary_bytes: Vec<u8> = receipt.journal.decode()?;
```

When you commit a `Vec<u8>` with 256 elements, risc0's serialization format includes:
1. **Length prefix** (the number 256)
2. **Data** (256 bytes)

The problem is that some part of the deserialization expected the length to fit in a `u8` (max 255), but our length was 256, causing the error.

## The Fix

Commit the **struct directly** instead of converting to Vec<u8>:

**After (CORRECT):**
```rust
// Guest code (methods/guest/src/main.rs line 43)
env::commit(&binary_image); // Commit BinaryImage32x64 struct directly

// Host code (host/src/main.rs line 269)
let binary_image: BinaryImage32x64 = receipt.journal.decode()?;
```

This way, the serialization uses the struct's `Serialize`/`Deserialize` implementation, which properly handles the 256-byte Vec inside.

## Changes Made

### 1. Guest Program ([methods/guest/src/main.rs](methods/guest/src/main.rs))
```rust
// Line 43 - Changed from:
env::commit(&binary_image.to_bytes());
// To:
env::commit(&binary_image);
```

### 2. Host Program - Proof Generation ([host/src/main.rs](host/src/main.rs))
```rust
// Line 269 - Changed from:
let binary_bytes: Vec<u8> = receipt.journal.decode()?;
let binary_image = BinaryImage32x64::from_bytes(&binary_bytes);
// To:
let binary_image: BinaryImage32x64 = receipt.journal.decode()?;
```

### 3. Host Program - Proof Verification ([host/src/main.rs](host/src/main.rs))
```rust
// Line 117 - Changed from:
let binary_bytes: Vec<u8> = receipt.journal.decode()?;
// To:
let binary_image: BinaryImage32x64 = receipt.journal.decode()?;
```

## Testing the Fix

### On Linux (Where you have a working environment):

1. **Rebuild everything** to ensure the guest program is updated:
   ```bash
   cargo clean -p methods
   cargo build --bin ror
   ```

2. **Run proof generation** (this will take ~2 hours again):
   ```bash
   time cargo run --bin ror -- \
     --private-key 0x1111111111111111111111111111111111111111111111111111111111111111 \
     --prove \
     --output proof_test.png
   ```

3. **Verify the proof** (if generation succeeds):
   ```bash
   cargo run --bin ror -- --verify proof_test.proof
   ```

### Quick Test with Dev-Mode (Fast):

Test the fix quickly without waiting 2 hours:

```bash
RISC0_DEV_MODE=1 cargo run --bin ror -- \
  --private-key 0x1111111111111111111111111111111111111111111111111111111111111111 \
  --prove \
  --output devmode_test.png
```

This should complete in seconds and confirm the deserialization works.

## Why This Happened

The `BinaryImage32x64` struct contains:
```rust
pub struct BinaryImage32x64 {
    pub data: Vec<u8>,  // 256 bytes
}
```

When we called `binary_image.to_bytes()`, we returned just the inner `Vec<u8>`. When risc0 serializes a top-level `Vec<u8>`, it uses a compact encoding that may have different length field sizes depending on the length.

By committing the struct directly, risc0 uses the `Serialize` implementation for `BinaryImage32x64`, which properly handles the Vec as a field of the struct.

## Expected Output After Fix

```
Generating ZK proof... (this may take a while)
✓ Proof generated successfully!
  Address: 0x...
  Parameters: walks=..., steps=...
  Binary image size: 256 bytes (24x smaller than RGB!)
  Proof saved to: proof_test.proof
  Image saved to: proof_test.png
  (Colors applied after verification - can be changed freely!)
```

## Impact on Binary Optimization

This fix does **not** change the optimization benefits:
- ✓ Binary image is still 256 bytes (not 6,144 bytes)
- ✓ 24x reduction in journal size
- ✓ Faster proof generation (less data in circuit)
- ✓ Colors still applied after verification

The only change is how we serialize/deserialize the binary image in the journal.
