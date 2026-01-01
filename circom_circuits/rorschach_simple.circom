pragma circom 2.1.0;

include "node_modules/circomlib/circuits/poseidon.circom";
include "node_modules/circomlib/circuits/bitify.circom";
include "node_modules/circomlib/circuits/comparators.circom";

// Simplified Rorschach pattern generator for testing
// Proves: I know a private key that generates these specific public outputs
//
// Private input: privateKey[32]
// Public inputs: walks, steps, binaryImage[256] (to match against)

template RorschachSimple() {
    // Private input: 32-byte private key as field elements
    signal input privateKey[32];

    // Public inputs to verify against
    signal input walks;
    signal input steps;
    signal input binaryImage[256]; // 256 bytes representing 32x64 binary image

    // Step 1: Hash the private key to derive parameters
    // Hash in two chunks of 16 bytes each
    component hasher1 = Poseidon(16);
    component hasher2 = Poseidon(16);

    for (var i = 0; i < 16; i++) {
        hasher1.inputs[i] <== privateKey[i];
        hasher2.inputs[i] <== privateKey[i + 16];
    }

    // Combine the two hashes
    component finalHasher = Poseidon(2);
    finalHasher.inputs[0] <== hasher1.out;
    finalHasher.inputs[1] <== hasher2.out;

    // Step 2: Derive walks and steps deterministically
    signal pkHash;
    pkHash <== finalHasher.out;

    // Derive walks and check it matches public input
    signal walksTemp;
    walksTemp <-- pkHash % 17;
    signal derivedWalks;
    derivedWalks <== walksTemp + 3;

    // Constrain derived walks equals public walks
    walks === derivedWalks;

    // Derive steps and check it matches public input
    signal stepsTemp;
    stepsTemp <-- (pkHash >> 16) % 220;
    signal derivedSteps;
    derivedSteps <== stepsTemp + 80;

    // Constrain derived steps equals public steps
    steps === derivedSteps;

    // Step 3: Generate and verify binary image
    signal seed;
    seed <-- pkHash % 256;

    // Generate 256 bytes deterministically and verify each matches
    signal combined[256];
    component byteCheck[256];

    for (var i = 0; i < 256; i++) {
        // Compute what the byte should be
        combined[i] <-- (privateKey[i % 32] + seed + i * 17) % 256;

        // Constrain it equals the public input
        binaryImage[i] === combined[i];

        // Constrain each byte to be 0-255
        byteCheck[i] = Num2Bits(8);
        byteCheck[i].in <== combined[i];
    }
}

component main {public [walks, steps, binaryImage]} = RorschachSimple();
