// Main circuit for Rorschach proof
// Proves:
// 1. Knowledge of private key that derives to a specific Ethereum address
// 2. Deterministic generation of binary image from that private key
fn main(
    // Private inputs (witness - kept secret)
    private_key: [u8; 32],

    // Public outputs (verifiable)
    ethereum_address: pub [u8; 20],
    walks: pub u64,
    steps: pub u64,
    binary_image: pub [u8; 256]
) {
    // NOTE: For MVP, we'll skip the full secp256k1 public key derivation
    // which is expensive in-circuit. Instead, we'll:
    // 1. Use the private key to derive parameters and image (proving we know it)
    // 2. Have the host pass in the ethereum_address as a public input
    // 3. The prover can verify off-chain that address matches the private key

    // This is a pragmatic approach for getting Noir working quickly.
    // Full secp256k1 derivation can be added later using ecrecover library.

    // Mark ethereum_address as used (it's just a public output for now)
    let _ = ethereum_address;

    // Derive parameters using Poseidon hash (circuit-friendly)
    let (derived_walks, derived_steps) = derive_parameters_poseidon(private_key);
    assert(derived_walks == walks);
    assert(derived_steps == steps);

    // Generate binary image using random walks
    let generated_image = generate_binary_image(private_key, walks, steps);

    // Verify all 256 bytes match
    for i in 0..256 {
        assert(generated_image[i] == binary_image[i]);
    }
}

// Derive walk and step parameters from private key using Poseidon hash
// Replaces ChaCha8 RNG with circuit-friendly Poseidon
fn derive_parameters_poseidon(pk: [u8; 32]) -> (u64, u64) {
    // Convert private key bytes to field elements for Poseidon
    // Poseidon works on field elements, so we need to pack bytes
    let mut pk_fields: [Field; 2] = [0, 0];

    // Pack first 16 bytes into pk_fields[0]
    let mut accumulator: Field = 0;
    for i in 0..16 {
        let byte_val = pk[i] as Field;
        accumulator = accumulator * 256 + byte_val;
    }
    pk_fields[0] = accumulator;

    // Pack last 16 bytes into pk_fields[1]
    accumulator = 0;
    for i in 16..32 {
        let byte_val = pk[i] as Field;
        accumulator = accumulator * 256 + byte_val;
    }
    pk_fields[1] = accumulator;

    // Hash with Pedersen to get deterministic randomness
    // (Pedersen is in std lib, simpler than Poseidon for MVP)
    let hash_output = std::hash::pedersen_hash(pk_fields);

    // Convert hash output to bytes for parameter extraction
    let hash_as_u64 = hash_output as u64;

    // Extract walks: 3-20 range (matches original ChaCha8 logic)
    let walks = 3 + (hash_as_u64 % 18);

    // Use different part of hash for steps
    let hash_shifted = (hash_output / 1000000) as u64;
    let steps = 80 + (hash_shifted % 221);

    (walks, steps)
}

// Generate binary Rorschach image from private key using random walks
fn generate_binary_image(pk: [u8; 32], walks: u64, steps: u64) -> [u8; 256] {
    let mut image_bytes = [0; 256];

    // Virtual coordinate system (same as Rust implementation)
    let VIRTUAL_WIDTH: u64 = 64;
    let PHYSICAL_WIDTH: u64 = 32;
    let HEIGHT: u64 = 64;
    let left_margin = VIRTUAL_WIDTH / 4;  // 16
    let right_boundary = 3 * VIRTUAL_WIDTH / 4;  // 48
    let top_margin = HEIGHT / 4;  // 16
    let bottom_boundary = 3 * HEIGHT / 4;  // 48

    // Initialize Poseidon-based RNG state
    let mut rng_counter: u32 = 0;

    // Convert private key to Field for Poseidon RNG seed
    let mut pk_field: Field = 0;
    for i in 0..32 {
        pk_field = pk_field * 256 + (pk[i] as Field);
    }

    // Perform random walks (note: walks must be bounded at compile time)
    // We'll set a maximum of 20 walks
    let MAX_WALKS: u64 = 20;
    for walk_idx in 0..MAX_WALKS {
        // Only execute if within actual walks count
        let should_walk = walk_idx < walks;

        if should_walk {
            // Generate random starting position
            let (start_x, next_counter1) = next_random_range(
                pk_field, rng_counter, left_margin, right_boundary
            );
            rng_counter = next_counter1;

            let (start_y, next_counter2) = next_random_range(
                pk_field, rng_counter, top_margin, bottom_boundary
            );
            rng_counter = next_counter2;

            let mut cursor_x = start_x;
            let mut cursor_y = start_y;

            // Set starting pixel
            set_binary_pixel(&mut image_bytes, cursor_x, cursor_y, VIRTUAL_WIDTH, PHYSICAL_WIDTH);

            // Random walk steps (bounded at compile time)
            let MAX_STEPS: u64 = 300;
            for step_idx in 0..MAX_STEPS {
                let should_step = step_idx < steps;

                if should_step {
                    // Decide direction based on position
                    let (direction, next_counter3) = decide_direction(
                        pk_field, rng_counter, cursor_x, cursor_y,
                        left_margin, right_boundary, top_margin, bottom_boundary
                    );
                    rng_counter = next_counter3;

                    // Move cursor based on direction
                    if direction == 0 {
                        // Left
                        if cursor_x > left_margin {
                            cursor_x -= 1;
                        }
                    } else if direction == 1 {
                        // Right
                        if cursor_x < right_boundary - 1 {
                            cursor_x += 1;
                        }
                    } else if direction == 2 {
                        // Up
                        if cursor_y > top_margin {
                            cursor_y -= 1;
                        }
                    } else if direction == 3 {
                        // Down
                        if cursor_y < bottom_boundary - 1 {
                            cursor_y += 1;
                        }
                    }

                    // Draw pixel at new position
                    set_binary_pixel(&mut image_bytes, cursor_x, cursor_y, VIRTUAL_WIDTH, PHYSICAL_WIDTH);
                }
            }
        }
    }

    image_bytes
}

