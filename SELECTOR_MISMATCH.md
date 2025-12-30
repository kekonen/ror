# Groth16 Selector Mismatch Issue

## Problem

On-chain Groth16 proof verification fails with `SelectorMismatch` error.

**Our proof selector**: `0x2f80f8d0`
**risc0-ethereum v3.0.1 expects**: `0x73c457ba`

## Root Cause

The Docker image `risczero/risc0-groth16-prover:v2025-04-03.1` used by risc0-groth16 v3.0.3 has a **different Groth16 verification key** than the published risc0-ethereum v3.0.1 contracts.

The selector is computed from: `hash(CONTROL_ROOT + BN254_CONTROL_ID + Groth16_VK)`

We matched the versions perfectly:
- ✅ risc0-zkvm v3.0.3
- ✅ risc0-ethereum v3.0.1 (which uses risc0-zkvm v3.0.3)

But the Docker prover is hardcoded in the risc0-groth16 crate to use a Docker image with a mismatched verification key.

## What Works

- STARK proof generation ✅
- Groth16 proof generation ✅
- Journal encoding/decoding ✅
- All Solidity contract logic ✅
- Tests with mock data ✅

## What Doesn't Work

- On-chain cryptographic verification of real Groth16 proofs ❌

## Solutions

1. **Wait for risc0 fix**: The risc0 team needs to release a risc0-ethereum version matching the Docker prover image
2. **Use Bonsai**: The cloud proving service should have synchronized keys
3. **Custom verifier**: Extract VK from Docker image and deploy custom contracts (complex)

## Current Status

All infrastructure is ready. Waiting for risc0 ecosystem version alignment.

**File**: `~/.cargo/registry/src/.../risc0-groth16-3.0.3/src/prove/docker.rs:193`
```rust
.arg("risczero/risc0-groth16-prover:v2025-04-03.1")  // Hardcoded, no override
```
