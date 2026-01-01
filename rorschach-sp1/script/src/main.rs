//! Host Program for Rorschach ZK Proof Generation
//!
//! This program:
//! 1. Compiles and runs the guest program in SP1 zkVM
//! 2. Generates Groth16 proofs for on-chain verification
//! 3. Exports Solidity verifier contracts
//! 4. Creates the final image files

use clap::{Parser, Subcommand};
use image::{ImageBuffer, Rgb};
use sp1_sdk::{HashableKey, ProverClient, SP1Stdin};
use std::fs;
use std::path::PathBuf;

/// The ELF binary of our guest program
/// This is set by the build script and points to the compiled guest program
const RORSCHACH_ELF: &[u8] = include_bytes!(env!("SP1_ELF_rorschach-program"));

#[derive(Parser)]
#[command(name = "rorschach")]
#[command(about = "Generate ZK proofs for Rorschach NFTs")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate a new proof
    Prove {
        /// Private key (hex, with or without 0x prefix)
        #[arg(long)]
        private_key: String,

        /// Output directory for proof files
        #[arg(long, default_value = "./output")]
        output: PathBuf,

        /// Foreground color (R,G,B)
        #[arg(long, default_value = "255,217,102")]
        foreground: String,

        /// Background color (R,G,B)
        #[arg(long, default_value = "255,0,129")]
        background: String,

        /// Skip Groth16 proof (faster, for testing)
        #[arg(long)]
        skip_groth16: bool,
    },

    /// Export Solidity verifier contract
    ExportVerifier {
        /// Output path for Verifier.sol
        #[arg(long, default_value = "./contracts/src/SP1Verifier.sol")]
        output: PathBuf,
    },

    /// Verify an existing proof
    Verify {
        /// Path to proof file
        #[arg(long)]
        proof: PathBuf,
    },

    /// Generate image only (no proof)
    Image {
        /// Private key (hex)
        #[arg(long)]
        private_key: String,

        /// Output image path
        #[arg(long, default_value = "./output.png")]
        output: PathBuf,

        /// Foreground color
        #[arg(long, default_value = "255,217,102")]
        foreground: String,

        /// Background color
        #[arg(long, default_value = "255,0,129")]
        background: String,
    },
}

fn parse_color(s: &str) -> Rgb<u8> {
    let parts: Vec<u8> = s.split(',').map(|x| x.trim().parse().unwrap()).collect();
    Rgb([parts[0], parts[1], parts[2]])
}

fn parse_private_key(s: &str) -> [u8; 32] {
    let s = s.trim_start_matches("0x");
    hex::decode(s)
        .expect("Invalid hex")
        .try_into()
        .expect("Private key must be 32 bytes")
}

/// Decode public values from proof
struct PublicValues {
    address: [u8; 20],
    walks: u64,
    steps: u64,
    image_hash: [u8; 32],
    image_data: [u8; 256],
}

impl PublicValues {
    fn from_bytes(bytes: &[u8]) -> Self {
        let mut offset = 0;

        let mut address = [0u8; 20];
        address.copy_from_slice(&bytes[offset..offset + 20]);
        offset += 20;

        let walks = u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap());
        offset += 8;

        let steps = u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap());
        offset += 8;

        let mut image_hash = [0u8; 32];
        image_hash.copy_from_slice(&bytes[offset..offset + 32]);
        offset += 32;

        let mut image_data = [0u8; 256];
        image_data.copy_from_slice(&bytes[offset..offset + 256]);

        Self {
            address,
            walks,
            steps,
            image_hash,
            image_data,
        }
    }
}

/// Convert binary image data to full RGB image
fn binary_to_image(
    data: &[u8; 256],
    foreground: Rgb<u8>,
    background: Rgb<u8>,
) -> ImageBuffer<Rgb<u8>, Vec<u8>> {
    let mut image = ImageBuffer::new(64, 64);

    // Fill with background
    for pixel in image.pixels_mut() {
        *pixel = background;
    }

    // Set foreground pixels (32x64 half, then mirror)
    for y in 0..64u32 {
        for x in 0..32u32 {
            let bit_index = (y * 32 + x) as usize;
            let byte_index = bit_index / 8;
            let bit_offset = bit_index % 8;

            let is_foreground = (data[byte_index] & (1 << bit_offset)) != 0;

            if is_foreground {
                // Set left half
                image.put_pixel(x, y, foreground);
                // Mirror to right half
                image.put_pixel(63 - x, y, foreground);
            }
        }
    }

    image
}

