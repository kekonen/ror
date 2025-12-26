# Proof Generation Notes

## Current Issue (Under Investigation)

The proof generation on macOS ARM (Apple Silicon) is experiencing intermittent linking errors with risc0 v3.0 CPU circuit symbols:

```
Undefined symbols for architecture arm64:
  "_risc0_circuit_keccak_cpu_poly_fp"
  "_risc0_circuit_keccak_cpu_witgen"
  "_risc0_circuit_recursion_cpu_accum"
  "_risc0_circuit_recursion_cpu_eval_check"
  "_risc0_circuit_recursion_cpu_witgen"
  "_risc0_circuit_rv32im_cpu_accum"
  "_risc0_circuit_rv32im_cpu_poly_fp"
  "_risc0_circuit_rv32im_cpu_witgen"
ld: symbol(s) not found for architecture arm64
```

### Root Cause Analysis ✓

**RESOLVED**: The linking errors were caused by stale build artifacts. The issue is **intermittent** and can be fixed by rebuilding.

**Key Finding**: All required symbols exist in the compiled libraries:
- Verified `_risc0_circuit_keccak_cpu_{poly_fp,witgen}` ✓
- Verified `_risc0_circuit_recursion_cpu_{accum,eval_check,witgen}` ✓
- Verified `_risc0_circuit_rv32im_cpu_{accum,poly_fp,witgen}` ✓

These symbols are present in:
- `target/debug/build/risc0-circuit-keccak-sys-*/out/librisc0_keccak_cpu.a`
- `target/debug/build/risc0-circuit-recursion-sys-*/out/librisc0_recursion_cpu.a`
- `target/debug/build/risc0-circuit-rv32im-sys-*/out/librisc0_rv32im_cpu.a`

### Solution

If you encounter linking errors:

1. **Clean and rebuild**: `cargo clean -p ror && cargo build --bin ror`
2. **Or full clean**: `cargo clean && cargo build`
3. The C++ kernel libraries will be rebuilt with correct symbols

### Why This Happens

The risc0 circuit libraries build C++ kernels via build scripts. Sometimes cargo's incremental compilation can leave stale artifacts that don't properly link. A clean rebuild resolves this.

### System Requirements Verified ✓

- Architecture: arm64 (Apple Silicon) ✓
- Toolchain: stable-aarch64-apple-darwin ✓
- risc0 v3.0.4 installed via rzup ✓
- No iconv conflicts (MacPorts not present) ✓
- Circuit libraries building correctly ✓

## Performance Impact

- **Debug mode**: Proof generation will take longer (5-20 minutes)
- **Release mode**: Would be faster but currently has linking issues

## To Run Proof Generation

Simply run:
```bash
./generate_proof.sh
```

Or with custom parameters:
```bash
./generate_proof.sh 0xYOUR_PRIVATE_KEY output.png
```

## Expected Output

When successful, you should see:
1. Compilation messages
2. "Generating ZK proof... (this may take a while)"
3. Progress updates (if any)
4. "✓ Proof generated successfully!"
5. Binary image size: **256 bytes** (24x smaller than RGB!)
6. Proof file location

## Verification

After proof generation completes:
```bash
./verify_proof.sh proof_test.proof
```

This will verify the cryptographic proof and show:
- Ethereum address (derived from private key)
- Walk/step parameters
- Binary image size (256 bytes)
