#!/bin/bash
# Proof generation script with increased stack size for risc0 v3.0
# The C++ circuit code needs more stack space than the default 8MB

PRIVATE_KEY="${1:-0x1111111111111111111111111111111111111111111111111111111111111111}"
OUTPUT_FILE="${2:-proof_test.png}"

echo "=== Generating ZK Proof with Large Stack ===" echo "Private key: $PRIVATE_KEY"
echo "Output file: $OUTPUT_FILE"
echo "Proof will be saved to: ${OUTPUT_FILE%.png}.proof"
echo ""

# Increase stack size to maximum (64MB) to prevent stack overflow in C++ circuit code
echo "Setting stack size to maximum (65520 KB)..."
ulimit -s 65520
ACTUAL_STACK=$(ulimit -s)
echo "Current stack size: ${ACTUAL_STACK} KB"
echo ""

echo "This will take 5-15 minutes depending on your CPU..."
echo "Press Ctrl+C to cancel"
echo ""
echo "Starting proof generation at $(date)"
echo "---"

# Source risc0 environment if available
if [ -f "$HOME/.risc0/env" ]; then
    source "$HOME/.risc0/env"
fi

# Use debug mode
time cargo run --bin ror -- \
    --private-key "$PRIVATE_KEY" \
    --prove \
    --output "$OUTPUT_FILE"

EXIT_CODE=$?

echo "---"
echo "Finished at $(date)"

if [ $EXIT_CODE -eq 0 ]; then
    echo ""
    echo "✓ Proof generation completed successfully!"
    echo ""

    if [ -f "${OUTPUT_FILE%.png}.proof" ]; then
        echo "Proof file created:"
        ls -lh "${OUTPUT_FILE%.png}.proof"
        echo ""
        echo "To verify the proof, run:"
        echo "  ./verify_proof.sh ${OUTPUT_FILE%.png}.proof"
    else
        echo "Warning: Proof file not found at expected location"
    fi

    if [ -f "$OUTPUT_FILE" ]; then
        echo ""
        echo "Image file created:"
        ls -lh "$OUTPUT_FILE"
    fi
else
    echo ""
    echo "✗ Proof generation failed with exit code: $EXIT_CODE"
    echo ""
    if grep -q "bus error" <<< "$EXIT_CODE"; then
        echo "If you see 'bus error', the C++ circuit code may need more stack."
        echo "This script already uses maximum stack size (65520 KB)."
        echo "Consider using dev-mode for testing: RISC0_DEV_MODE=1"
    fi
    echo ""
    echo "If you see linking errors (undefined symbols), try:"
    echo "  cargo clean -p ror && ./generate_proof_large_stack.sh"
fi

exit $EXIT_CODE
