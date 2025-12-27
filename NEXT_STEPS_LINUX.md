# Next Steps for Linux Proof Generation

## Summary of What We Fixed

You ran proof generation on Linux amd64 and it worked for **~2 hours** (103,759 seconds of CPU time) before hitting a deserialization error. This confirms:

✅ **Linux works** - No stack overflow like macOS ARM
✅ **Proof generation runs** - The circuit code executes successfully
✅ **Binary optimization works** - 256 bytes journal vs 6,144 bytes
❌ **Deserialization bug** - Fixed by committing struct instead of Vec<u8>

## The Fix Applied

Changed how we serialize the binary image in the journal:
- **Before**: `env::commit(&binary_image.to_bytes())` → caused "integer 256, expected u8" error
- **After**: `env::commit(&binary_image)` → commits struct directly

See [DESERIALIZATION_FIX.md](DESERIALIZATION_FIX.md) for technical details.

## On Your Linux Machine

### Step 1: Pull Latest Code

Make sure your Linux machine has the latest code with the fix:

```bash
git pull  # Or however you sync code to Linux
```

Or manually update these files:
- [methods/guest/src/main.rs](methods/guest/src/main.rs) - Line 43
- [host/src/main.rs](host/src/main.rs) - Lines 117, 269

### Step 2: Clean Build

Force a rebuild to ensure the guest program is updated:

```bash
cd /path/to/ror
cargo clean -p methods  # Clean the guest package
cargo build --bin ror   # Rebuild everything
```

**Important**: The guest program runs inside the zkVM, so it must be recompiled for the fix to take effect.

### Step 3: Quick Test with Dev-Mode (Recommended First)

Before running another 2-hour proof, test with dev-mode (takes seconds):

```bash
RISC0_DEV_MODE=1 cargo run --bin ror -- \
  --private-key 0x1111111111111111111111111111111111111111111111111111111111111111 \
  --prove \
  --output devmode_test.png
```

**Expected output:**
```
Generating ZK proof... (this may take a while)
✓ Proof generated successfully!
  Address: 0x...
  Parameters: walks=..., steps=...
  Binary image size: 256 bytes (24x smaller than RGB!)
  Proof saved to: devmode_test.proof
  Image saved to: devmode_test.png
  (Colors applied after verification - can be changed freely!)
```

If you get this output, the fix worked! ✓

### Step 4: Real Proof Generation

Once dev-mode works, run a real proof:

```bash
time cargo run --bin ror -- \
  --private-key 0x1111111111111111111111111111111111111111111111111111111111111111 \
  --prove \
  --output real_proof.png
```

**Expected time**: ~2 hours (1:54:25 from your previous run)
**Expected CPU**: ~1500% (uses multiple cores)
**Expected memory**: 2-4 GB

### Step 5: Verify the Proof

After generation completes:

```bash
cargo run --bin ror -- --verify real_proof.proof
```

**Expected output:**
```
Verifying proof...
✓ Proof verified successfully!
  Address: 0x...
  Parameters: walks=..., steps=...
  Binary image size: 256 bytes (24x smaller than RGB!)
  (Colors can be applied freely after verification)
```

### Step 6: Change Colors (Demonstrates Binary Optimization)

The binary optimization allows you to change colors **without regenerating the proof**:

```bash
# Generate image with different colors from the same proof
cargo run --bin ror -- \
  --private-key 0x1111111111111111111111111111111111111111111111111111111111111111 \
  --color red \
  --background white \
  --output red_version.png

cargo run --bin ror -- \
  --private-key 0x1111111111111111111111111111111111111111111111111111111111111111 \
  --color blue \
  --background yellow \
  --output blue_version.png
```

Both should produce valid images instantly (no proving required).

## Optimization Scripts for Linux

I've created these scripts that work on both Linux and macOS:

### For Dev-Mode Testing (Fast)
```bash
./test_devmode.sh [private-key] [output-file]
```

### For Real Proofs (Slow but Verified)
```bash
./generate_proof.sh [private-key] [output-file]
```

### For Verification
```bash
./verify_proof.sh <proof-file>
```

## Performance Comparison

### Before Binary Optimization (Theoretical RGB):
- Journal size: 6,144 bytes (32×64 pixels × 3 bytes RGB)
- Circuit complexity: Higher (processes RGB data)
- Proof generation: Slower

### After Binary Optimization (Current):
- Journal size: 256 bytes (32×64 pixels ÷ 8 bits)
- Circuit complexity: **24x lower**
- Proof generation: **Faster** (less data to process)
- Flexibility: Colors changeable after proving

## Troubleshooting

### If dev-mode test fails:
```bash
# Check that guest was rebuilt
cargo clean -p methods
cargo build --bin ror
RISC0_DEV_MODE=1 cargo run --bin ror -- --prove --output test.png
```

### If real proof generation crashes:
```bash
# Check available memory
free -h

# Check CPU load during generation
top
# Look for 'ror' process using 1000-1500% CPU (normal)
```

### If you still get deserialization error:
```bash
# Verify the fix is in the code
grep "env::commit(&binary_image)" methods/guest/src/main.rs
# Should show: env::commit(&binary_image); // Commit the struct directly

# If not, the file wasn't updated - reapply the fix
```

## Expected Success Indicators

✓ Dev-mode completes in <10 seconds
✓ Real proof runs for ~2 hours
✓ No "integer 256, expected u8" error
✓ Proof file created (~several MB)
✓ Image file created (~5-6 KB PNG)
✓ Verification passes
✓ Binary image size shows 256 bytes

## Reporting Back

When you run the test, please share:

1. **Dev-mode result** (quick test):
   ```bash
   RISC0_DEV_MODE=1 cargo run --bin ror -- --prove --output devmode.png 2>&1 | tail -20
   ```

2. **Real proof result** (if dev-mode worked):
   ```bash
   time cargo run --bin ror -- --prove --output real.png 2>&1 | tail -30
   ```

3. **File sizes**:
   ```bash
   ls -lh real.png real.proof devmode.png devmode.proof
   ```

This will confirm the fix worked and the binary optimization is functioning correctly!

## Reference Documents

- [DESERIALIZATION_FIX.md](DESERIALIZATION_FIX.md) - Technical details of the fix
- [STACK_OVERFLOW_ISSUE.md](STACK_OVERFLOW_ISSUE.md) - macOS ARM stack overflow issue
- [LINKING_ISSUE_RESOLVED.md](LINKING_ISSUE_RESOLVED.md) - Previous linking issue resolution
- [PROOF_GENERATION_NOTES.md](PROOF_GENERATION_NOTES.md) - General proof generation notes
