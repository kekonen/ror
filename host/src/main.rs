use std::{fs, path::PathBuf, str::FromStr};

use alloy::signers::local::PrivateKeySigner;
use alloy_sol_types::{sol, SolValue};
use clap::Parser;
use image::{ImageBuffer, Rgb};
use risc0_zkvm::{default_prover, ExecutorEnv, Receipt};
use ror_core::{binary_to_rgb, derive_parameters, generate_rorschach_half, BinaryImage32x64, Image32x64, Pixel, ProofOutputs};

// Include the generated guest code
use methods::{GUEST_ELF, GUEST_ID};

// Define Solidity-compatible types for ABI encoding
sol! {
    struct JournalData {
        address ethAddress;
        uint64 walks;
        uint64 steps;
        bytes imageBytes;
    }
}

/// Encode journal data for Solidity compatibility
fn encode_journal_for_solidity(outputs: &ProofOutputs) -> Vec<u8> {
    // Flatten binary_chunks into single bytes array
    let mut image_bytes = Vec::with_capacity(256);
    for chunk in &outputs.binary_chunks {
        image_bytes.extend_from_slice(chunk);
    }

    // Convert address bytes to alloy_sol_types Address type
    let address = alloy_sol_types::private::Address::from_slice(&outputs.address);

    // Create JournalData struct
    let journal_data = JournalData {
        ethAddress: address,
        walks: outputs.walks,
        steps: outputs.steps,
        imageBytes: image_bytes.into(),
    };

    // ABI-encode for Solidity compatibility
    journal_data.abi_encode()
}

/// Check if platform supports Groth16 proving
fn check_groth16_platform() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(not(target_arch = "x86_64"))]
    {
        return Err("Groth16 proving requires x86_64 architecture. Current: ARM/other.
                   Please run on x86 Linux with Docker installed.".into());
    }

    #[cfg(target_arch = "x86_64")]
    {
        // Check if Docker is available
        let docker_check = std::process::Command::new("docker")
            .arg("--version")
            .output();

        match docker_check {
            Ok(output) if output.status.success() => Ok(()),
            _ => Err("Docker not found. Groth16 proving requires Docker.
                     Install: sudo apt-get install docker.io".into()),
        }
    }
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Output file path
    #[arg(short, long, value_name = "FILE", default_value = "./output.png")]
    output: PathBuf,

    /// Turn debugging information on
    #[arg(short, long, action = clap::ArgAction::Count)]
    debug: u8,

    #[arg(short, long, default_value = "255,0,129")]
    background: RgbX,

    #[arg(short, long, default_value = "255,217,102")]
    color: RgbX,

    /// Number of steps per walk (optional - derived from private key if not specified)
    #[arg(short, long)]
    steps: Option<u64>,

    /// Number of walks (optional - derived from private key if not specified)
    #[arg(short, long)]
    walks: Option<u64>,

    /// Ethereum private key (hex format, with or without 0x prefix)
    #[arg(long)]
    private_key: Option<String>,

    /// Generate a new random private key
    #[arg(long)]
    generate_key: bool,

    /// Disable private key stamp on edges
    #[arg(long)]
    no_stamp: bool,

    /// Offset of corner stamps from edges (default: 0)
    #[arg(long, default_value = "0")]
    stamp_offset: u64,

    /// Generate ZK proof
    #[arg(long)]
    prove: bool,

    /// Generate Groth16 proof for on-chain verification (requires x86 Linux + Docker)
    #[arg(long)]
    prove_groth16: bool,

    /// Verify an existing proof
    #[arg(long)]
    verify: Option<PathBuf>,
}

#[derive(Debug, Clone)]
struct RgbX(u8, u8, u8);

impl FromStr for RgbX {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts = s
            .split(',')
            .map(|s| s.parse().expect("Invalid RGB value"))
            .collect::<Vec<u8>>();
        if parts.len() < 3 {
            Err("RGB must have 3 components".to_string())
        } else {
            Ok(RgbX(parts[0], parts[1], parts[2]))
        }
    }
}

impl RgbX {
    fn to_rgb(&self) -> Rgb<u8> {
        Rgb([self.0, self.1, self.2])
    }