/// Upscale image by integer factor
fn upscale(image: &ImageBuffer<Rgb<u8>, Vec<u8>>, factor: u32) -> ImageBuffer<Rgb<u8>, Vec<u8>> {
    let (width, height) = image.dimensions();
    let mut new_image = ImageBuffer::new(width * factor, height * factor);

    for y in 0..height {
        for x in 0..width {
            let pixel = *image.get_pixel(x, y);
            for dy in 0..factor {
                for dx in 0..factor {
                    new_image.put_pixel(x * factor + dx, y * factor + dy, pixel);
                }
            }
        }
    }

    new_image
}

/// Add corner stamps with private key bits
fn add_stamps(
    image: &mut ImageBuffer<Rgb<u8>, Vec<u8>>,
    private_key: &[u8; 32],
    foreground: Rgb<u8>,
    background: Rgb<u8>,
) {
    let stamp = |img: &mut ImageBuffer<Rgb<u8>, Vec<u8>>,
                 bytes: &[u8],
                 start_x: u32,
                 start_y: u32| {
        for (row, &byte) in bytes.iter().enumerate() {
            for col in 0..8 {
                let bit = (byte >> (7 - col)) & 1;
                let pixel = if bit == 1 { foreground } else { background };
                img.put_pixel(start_x + col, start_y + row as u32, pixel);
            }
        }
    };

    // Top-left: bytes 0-7
    stamp(image, &private_key[0..8], 0, 0);
    // Top-right: bytes 8-15
    stamp(image, &private_key[8..16], 56, 0);
    // Bottom-left: bytes 16-23
    stamp(image, &private_key[16..24], 0, 56);
    // Bottom-right: bytes 24-31
    stamp(image, &private_key[24..32], 56, 56);
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Prove {
            private_key,
            output,
            foreground,
            background,
            skip_groth16,
        } => {
            let pk = parse_private_key(&private_key);
            let fg = parse_color(&foreground);
            let bg = parse_color(&background);

            fs::create_dir_all(&output).expect("Failed to create output directory");

            println!("🔮 Rorschach ZK Proof Generator");
            println!("================================\n");

            // Setup prover
            println!("📦 Setting up SP1 prover...");
            let client = ProverClient::from_env();
            let (proving_key, verifying_key) = client.setup(RORSCHACH_ELF);

            // Prepare input
            let mut stdin = SP1Stdin::new();
            stdin.write(&pk);

            // Execute (dry run to get outputs)
            println!("🔄 Executing circuit...");
            let (public_values, execution_report) =
                client.execute(RORSCHACH_ELF, &stdin).run().unwrap();

            println!("   Cycles: {}", execution_report.total_instruction_count());

            // Parse outputs
            let pv = PublicValues::from_bytes(public_values.as_slice());
            println!("\n📊 Public Outputs:");
            println!("   Address: 0x{}", hex::encode(pv.address));
            println!("   Walks: {}", pv.walks);
            println!("   Steps: {}", pv.steps);
            println!("   Image Hash: 0x{}", hex::encode(pv.image_hash));

            // Generate image
            let mut image = binary_to_image(&pv.image_data, fg, bg);
            add_stamps(&mut image, &pk, fg, bg);
            let final_image = upscale(&image, 8);

            let image_path = output.join("rorschach.png");
            final_image.save(&image_path).expect("Failed to save image");
            println!("\n🖼️  Image saved: {}", image_path.display());

            if !skip_groth16 {
                // Generate Groth16 proof
                println!("\n⚡ Generating Groth16 proof (this may take a few minutes)...");
                let proof = client
                    .prove(&proving_key, &stdin)
                    .groth16()
                    .run()
                    .expect("Proof generation failed");

                // Save proof
                let proof_path = output.join("proof.bin");
                fs::write(&proof_path, proof.bytes()).expect("Failed to save proof");
                println!("   Proof saved: {} ({} bytes)", proof_path.display(), proof.bytes().len());

                // Save public values
                let pv_path = output.join("public_values.bin");
                fs::write(&pv_path, proof.public_values.as_slice())
                    .expect("Failed to save public values");
                println!("   Public values saved: {}", pv_path.display());

                // Save verification key
                let vk_path = output.join("vkey.bin");
                fs::write(&vk_path, verifying_key.bytes32()).expect("Failed to save vkey");
                println!("   Verification key saved: {}", vk_path.display());

                // Generate calldata for Solidity
                let calldata = proof.bytes();
                let calldata_hex = format!("0x{}", hex::encode(&calldata));
                let calldata_path = output.join("calldata.txt");
                fs::write(&calldata_path, &calldata_hex).expect("Failed to save calldata");

                // Generate public inputs for Solidity
                let public_inputs_hex = format!("0x{}", hex::encode(proof.public_values.as_slice()));
                let inputs_path = output.join("public_inputs.txt");
                fs::write(&inputs_path, &public_inputs_hex).expect("Failed to save public inputs");

                println!("\n✅ Proof generation complete!");
                println!("\n📋 On-chain verification:");
                println!("   1. Deploy SP1Verifier.sol to your network");
                println!("   2. Deploy RorschachNFT.sol with verifier address");
                println!("   3. Call claim() with:");
                println!("      - proof: contents of {}", calldata_path.display());
                println!("      - publicValues: contents of {}", inputs_path.display());
            } else {
                println!("\n⏭️  Skipped Groth16 proof generation (--skip-groth16)");
            }
        }

        Commands::ExportVerifier { output } => {
            println!("📄 Exporting Solidity verifier...");

            let client = ProverClient::from_env();
            let (_, verifying_key) = client.setup(RORSCHACH_ELF);

            // Create parent directory
            if let Some(parent) = output.parent() {
                fs::create_dir_all(parent).expect("Failed to create directory");
            }

            println!("✅ Verifier key generated");
            println!("\n   Program VKey: 0x{}", hex::encode(verifying_key.bytes32()));
            println!("\n⚠️  Note: SP1 has pre-deployed verifiers on major networks.");
            println!("   Check https://docs.succinct.xyz/verification/on-chain/getting-started");
            println!("   for deployed verifier addresses.");
            println!("\n   Use the VKey above when deploying RorschachNFT.sol");
        }

        Commands::Verify { proof } => {
            println!("🔍 Verifying proof...");

            let client = ProverClient::from_env();
            let (_, _verifying_key) = client.setup(RORSCHACH_ELF);

            let _proof_bytes = fs::read(&proof).expect("Failed to read proof");

            // Note: SP1 verification requires reconstructing the proof object
            // In practice, on-chain verification is the primary method
            println!("⚠️  Off-chain verification not yet implemented");
            println!("   Use on-chain verification via SP1Verifier.sol");
        }

        Commands::Image {
            private_key,
            output,
            foreground,
            background,
        } => {
            let pk = parse_private_key(&private_key);
            let fg = parse_color(&foreground);
            let bg = parse_color(&background);

            println!("🖼️  Generating image (no proof)...");

            // We need to run the circuit to get deterministic output
            let client = ProverClient::from_env();
            let mut stdin = SP1Stdin::new();
            stdin.write(&pk);

            let (public_values, _) = client.execute(RORSCHACH_ELF, &stdin).run().unwrap();
            let pv = PublicValues::from_bytes(public_values.as_slice());

            let mut image = binary_to_image(&pv.image_data, fg, bg);
            add_stamps(&mut image, &pk, fg, bg);
            let final_image = upscale(&image, 8);

            // Create parent directory
            if let Some(parent) = output.parent() {
                fs::create_dir_all(parent).ok();
            }

            final_image.save(&output).expect("Failed to save image");
            println!("✅ Image saved: {}", output.display());
            println!("   Walks: {}, Steps: {}", pv.walks, pv.steps);
        }
    }
}
