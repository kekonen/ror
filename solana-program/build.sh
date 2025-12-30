#!/bin/bash
# Build script for Rorschach Solana program

set -e

echo "=== Building Rorschach Solana Program ==="
echo

# Build the program
echo "Step 1/2: Building Solana program..."
cd program
cargo build-sbf --manifest-path=Cargo.toml
echo "✓ Program built"
echo

# Build the client
echo "Step 2/2: Building client..."
cd ../client
cargo build --release
echo "✓ Client built"
echo

echo "=== Build Complete ==="
echo
echo "Program binary: solana-program/target/deploy/rorschach_solana.so"
echo "Client binary: solana-program/target/release/rorschach-client"
echo
echo "Next steps:"
echo "  1. Start local validator: solana-test-validator"
echo "  2. Deploy program: solana program deploy target/deploy/rorschach_solana.so"
echo "  3. Use client to interact: ./target/release/rorschach-client --help"
