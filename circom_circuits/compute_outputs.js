// Compute the circuit outputs given a private key
// This matches the logic in rorschach_simple.circom

const buildPoseidon = require("circomlibjs").buildPoseidon;

async function computeOutputs(privateKey) {
    const poseidon = await buildPoseidon();
    const F = poseidon.F;

    // Hash private key in two chunks
    const chunk1 = privateKey.slice(0, 16);
    const chunk2 = privateKey.slice(16, 32);

    const hash1 = poseidon(chunk1);
    const hash2 = poseidon(chunk2);

    // Final hash
    const pkHash = poseidon([hash1, hash2]);
    const pkHashBigInt = F.toObject(pkHash);

    console.log("Private key hash:", pkHashBigInt.toString());

    // Derive walks: (pkHash % 17) + 3
    const walksTemp = Number(pkHashBigInt % 17n);
    const walks = walksTemp + 3;

    // Derive steps: ((pkHash >> 16) % 220) + 80
    const stepsTemp = Number((pkHashBigInt >> 16n) % 220n);
    const steps = stepsTemp + 80;

    // Derive seed: pkHash % 256
    const seed = Number(pkHashBigInt % 256n);

    // Generate binary image
    const binaryImage = [];
    for (let i = 0; i < 256; i++) {
        const combined = Number((BigInt(privateKey[i % 32]) + BigInt(seed) + BigInt(i * 17)) % 256n);
        binaryImage.push(combined);
    }

    return {
        walks,
        steps,
        binaryImage
    };
}

// Test with our private key
const privateKey = Array(32).fill(17);

computeOutputs(privateKey).then(outputs => {
    console.log("Outputs:");
    console.log("Walks:", outputs.walks);
    console.log("Steps:", outputs.steps);
    console.log("Binary image (first 10 bytes):", outputs.binaryImage.slice(0, 10));

    // Write full input for snarkjs
    const fullInput = {
        privateKey: privateKey,
        walks: outputs.walks.toString(),
        steps: outputs.steps.toString(),
        binaryImage: outputs.binaryImage.map(b => b.toString())
    };

    const fs = require('fs');
    fs.writeFileSync('input_full.json', JSON.stringify(fullInput, null, 2));
    console.log("\nFull input written to input_full.json");
});