// Helper: Set a binary pixel in the packed byte array
fn set_binary_pixel(
    bytes: &mut [u8; 256],
    virtual_x: u64,
    y: u64,
    VIRTUAL_WIDTH: u64,
    PHYSICAL_WIDTH: u64
) {
    // Transform virtual coordinates to physical (mirror across center)
    let physical_x = if virtual_x >= PHYSICAL_WIDTH {
        VIRTUAL_WIDTH - virtual_x - 1
    } else {
        virtual_x
    };

    // Calculate bit position in packed array
    let bit_index = y * PHYSICAL_WIDTH + physical_x;
    let byte_index = (bit_index / 8) as u32;
    let bit_offset = (bit_index % 8) as u8;

    // Set the bit (MSB first within each byte)
    bytes[byte_index] |= (1 as u8) << (7 - bit_offset);
}

// Helper: Decide movement direction based on position (boundary awareness)
fn decide_direction(
    seed: Field,
    counter: u32,
    cursor_x: u64,
    cursor_y: u64,
    left_margin: u64,
    right_boundary: u64,
    top_margin: u64,
    bottom_boundary: u64
) -> (u8, u32) {
    // Fixed-point scale for probability calculations
    let SCALE: u64 = 1_000_000;

    // Calculate distances from boundaries
    let dist_left = cursor_x - left_margin;
    let dist_right = right_boundary - cursor_x;
    let dist_top = cursor_y - top_margin;
    let dist_bottom = bottom_boundary - cursor_y;

    // Calculate probabilities (favor moving away from edges)
    let left_prob = if dist_left >= 16 { SCALE } else { (dist_left * SCALE) / 16 };
    let right_prob = if dist_right >= 16 { SCALE } else { (dist_right * SCALE) / 16 };
    let up_prob = if dist_top >= 16 { SCALE } else { (dist_top * SCALE) / 16 };
    let down_prob = if dist_bottom >= 16 { SCALE } else { (dist_bottom * SCALE) / 16 };

    let total = left_prob + right_prob + up_prob + down_prob;

    // Generate random value using Pedersen hash
    let hash_input = [seed, counter as Field];
    let rand_field = std::hash::pedersen_hash(hash_input);
    let rand_val = (rand_field as u64) % total;

    // Select direction based on cumulative probabilities
    let direction = if rand_val < left_prob {
        0  // Left
    } else if rand_val < left_prob + right_prob {
        1  // Right
    } else if rand_val < left_prob + right_prob + up_prob {
        2  // Up
    } else {
        3  // Down
    };

    (direction, counter + 1)
}

// Helper: Generate random number in range using Pedersen hash
fn next_random_range(seed: Field, counter: u32, min: u64, max: u64) -> (u64, u32) {
    let hash_input = [seed, counter as Field];
    let hash_output = std::hash::pedersen_hash(hash_input);

    let range = max - min;
    let value = min + ((hash_output as u64) % range);

    (value, counter + 1)
}

#[test]
fn test_derive_parameters() {
    // Test that parameter derivation is deterministic
    let test_key = [0x12; 32];
    let (walks1, steps1) = derive_parameters_poseidon(test_key);
    let (walks2, steps2) = derive_parameters_poseidon(test_key);

    assert(walks1 == walks2);
    assert(steps1 == steps2);
    assert(walks1 >= 3);
    assert(walks1 <= 20);
    assert(steps1 >= 80);
    assert(steps1 <= 300);
}

#[test]
fn test_pixel_setting() {
    // Test that pixel setting works correctly
    let mut image = [0; 256];

    // Set a pixel in the upper left
    set_binary_pixel(&mut image, 0, 0, 64, 32);

    // First bit of first byte should be set
    assert(image[0] == 0x80);  // 10000000 in binary
}
