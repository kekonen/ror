// Quick test to verify Pedersen parameter derivation
// Run with: cargo run --bin test_pedersen

use ror_core::derive_parameters_pedersen;

fn main() {
    println!("Testing Pedersen-based parameter derivation...\n");

    // Test with the same private key used in Prover.toml
    let test_key = [
        0x12, 0x34, 0x56, 0x78, 0x90, 0xab, 0xcd, 0xef,
        0x12, 0x34, 0x56, 0x78, 0x90, 0xab, 0xcd, 0xef,
        0x12, 0x34, 0x56, 0x78, 0x90, 0xab, 0xcd, 0xef,
        0x12, 0x34, 0x56, 0x78, 0x90, 0xab, 0xcd, 0xef,
    ];

    println!("Test private key: {:02x?}", test_key);

    let (walks, steps) = derive_parameters_pedersen(&test_key);

    println!("\nDerived parameters:");
    println!("  Walks: {} (expected range: 3-20)", walks);
    println!("  Steps: {} (expected range: 80-300)", steps);

    // Validate ranges
    assert!(walks >= 3 && walks <= 20, "Walks out of range: {}", walks);
    assert!(steps >= 80 && steps <= 300, "Steps out of range: {}", steps);

    println!("\n✓ Parameters are within valid ranges!");

    // Test determinism - same key should always produce same values
    let (walks2, steps2) = derive_parameters_pedersen(&test_key);
    assert_eq!(walks, walks2, "Determinism check failed for walks");
    assert_eq!(steps, steps2, "Determinism check failed for steps");

    println!("✓ Determinism verified - same key produces same parameters");

    // Test with a different key
    let different_key = [0xFF; 32];
    let (walks3, steps3) = derive_parameters_pedersen(&different_key);
    println!("\nDifferent key test:");
    println!("  Walks: {}, Steps: {}", walks3, steps3);
    assert!(walks3 >= 3 && walks3 <= 20);
    assert!(steps3 >= 80 && steps3 <= 300);

    println!("\n✅ All tests passed!");
    println!("\nYou can now update circuits/Prover.toml with:");
    println!("walks = \"{}\"", walks);
    println!("steps = \"{}\"", steps);
}