    fn to_pixel(&self) -> Pixel {
        Pixel::new(self.0, self.1, self.2)
    }
}

fn generate_proof(private_key: &[u8; 32]) -> Result<Receipt, Box<dyn std::error::Error>> {
    println!("Generating ZK proof... (this may take a while)");

    let env = ExecutorEnv::builder()
        .write(private_key)?
        .build()?;

    let prover = default_prover();
    let prove_info = prover.prove(env, GUEST_ELF)?;
    let receipt = prove_info.receipt;

    Ok(receipt)
}

#[cfg(feature = "groth16")]
fn generate_groth16_proof(
    private_key: &[u8; 32],
    output_path: &PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    use risc0_zkvm::{ProverOpts, InnerReceipt};

    println!("Generating Groth16 proof... (this may take 5-10 minutes)");
    println!("Using Docker backend for Groth16 conversion...");

    // Build execution environment
    let env = ExecutorEnv::builder()
        .write(private_key)?
        .build()?;

    // Get prover with Groth16 options
    println!("Step 1/2: Generating STARK proof...");
    let prover = default_prover();
    let opts = ProverOpts::groth16();
    let prove_info = prover.prove_with_opts(env, GUEST_ELF, &opts)?;

    println!("Step 2/2: Converting to Groth16 via Docker...");
    let receipt = prove_info.receipt;

    // Optionally save STARK receipt for debugging/reference
    let stark_path = output_path.with_extension("stark.proof");
    if let Ok(stark_bytes) = bincode::serialize(&receipt) {
        let _ = fs::write(&stark_path, stark_bytes);
    }

    // Decode outputs
    let outputs: ProofOutputs = receipt.journal.decode()?;

    // Get the Groth16 seal from the receipt
    let groth16_seal = match &receipt.inner {
        InnerReceipt::Groth16(receipt) => &receipt.seal,
        _ => return Err("Expected Groth16 receipt but got different type".into()),
    };

    // Reconstruct binary image from chunks
    let mut binary_data = Vec::with_capacity(256);
    for chunk in &outputs.binary_chunks {
        binary_data.extend_from_slice(chunk);
    }
    let binary_image = BinaryImage32x64::from_bytes(&binary_data);

    println!("✓ Groth16 proof generated successfully!");
    println!("  Address: 0x{}", hex::encode(outputs.address));
    println!("  Parameters: walks={}, steps={}", outputs.walks, outputs.steps);
    println!("  Binary image size: {} bytes", binary_image.data.len());

    // Save seal (Groth16 proof) for on-chain verification
    let seal_path = output_path.with_extension("seal");
    fs::write(&seal_path, groth16_seal)?;
    println!("  Groth16 seal saved to: {} ({} bytes)", seal_path.display(), groth16_seal.len());

    // Save journal (public outputs) for on-chain verification
    let journal_bytes = encode_journal_for_solidity(&outputs);
    let journal_path = output_path.with_extension("journal");
    fs::write(&journal_path, &journal_bytes)?;
    println!("  Journal saved to: {} ({} bytes)", journal_path.display(), journal_bytes.len());

    // Also save full receipt for reference
    let receipt_path = output_path.with_extension("groth16.proof");
    let receipt_bytes = bincode::serialize(&receipt)?;
    fs::write(&receipt_path, receipt_bytes)?;
    println!("  Full receipt saved to: {}", receipt_path.display());

    Ok(())
}

#[cfg(not(feature = "groth16"))]
fn generate_groth16_proof(
    _private_key: &[u8; 32],
    _output_path: &PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    Err("Groth16 feature not enabled. Build with: cargo build --features groth16".into())
}

fn verify_proof(proof_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    println!("Verifying proof...");

    let receipt_bytes = fs::read(proof_path)?;
    let receipt: Receipt = bincode::deserialize(&receipt_bytes)?;

    receipt.verify(GUEST_ID)?;

    // Decode all outputs as a single struct (risc0 best practice)
    let outputs: ProofOutputs = receipt.journal.decode()?;

    // Reconstruct binary image from chunks
    let mut binary_data = Vec::with_capacity(256);
    for chunk in &outputs.binary_chunks {
        binary_data.extend_from_slice(chunk);
    }
    let binary_image = BinaryImage32x64::from_bytes(&binary_data);

    println!("✓ Proof verified successfully!");
    println!("  Address: 0x{}", hex::encode(outputs.address));
    println!("  Parameters: walks={}, steps={}", outputs.walks, outputs.steps);
    println!("  Binary image size: {} bytes (24x smaller than RGB!)", binary_image.data.len());
    println!("  (Colors can be applied freely after verification)");

    Ok(())
}

