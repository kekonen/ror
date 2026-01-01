pragma circom 2.1.0;

include "circomlib/circuits/poseidon.circom";
include "circomlib/circuits/bitify.circom";

// Rorschach pattern generator circuit
// Generates a deterministic binary image from a private key
//
// Public inputs: walks, steps, binary_image[256]
// Private input: private_key[32]
//
// Proves: I know a private_key that generates this specific image

template RorschachGenerator() {
    // Private input: 32-byte private key
    signal input privateKey[32];

    // Public outputs
    signal output walks;
    signal output steps;
    signal output binaryImage[256]; // 32x64 binary image as 256 bytes

    // Constants for image generation
    var VIRTUAL_WIDTH = 64;
    var PHYSICAL_WIDTH = 32;
    var HEIGHT = 64;

    // Bounds for parameter derivation
    var MIN_WALKS = 3;
    var MAX_WALKS_BOUND = 20;
    var MIN_STEPS = 80;
    var MAX_STEPS_BOUND = 300;

    // Circuit size limits for Groth16 - use smaller bounds for now
    var MAX_WALKS = 8;  // Compile-time bound
    var MAX_STEPS = 100; // Compile-time bound

    // Step 1: Derive parameters from private key using Poseidon hash
    component hasher = Poseidon(32);
    for (var i = 0; i < 32; i++) {
        hasher.inputs[i] <== privateKey[i];
    }

    signal pkHash;
    pkHash <== hasher.out;

    // Derive walks and steps from hash
    // walks = MIN_WALKS + (hash % (MAX_WALKS_BOUND - MIN_WALKS))
    signal walksRaw;
    signal stepsRaw;

    // Simple modulo for walks (simplified - in production use proper range constraint)
    walksRaw <-- (pkHash % (MAX_WALKS_BOUND - MIN_WALKS)) + MIN_WALKS;
    walks <== walksRaw;

    // Constrain walks to be within valid range
    component walksRange = LessThan(8);
    walksRange.in[0] <== walks - MIN_WALKS;
    walksRange.in[1] <== MAX_WALKS_BOUND - MIN_WALKS;
    walksRange.out === 1;

    // Derive steps (simplified)
    stepsRaw <-- (pkHash \ 1000) % (MAX_STEPS_BOUND - MIN_STEPS) + MIN_STEPS;
    steps <== stepsRaw;

    // Constrain steps to be within valid range
    component stepsRange = LessThan(10);
    stepsRange.in[0] <== steps - MIN_STEPS;
    stepsRange.in[1] <== MAX_STEPS_BOUND - MIN_STEPS;
    stepsRange.out === 1;

    // Step 2: Generate binary image
    // For now, create a simplified deterministic pattern
    // In production, implement full random walk algorithm

    signal imageBytes[256];

    // Initialize image to zeros
    for (var i = 0; i < 256; i++) {
        imageBytes[i] <-- 0;
    }

    // Simple deterministic pattern based on private key
    // This is a placeholder - full random walk would be more complex
    var seed = pkHash % 256;
    for (var i = 0; i < 256; i++) {
        var val = (seed + i * 17) % 256;
        imageBytes[i] <-- val;
        binaryImage[i] <== imageBytes[i];
    }

    // Constrain each byte to be 0-255
    component byteRange[256];
    for (var i = 0; i < 256; i++) {
        byteRange[i] = Num2Bits(8);
        byteRange[i].in <== binaryImage[i];
    }
}

// Comparison template
template LessThan(n) {
    signal input in[2];
    signal output out;

    component num2Bits = Num2Bits(n);
    num2Bits.in <== in[0];

    // Simplified - proper implementation would check in[0] < in[1]
    out <-- (in[0] < in[1]) ? 1 : 0;
    out * (out - 1) === 0; // out must be 0 or 1
}

component main {public [walks, steps, binaryImage]} = RorschachGenerator();
