# Stack Overflow Issue with risc0 v3.0 on macOS ARM

## Problem

Proof generation crashes with a **bus error** due to stack overflow in the risc0 C++ circuit code:

```
[1]    27958 bus error  target/debug/ror --private-key  --prove --output kek
```

### Root Cause

The crash report (`~/Library/Logs/DiagnosticReports/ror-*.ips`) shows:

```json
"exception": {
  "type": "EXC_BAD_ACCESS",
  "signal": "SIGBUS",
  "subtype": "KERN_PROTECTION_FAILURE at 0x000000016fcdfff8",
  "message": "Could not determine thread index for stack guard region"
}

"vmRegionInfo": "0x16fcdfff8 is in 0x16fcdc000-0x16fce0000;
      Stack                       16fc54000-16fcdc000    [  544K] rw-/rwx SM=PRV
--->  Stack Guard                 16fcdc000-16fce0000    [   16K] ---/rwx SM=NUL
      Stack                       16fce0000-16fcfc000    [  112K] rw-/rwx SM=ZER"
```

**Translation**: The worker thread crashed when its stack hit the protected "Stack Guard" region. This guard is specifically designed to catch stack overflows.

### Call Stack at Crash

The crash occurs deep in the C++ circuit execution:

```
exec_TopExtract
  → execUser_Accum
    → exec_TopAccum
      → step_TopAccum
        → stepAccum
          → risc0_circuit_rv32im_cpu_accum (CRASH HERE)
```

The risc0 v3.0 C++ circuit implementation for RV32IM uses deep recursion or large stack allocations during the accumulation phase.

## Solutions

### Option 1: Increase Stack Size (Recommended for Real Proofs)

Use the `generate_proof_large_stack.sh` script which sets stack size to maximum (64MB):

```bash
./generate_proof_large_stack.sh [private-key] [output-file]
```

**How it works:**
```bash
ulimit -s 65520  # Set stack to 65520 KB (64MB, maximum on macOS)
cargo run --bin ror -- --private-key ... --prove --output ...
```

**Limitations:**
- macOS ARM default stack: 8,176 KB (~8MB)
- macOS ARM hard limit: 65,520 KB (~64MB)
- Cannot exceed hard limit without changing system settings

**Note**: Even with maximum stack size (64MB), the proof generation may still crash if the circuit complexity is too high. This is a known limitation of risc0 v3.0's C++ circuit implementation on macOS ARM.

### Option 2: Dev-Mode (Fast Testing Only)

Use `RISC0_DEV_MODE=1` to create fake proofs for testing your code logic:

```bash
./test_devmode.sh [private-key] [output-file]
```

Or manually:
```bash
RISC0_DEV_MODE=1 cargo run --bin ror -- --private-key ... --prove --output ...
```

**Pros:**
- Very fast (seconds instead of minutes)
- Tests all your code logic
- No stack overflow issues

**Cons:**
- Creates FAKE receipts
- Will NOT pass real verification
- Only for development/testing

**Security Warning**: Never use dev-mode receipts in production! They bypass all cryptographic proving.

### Option 3: Alternative Prover Backends

If stack overflow persists even with max stack size, consider:

1. **Bonsai** (Remote proving service):
   ```bash
   # Enable bonsai feature
   cargo run --bin ror --features bonsai -- --prove ...
   ```
   - Requires Bonsai API key
   - Proves on remote servers (no local stack issues)
   - May incur costs

2. **Docker** (Linux environment):
   - Run proof generation in Docker Linux container
   - Linux may have different stack behavior
   - More consistent across platforms

3. **Downgrade to risc0 v1.x**:
   - Older versions may have different stack usage patterns
   - Less optimized but more stable on macOS ARM

## Technical Details

### Why This Happens

risc0 v3.0 introduced significant changes to the circuit implementation:
- Uses C++ code compiled via build scripts
- Heavy use of poolstl for parallelization
- Deep function call stacks in `exec_TopAccum` and related functions
- Large local variables and buffers on the stack

macOS ARM (Apple Silicon) has stricter stack limits than Linux:
- Default: 8MB per thread
- Hard limit: 64MB per thread
- Stack guards catch overflows early

### Binary Optimization Impact

Our binary image optimization (256 bytes vs 6,144 bytes) **helps** but doesn't prevent stack overflow because:
- Stack overflow happens in the **prover circuit code**, not in handling the journal
- The circuit still needs to verify the same computation regardless of journal size
- The benefit is in proof generation speed and journal transmission, not stack usage

## Monitoring and Debugging

### Check Current Stack Size

```bash
ulimit -s        # Soft limit (current)
ulimit -Hs       # Hard limit (maximum)
```

### View Crash Reports

```bash
ls -lt ~/Library/Logs/DiagnosticReports/ror-*.ips | head -5
```

### Check Process Stats During Proof Generation

```bash
ps aux | grep "[r]or --private-key"
# Look for high CPU % (normal) and memory usage
```

### Expected Behavior During Proof Generation

- **CPU**: ~90-100% on one core (normal)
- **Memory**: 2-4 GB (normal for risc0)
- **Time**: 5-20 minutes depending on parameters
- **Threads**: Multiple worker threads (poolstl parallelization)

## Related Issues

- [RISC Zero dev-mode documentation](https://dev.risczero.com/api/generating-proofs/dev-mode)
- [Bus error on macOS](https://discussions.apple.com/thread/8294966)
- Stack overflow without diagnostic error ([example issue](https://github.com/dotnet/runtime/issues/66302))

## Recommendations

For development and testing:
1. Use `test_devmode.sh` for quick iterations
2. Verify your code logic works correctly
3. Test with small parameter values (walks, steps)

For production proof generation:
1. Use `generate_proof_large_stack.sh` with maximum stack
2. If it still crashes, consider Bonsai or Docker
3. Monitor system resources during generation
4. Consider generating proofs on Linux servers instead of macOS

## Future Improvements

Potential solutions being explored:
1. Report stack overflow issue to risc0 project
2. Request optimization of C++ circuit code for macOS ARM
3. Investigate if Metal GPU backend affects stack usage
4. Test risc0 v4.0 when released (may have improvements)
