// Test tool for Noir integration
// Demonstrates calling nargo and parsing outputs
//
// Run with: cargo run --bin test_noir_integration

use ror_core::noir::execute_noir_circuit;
use ror_core::BinaryImage32x64;

fn main() {
    println!("=== Noir Circuit Integration Test ===\n");

    // Use the same test private key from Prover.toml
    let private_key = [
        0x12, 0x34, 0x56, 0x78, 0x90, 0xab, 0xcd, 0xef,
        0x12, 0x34, 0x56, 0x78, 0x90, 0xab, 0xcd, 0xef,
        0x12, 0x34, 0x56, 0x78, 0x90, 0xab, 0xcd, 0xef,
        0x12, 0x34, 0x56, 0x78, 0x90, 0xab, 0xcd, 0xef,
    ];

    println!("Private key: {:02x?}\n", &private_key[..8]);

    // Execute Noir circuit
    println!("Executing Noir circuit via nargo...");
    match execute_noir_circuit(&private_key, "circuits") {
        Ok(outputs) => {
            println!("✅ Circuit execution successful!\n");

            println!("Computed values:");
            println!("  Walks: {}", outputs.walks);
            println!("  Steps: {}", outputs.steps);

            // Create BinaryImage from the outputs
            let binary_image = BinaryImage32x64::from_bytes(&outputs.binary_image);

            // Count how many pixels are set
            let mut pixel_count = 0;
            for y in 0..64 {
                for x in 0..32 {
                    if binary_image.get_pixel(x, y) {
                        pixel_count += 1;
                    }
                }
            }

            println!("  Pixels set: {} / {} ({:.1}%)",
                pixel_count,
                32 * 64,
                (pixel_count as f64 / (32.0 * 64.0)) * 100.0
            );

            println!("\nBinary image data (first 32 bytes):");
            for i in 0..32 {
                print!(" {:02x}", outputs.binary_image[i]);
                if (i + 1) % 16 == 0 {
                    println!();
                }
            }

            println!("\n✅ Noir integration working correctly!");
            println!("\nNext steps:");
            println!("1. Generate proof with: cd circuits && nargo prove");
            println!("2. Verify proof with: cd circuits && nargo verify");
            println!("3. Integrate into host/src/main.rs for full workflow");
        }
        Err(e) => {
            eprintln!("❌ Error executing circuit: {}", e);
            eprintln!("\nMake sure:");
            eprintln!("1. You're running from the project root directory");
            eprintln!("2. nargo is installed and in PATH");
            eprintln!("3. circuits/ directory exists with compiled circuit");
            std::process::exit(1);
        }
    }
}