fn mirror_half_to_full(half: &Image32x64, background: Pixel) -> ImageBuffer<Rgb<u8>, Vec<u8>> {
    let mut full_image = vec![background; 64 * 64];

    // Copy left half and mirror to right half
    for y in 0..64 {
        for x in 0..32 {
            let pixel = half.get_pixel(x, y).unwrap_or(background);
            full_image[(y * 64 + x) as usize] = pixel;

            let mirrored_x = 64 - x - 1;
            full_image[(y * 64 + mirrored_x) as usize] = pixel;
        }
    }

    // Convert to ImageBuffer
    let pixel_data: Vec<u8> = full_image
        .iter()
        .flat_map(|p| vec![p.r, p.g, p.b])
        .collect();

    ImageBuffer::from_raw(64, 64, pixel_data).unwrap()
}

fn add_corner_stamps(
    image: &mut ImageBuffer<Rgb<u8>, Vec<u8>>,
    private_key: &[u8; 32],
    foreground: Rgb<u8>,
    background: Rgb<u8>,
    offset: u64,
) {
    let width = 64u64;
    let height = 64u64;

    // Top-left: bytes 0-7
    stamp_corner(image, &private_key[0..8], offset, offset, foreground, background);

    // Top-right: bytes 8-15
    stamp_corner(
        image,
        &private_key[8..16],
        width - 8 - offset,
        offset,
        foreground,
        background,
    );

    // Bottom-left: bytes 16-23
    stamp_corner(
        image,
        &private_key[16..24],
        offset,
        height - 8 - offset,
        foreground,
        background,
    );

    // Bottom-right: bytes 24-31
    stamp_corner(
        image,
        &private_key[24..32],
        width - 8 - offset,
        height - 8 - offset,
        foreground,
        background,
    );
}

fn stamp_corner(
    image: &mut ImageBuffer<Rgb<u8>, Vec<u8>>,
    bytes: &[u8],
    start_x: u64,
    start_y: u64,
    foreground: Rgb<u8>,
    background: Rgb<u8>,
) {
    for (row, &byte) in bytes.iter().enumerate() {
        for col in 0..8 {
            let bit = (byte >> (7 - col)) & 1;
            let pixel = if bit == 1 { foreground } else { background };
            image.put_pixel(
                (start_x + col) as u32,
                (start_y + row as u64) as u32,
                pixel,
            );
        }
    }
}

