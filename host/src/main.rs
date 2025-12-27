use std::{fs, path::PathBuf, str::FromStr};

use alloy::signers::local::PrivateKeySigner;
use clap::Parser;
use image::{ImageBuffer, Rgb};
use risc0_zkvm::{default_prover, ExecutorEnv, Receipt};
use ror_core::{binary_to_rgb, derive_parameters, generate_rorschach_half, BinaryImage32x64, Image32x64, Pixel};

// Include the generated guest code
use methods::{GUEST_ELF, GUEST_ID};

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

fn verify_proof(proof_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    println!("Verifying proof...");

    let receipt_bytes = fs::read(proof_path)?;
    let receipt: Receipt = bincode::deserialize(&receipt_bytes)?;

    receipt.verify(GUEST_ID)?;

    // Extract public outputs (now includes binary image, not RGB!)
    let address: [u8; 20] = receipt.journal.decode()?;
    let walks: u64 = receipt.journal.decode()?;
    let steps: u64 = receipt.journal.decode()?;

    // Decode binary image as 8 chunks of 32 bytes each (256 total)
    let mut binary_data = Vec::with_capacity(256);
    for _ in 0..8 {
        let chunk: [u8; 32] = receipt.journal.decode()?;
        binary_data.extend_from_slice(&chunk);
    }
    let binary_image = BinaryImage32x64::from_bytes(&binary_data);

    println!("✓ Proof verified successfully!");
    println!("  Address: 0x{}", hex::encode(address));
    println!("  Parameters: walks={}, steps={}", walks, steps);
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

    // Proof generation mode
    if cli.prove {
        let receipt = generate_proof(&private_key)?;

        // Extract public outputs (binary image, not RGB!)
        let address: [u8; 20] = receipt.journal.decode()?;
        let walks: u64 = receipt.journal.decode()?;
        let steps: u64 = receipt.journal.decode()?;

        // Decode binary image as 8 chunks of 32 bytes each (256 total)
        let mut binary_data = Vec::with_capacity(256);
        for _ in 0..8 {
            let chunk: [u8; 32] = receipt.journal.decode()?;
            binary_data.extend_from_slice(&chunk);
        }
        let binary_image = BinaryImage32x64::from_bytes(&binary_data);

        println!("✓ Proof generated successfully!");
        println!("  Address: 0x{}", hex::encode(address));
        println!("  Parameters: walks={}, steps={}", walks, steps);
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
