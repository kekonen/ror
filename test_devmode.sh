#!/bin/bash
# Test proof generation in dev-mode (fast, no actual ZK proof)
# Dev-mode creates fake receipts for testing logic without full proving

PRIVATE_KEY="${1:-0x1111111111111111111111111111111111111111111111111111111111111111}"
OUTPUT_FILE="${2:-devmode_test.png}"

echo "=== Testing in Dev-Mode (No Real Proof) ==="
echo "Private key: $PRIVATE_KEY"
echo "Output file: $OUTPUT_FILE"
echo ""
echo "⚠️  WARNING: Dev-mode creates FAKE receipts for testing only!"
echo "   These will NOT pass real verification."
echo "   Use this only to test your code logic."
echo ""

# Set dev-mode environment variable
export RISC0_DEV_MODE=1

# Run proof generation (will be fast because it skips real proving)
echo "Running in dev-mode (fast, fake proof)..."
time cargo run --bin ror -- \
    --private-key "$PRIVATE_KEY" \
    --prove \
    --output "$OUTPUT_FILE"

EXIT_CODE=$?

if [ $EXIT_CODE -eq 0 ]; then
    echo ""
    echo "✓ Dev-mode test completed successfully!"
    echo ""
    echo "Note: The 'proof' generated is FAKE and only for testing."
    echo "To generate a real proof, use ./generate_proof_large_stack.sh"
else
    echo ""
    echo "✗ Dev-mode test failed with exit code: $EXIT_CODE"
fi

exit $EXIT_CODE