fn upscale(image: &ImageBuffer<Rgb<u8>, Vec<u8>>, factor: u32) -> ImageBuffer<Rgb<u8>, Vec<u8>> {
    let (width, height) = image.dimensions();
    let new_width = width * factor;
    let new_height = height * factor;

    let mut new_image = ImageBuffer::new(new_width, new_height);

    for y in 0..height {
        for x in 0..width {
            let pixel = image.get_pixel(x, y);
            for dy in 0..factor {
                for dx in 0..factor {
                    new_image.put_pixel(x * factor + dx, y * factor + dy, *pixel);
                }
            }
        }
    }

    new_image
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Verify mode
    if let Some(proof_path) = cli.verify {
        return verify_proof(&proof_path);
    }

    // Generate or parse private key
    let private_key: [u8; 32] = if let Some(pk_hex) = cli.private_key {
        let pk_hex = pk_hex.trim_start_matches("0x");
        hex::decode(pk_hex)?
            .try_into()
            .map_err(|_| "Private key must be exactly 32 bytes")?
    } else if cli.generate_key {
        let signer = PrivateKeySigner::random();
        let pk_bytes = signer.credential().to_bytes();
        println!("Generated private key: 0x{}", hex::encode(pk_bytes));
        println!("Address: {}", signer.address());
        pk_bytes.into()
    } else {
        return Err("Must provide --private-key or --generate-key".into());
    };

    // Groth16 proof generation mode (on-chain ready)
    if cli.prove_groth16 {
        check_groth16_platform()?;
        generate_groth16_proof(&private_key, &cli.output)?;

        // Still generate the image for visualization
        let (default_walks, default_steps) = derive_parameters(&private_key);
        let walks = cli.walks.unwrap_or(default_walks);
        let steps = cli.steps.unwrap_or(default_steps);

        let foreground = cli.color.to_pixel();
        let background = cli.background.to_pixel();
        let half_image = generate_rorschach_half(&private_key, walks, steps, foreground, background);

        let mut full_image = mirror_half_to_full(&half_image, background);
        if !cli.no_stamp {
            add_corner_stamps(&mut full_image, &private_key,
                cli.color.to_rgb(), cli.background.to_rgb(), cli.stamp_offset);
        }
        let final_image = upscale(&full_image, 8);
        final_image.save(&cli.output)?;
        println!("  Image saved to: {}", cli.output.display());

        return Ok(());
    }

    // STARK proof generation mode
    if cli.prove {
        let receipt = generate_proof(&private_key)?;

        // Decode all outputs as a single struct (risc0 best practice)
        let outputs: ProofOutputs = receipt.journal.decode()?;

        // Reconstruct binary image from chunks
        let mut binary_data = Vec::with_capacity(256);
        for chunk in &outputs.binary_chunks {
            binary_data.extend_from_slice(chunk);
        }
        let binary_image = BinaryImage32x64::from_bytes(&binary_data);

        println!("✓ Proof generated successfully!");
        println!("  Address: 0x{}", hex::encode(outputs.address));
        println!("  Parameters: walks={}, steps={}", outputs.walks, outputs.steps);
        println!("  Binary image size: {} bytes (24x smaller than RGB!)", binary_image.data.len());

        // Save proof
        let proof_path = cli.output.with_extension("proof");
        let receipt_bytes = bincode::serialize(&receipt)?;
        fs::write(&proof_path, receipt_bytes)?;
        println!("  Proof saved to: {}", proof_path.display());

        // Convert binary image to RGB with user's chosen colors
        let foreground = cli.color.to_pixel();
        let background = cli.background.to_pixel();
        let half_image = binary_to_rgb(&binary_image, foreground, background);

        // Mirror to full 64×64
        let mut full_image = mirror_half_to_full(&half_image, background);

        // Add stamps if requested
        if !cli.no_stamp {
            add_corner_stamps(
                &mut full_image,
                &private_key,
                Rgb(foreground.to_rgb_array()),
                Rgb(background.to_rgb_array()),
                cli.stamp_offset
            );
        }

        // Upscale to 512×512
        let final_image = upscale(&full_image, 8);

        // Save final image
        final_image.save(&cli.output)?;
        println!("  Image saved to: {}", cli.output.display());
        println!("  (Colors applied after verification - can be changed freely!)");

        return Ok(());
    }

    // Normal generation mode (without proof)
    let (default_walks, default_steps) = derive_parameters(&private_key);
    let walks = cli.walks.unwrap_or(default_walks);
    let steps = cli.steps.unwrap_or(default_steps);

    if cli.debug > 0 {
        println!("Using walks={}, steps={}", walks, steps);
        if cli.walks.is_none() && cli.steps.is_none() {
            println!("(derived from private key)");
        }
    }

    let foreground = cli.color.to_pixel();
    let background = cli.background.to_pixel();

    // Generate half-canvas
    let half_image = generate_rorschach_half(&private_key, walks, steps, foreground, background);

    // Mirror to full 64×64
    let mut full_image = mirror_half_to_full(&half_image, background);

    // Add stamps if requested
    if !cli.no_stamp {
        add_corner_stamps(
            &mut full_image,
            &private_key,
            cli.color.to_rgb(),
            cli.background.to_rgb(),
            cli.stamp_offset,
        );
    }

    // Upscale to 512×512
    let final_image = upscale(&full_image, 8);

    // Save
    final_image.save(&cli.output)?;

    if cli.debug > 0 {
        println!("Image saved to: {}", cli.output.display());
    }

    Ok(())
}
