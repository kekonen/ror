// Diagnostic program to show what parameters the circuit derives
// This helps us understand what values to put in Prover.toml

use dep::std;

fn main(private_key: [u8; 32]) -> pub (u64, u64) {
    let (walks, steps) = derive_parameters_pedersen(private_key);
    (walks, steps)
}

fn derive_parameters_pedersen(pk: [u8; 32]) -> (u64, u64) {
    // Pack bytes into field elements
    let mut pk_fields: [Field; 2] = [0, 0];

    let mut accumulator: Field = 0;
    for i in 0..16 {
        let byte_val = pk[i] as Field;
        accumulator = accumulator * 256 + byte_val;
    }
    pk_fields[0] = accumulator;

    accumulator = 0;
    for i in 16..32 {
        let byte_val = pk[i] as Field;
        accumulator = accumulator * 256 + byte_val;
    }
    pk_fields[1] = accumulator;

    let hash_output = std::hash::pedersen_hash(pk_fields);
    let hash_as_u64 = hash_output as u64;

    let walks = 3 + (hash_as_u64 % 18);
    let hash_shifted = (hash_output / 1000000) as u64;
    let steps = 80 + (hash_shifted % 221);

    (walks, steps)
}
