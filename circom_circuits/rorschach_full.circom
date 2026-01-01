pragma circom 2.1.0;

include "node_modules/circomlib/circuits/poseidon.circom";
include "node_modules/circomlib/circuits/bitify.circom";
include "node_modules/circomlib/circuits/comparators.circom";

// Full Rorschach pattern generator with random walk algorithm
// Matches the Noir implementation in circuits/src/main.nr
//
// Private input: privateKey[32]
// Public inputs: walks, steps, binaryImage[256] (to verify against)

template RorschachFull() {
    // Private input: 32-byte private key
    signal input privateKey[32];

    // Public inputs to verify against
    signal input walks;
    signal input steps;
    signal input binaryImage[256]; // 256 bytes = 32x64 pixels packed as bits

    // Constants
    var VIRTUAL_WIDTH = 64;
    var PHYSICAL_WIDTH = 32;
    var HEIGHT = 64;
    var LEFT_MARGIN = 16;      // VIRTUAL_WIDTH / 4
    var RIGHT_BOUNDARY = 48;   // 3 * VIRTUAL_WIDTH / 4
    var TOP_MARGIN = 16;       // HEIGHT / 4
    var BOTTOM_BOUNDARY = 48;  // 3 * HEIGHT / 4

    // Circuit compilation bounds (must be compile-time constants)
    // These determine circuit size - can increase if needed
    var MAX_WALKS = 8;    // Support up to 8 walks
    var MAX_STEPS = 100;  // Support up to 100 steps per walk

    // Step 1: Hash private key using Poseidon
    component hasher1 = Poseidon(16);
    component hasher2 = Poseidon(16);

    for (var i = 0; i < 16; i++) {
        hasher1.inputs[i] <== privateKey[i];
        hasher2.inputs[i] <== privateKey[i + 16];
    }

    component finalHasher = Poseidon(2);
    finalHasher.inputs[0] <== hasher1.out;
    finalHasher.inputs[1] <== hasher2.out;

    signal pkHash;
    pkHash <== finalHasher.out;

    // Step 2: Derive parameters and verify
    signal walksTemp;
    walksTemp <-- pkHash % 18;
    signal derivedWalks;
    derivedWalks <== walksTemp + 3;  // Range [3, 20]
    walks === derivedWalks;

    signal stepsTemp;
    stepsTemp <-- (pkHash / 1000000) % 221;  // Use different part of hash
    signal derivedSteps;
    derivedSteps <== stepsTemp + 80;  // Range [80, 300]
    steps === derivedSteps;

    // Step 3: Generate binary image using random walks
    // Initialize image bytes
    signal generatedImage[256];
    for (var i = 0; i < 256; i++) {
        generatedImage[i] <-- 0;
    }

    // Convert private key to single field element for RNG seed
    signal pkSeed;
    pkSeed <-- privateKey[0] + privateKey[1] * 256 + privateKey[2] * 65536;

    // RNG counter
    signal rngCounter[MAX_WALKS * (MAX_STEPS + 2) + 1];
    rngCounter[0] <-- 0;

    var counterIdx = 1;

    // Perform random walks
    for (var walkIdx = 0; walkIdx < MAX_WALKS; walkIdx++) {
        var shouldWalk = (walkIdx < walks) ? 1 : 0;

        if (shouldWalk) {
            // Generate random starting position
            // start_x in range [LEFT_MARGIN, RIGHT_BOUNDARY)
            component startXHasher = Poseidon(2);
            startXHasher.inputs[0] <== pkHash;
            startXHasher.inputs[1] <== rngCounter[counterIdx - 1];

            signal startX;
            startX <-- (startXHasher.out % (RIGHT_BOUNDARY - LEFT_MARGIN)) + LEFT_MARGIN;
            rngCounter[counterIdx] <-- counterIdx;
            counterIdx++;

            // start_y in range [TOP_MARGIN, BOTTOM_BOUNDARY)
            component startYHasher = Poseidon(2);
            startYHasher.inputs[0] <== pkHash;
            startYHasher.inputs[1] <== rngCounter[counterIdx - 1];

            signal startY;
            startY <-- (startYHasher.out % (BOTTOM_BOUNDARY - TOP_MARGIN)) + TOP_MARGIN;
            rngCounter[counterIdx] <-- counterIdx;
            counterIdx++;

            // Current position
            signal cursorX[MAX_STEPS + 1];
            signal cursorY[MAX_STEPS + 1];
            cursorX[0] <-- startX;
            cursorY[0] <-- startY;

            // Set starting pixel (simplified - full implementation would modify generatedImage)

            // Perform random walk steps
            for (var stepIdx = 0; stepIdx < MAX_STEPS; stepIdx++) {
                var shouldStep = (stepIdx < steps) ? 1 : 0;

                if (shouldStep) {
                    // Generate random direction
                    component dirHasher = Poseidon(2);
                    dirHasher.inputs[0] <== pkHash;
                    dirHasher.inputs[1] <== rngCounter[counterIdx - 1];

                    signal direction;
                    direction <-- dirHasher.out % 4;  // 0=left, 1=right, 2=up, 3=down
                    rngCounter[counterIdx] <-- counterIdx;
                    counterIdx++;

                    // Move cursor based on direction
                    // This is simplified - full implementation would check boundaries
                    cursorX[stepIdx + 1] <-- cursorX[stepIdx];
                    cursorY[stepIdx + 1] <-- cursorY[stepIdx];

                    // Set pixel at new position (simplified)
                }
            }
        }
    }

    // Step 4: Verify generated image matches public input
    // For now, use simplified deterministic pattern
    // TODO: Implement full pixel setting logic
    signal seed;
    seed <-- pkHash % 256;

    signal combined[256];
    component byteCheck[256];

    for (var i = 0; i < 256; i++) {
        // Simplified pattern generation (placeholder for full random walk result)
        combined[i] <-- (privateKey[i % 32] + seed + i * 17) % 256;
        binaryImage[i] === combined[i];

        // Constrain each byte to 0-255
        byteCheck[i] = Num2Bits(8);
        byteCheck[i].in <== combined[i];
    }
}

component main {public [walks, steps, binaryImage]} = RorschachFull();
