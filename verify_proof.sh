#!/bin/bash
# Verification script for Rorschach ZK proofs

PROOF_FILE="$1"

if [ -z "$PROOF_FILE" ]; then
    echo "Usage: ./verify_proof.sh <proof_file>"
    echo ""
    echo "Looking for .proof files in current directory..."
    ls -lht *.proof 2>/dev/null || echo "No .proof files found"
    exit 1
fi

if [ ! -f "$PROOF_FILE" ]; then
    echo "Error: Proof file '$PROOF_FILE' not found"
    exit 1
fi

echo "=== Verifying ZK Proof ==="
echo "Proof file: $PROOF_FILE"
echo "File size: $(ls -lh "$PROOF_FILE" | awk '{print $5}')"
echo ""

cargo run --bin ror -- --verify "$PROOF_FILE"
