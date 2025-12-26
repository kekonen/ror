use std::{path::PathBuf, str::FromStr};

use alloy::signers::local::PrivateKeySigner;
use clap::Parser;
use image::{ImageBuffer, Pixel, Rgb};
use rand::prelude::*;
use rand::SeedableRng;

struct PixelImage<T> {
    width: u64,
    height: u64,
    // image::Rgb<u8>
    pixels: Vec<T>,
}

impl PixelImage<Rgb<u8>> {
    fn new(width: u64, height: u64, rgb: Option<Rgb<u8>>) -> Self {
        let pixel = rgb.unwrap_or(Rgb([255, 255, 255]));
        let mut pixels: Vec<Rgb<u8>> = Vec::with_capacity((width * height) as usize);
        for _ in 0..width * height {
            pixels.push(rgb.unwrap_or(pixel));
        }
        Self {
            width,
            height,
            pixels,
        }
    }

    fn get_pixel(&self, x: u64, y: u64) -> Option<&Rgb<u8>> {
        if x < self.width && y < self.height {
            Some(&self.pixels[(y * self.width + x) as usize])
        } else {
            None
        }
    }

    fn set_pixel(&mut self, x: u64, y: u64, pixel: Rgb<u8>) {
        if x < self.width && y < self.height {
            self.pixels[(y * self.width + x) as usize] = pixel.into();
        }
    }

    fn export_image(&self) -> ImageBuffer<Rgb<u8>, Vec<u8>> {
        let pixel_data = self
            .pixels
            .iter()
            .flat_map(|p| p.channels().to_vec())
            .collect::<Vec<u8>>();
        ImageBuffer::from_raw(self.width as u32, self.height as u32, pixel_data).unwrap()
    }

    fn upscale(&self, factor: u64) -> Self {
        let mut new_image = Self::new(self.width * factor, self.height * factor, None);
        for x in 0..self.height {
            for y in 0..self.width {
                let pixel = self.get_pixel(x, y).unwrap();

                for ix in 0..factor {
                    for iy in 0..factor {
                        new_image.set_pixel(x * factor + ix, y * factor + iy, pixel.clone());
                    }
                }
            }
        }

        new_image
    }
}

struct Drawyer<T> {
    cursor_x: u64,
    cursor_y: u64,
    image: PixelImage<T>,

    rng: StdRng,
}

impl Drawyer<Rgb<u8>> {
    fn new(width: u64, height: u64, rng: StdRng) -> Self {
        Self {
            cursor_x: 0,
            cursor_y: 0,
            image: PixelImage::new(width, height, None),
            rng,
        }
    }

    fn with_seed(width: u64, height: u64, seed: u64) -> Self {
        let rng = StdRng::seed_from_u64(seed);
        Self::new(width, height, rng)
    }

    fn rng(&mut self) -> &mut StdRng {
        &mut self.rng
    }

    fn with_image(rng: StdRng, image: PixelImage<Rgb<u8>>) -> Self {
        Self {
            cursor_x: 0,
            cursor_y: 0,
            image,
            rng,
        }
    }

    fn random_cursor(&mut self) {
        let rng = &mut self.rng;
        // Center-left region (leaves margin for stamps and corners)
        // x: [width/8, 3*width/8) - quarter-width region on left side
        let left_margin = self.image.width / 8;
        let left_center_end = 3 * self.image.width / 8;
        self.cursor_x = rng.random_range(left_margin..left_center_end);

        // Vertical center (middle half)
        // y: [height/4, 3*height/4) - keeps pattern away from top/bottom
        let top_margin = self.image.height / 4;
        let bottom_margin = 3 * self.image.height / 4;
        self.cursor_y = rng.random_range(top_margin..bottom_margin);
    }

    fn draw(&mut self, pixel: Rgb<u8>) {
        self.image.set_pixel(self.cursor_x, self.cursor_y, pixel);
    }

    fn move_cursor(&mut self, x: u64, y: u64) {
        self.cursor_x = x;
        self.cursor_y = y;
    }

    fn move_cursor_relative(&mut self, x: i32, y: i32) {
        let new_x = self.cursor_x as i32 + x;
        let new_y = self.cursor_y as i32 + y;

        // Clamp to centered region to keep pattern localized
        // Horizontal: [width/8, 3*width/8] for left-center
        // Vertical: [height/4, 3*height/4] for center
        let left_margin = self.image.width / 8;
        let right_boundary = 3 * self.image.width / 8;
        let top_margin = self.image.height / 4;
        let bottom_boundary = 3 * self.image.height / 4;

        if new_x >= 0 && new_y >= 0 {
            self.cursor_x = (new_x as u64).clamp(left_margin, right_boundary - 1);
            self.cursor_y = (new_y as u64).clamp(top_margin, bottom_boundary - 1);
        }
    }

    fn distance_from_top(&self) -> u64 {
        self.cursor_y
    }

    fn distance_from_bottom(&self) -> u64 {
        self.image.height - self.cursor_y
    }

    fn distance_from_left(&self) -> u64 {
        self.cursor_x
    }

    fn distance_from_right(&self) -> u64 {
        if self.cursor_x > self.image.width {
            println!(
                "Cursor out of bounds {} > {}",
                self.cursor_x, self.image.width
            );
        }
        self.image.width - self.cursor_x
    }
}

struct Artist<T> {
    drawyer: Drawyer<T>,
    pixel: Rgb<u8>,
}

enum Decision {
    Left,
    Right,
    Up,
    Down,
}

impl Artist<Rgb<u8>> {
    // fn new(width: u64, height: u64, seed: u64) -> Self {

    //     Self {
    //         drawyer: Drawyer::(width, height, seed),
    //         seed,
    //     }
    // }

    fn with_image(seed: [u8; 32], pixel: Rgb<u8>, image: PixelImage<Rgb<u8>>) -> Self {
        let rng = StdRng::from_seed(seed);
        Self {
            drawyer: Drawyer::with_image(rng, image),
            pixel,
        }
    }

    fn rng(&mut self) -> &mut StdRng {
        self.drawyer.rng()
    }

    fn draw(&mut self, pixel: Rgb<u8>) {
        self.drawyer.draw(pixel);
    }

    fn move_cursor(&mut self, x: u64, y: u64) {
        self.drawyer.move_cursor(x, y);
    }

    fn move_cursor_relative(&mut self, x: i32, y: i32) {
        self.drawyer.move_cursor_relative(x, y);
    }

    fn move_cursor_by_decision(&mut self, decision: Decision) {
        match decision {
            Decision::Left => self.move_cursor_relative(-1, 0),
            Decision::Right => self.move_cursor_relative(1, 0),
            Decision::Up => self.move_cursor_relative(0, -1),
            Decision::Down => self.move_cursor_relative(0, 1),
        }
    }

    fn left_probablity(&self) -> f32 {
        // Prevent going too far left (stay away from x=0 stamp area)
        let left_margin = self.drawyer.image.width / 8;
        let distance_from_margin = self.drawyer.cursor_x.saturating_sub(left_margin);

        if distance_from_margin >= self.drawyer.image.width / 8 {
            1.0
        } else {
            distance_from_margin as f32 / (self.drawyer.image.width / 8) as f32
        }
    }

    fn right_probablity(&self) -> f32 {
        // Use width/2 - margin as right boundary (half-canvas generation)
        let right_boundary = 3 * self.drawyer.image.width / 8;
        let distance_from_right = right_boundary.saturating_sub(self.drawyer.cursor_x);

        if distance_from_right >= self.drawyer.image.width / 8 {
            1.0
        } else {
            distance_from_right as f32 / (self.drawyer.image.width / 8) as f32
        }
    }

    fn up_probablity(&self) -> f32 {
        // Keep pattern away from top edge
        let top_margin = self.drawyer.image.height / 4;
        let distance_from_top = self.drawyer.cursor_y.saturating_sub(top_margin);

        if distance_from_top >= self.drawyer.image.height / 4 {
            1.0
        } else {
            distance_from_top as f32 / (self.drawyer.image.height / 4) as f32
        }
    }

    fn down_probablity(&self) -> f32 {
        // Keep pattern away from bottom edge
        let bottom_boundary = 3 * self.drawyer.image.height / 4;
        let distance_from_bottom = bottom_boundary.saturating_sub(self.drawyer.cursor_y);

        if distance_from_bottom >= self.drawyer.image.height / 4 {
            1.0
        } else {
            distance_from_bottom as f32 / (self.drawyer.image.height / 4) as f32
        }
    }

    /// Deterministic direction decision using cumulative distribution
    /// Replaces WeightedIndex to avoid heap allocation (ZK-friendly)
    fn decide_direction(&mut self) -> Decision {
        let left = self.left_probablity();
        let right = self.right_probablity();
        let up = self.up_probablity();
        let down = self.down_probablity();

        let total = left + right + up + down;
        let rand_val: f32 = self.rng().random();
        let threshold = rand_val * total;

        // Cumulative distribution function
        if threshold < left {
            Decision::Left
        } else if threshold < left + right {
            Decision::Right
        } else if threshold < left + right + up {
            Decision::Up
        } else {
            Decision::Down
        }
    }

    // mirror across x axis like rorchach test
    fn mirror(&mut self) {
        let mut new_image =
            PixelImage::new(self.drawyer.image.width, self.drawyer.image.height, None);

        for x in 0..self.drawyer.image.width / 2 {
            for y in 0..self.drawyer.image.height {
                let pixel = self.drawyer.image.get_pixel(x, y).unwrap();
                new_image.set_pixel(x, y, *pixel);

                let mirrored_x = self.drawyer.image.width - x - 1;
                new_image.set_pixel(mirrored_x, y, *pixel);
            }
        }
        self.drawyer.image = new_image;
    }

    /// Encode private key as corner stamps (like playing cards)
    /// Each corner gets 8 bytes (64 bits) encoded as an 8×8 binary grid
    /// Uses the image's color palette (foreground pixel for 1, background for 0)
    fn private_key_stamp(&mut self, pk: &[u8; 32], background: Rgb<u8>, offset: u64) {
        // Split private key into 4 chunks of 8 bytes each
        // Top-left: bytes 0-7
        // Top-right: bytes 8-15
        // Bottom-left: bytes 16-23
        // Bottom-right: bytes 24-31

        let width = self.drawyer.image.width;
        let height = self.drawyer.image.height;

        // Top-left corner
        self.stamp_corner(&pk[0..8], offset, offset, background);

        // Top-right corner
        self.stamp_corner(&pk[8..16], width - 8 - offset, offset, background);

        // Bottom-left corner
        self.stamp_corner(&pk[16..24], offset, height - 8 - offset, background);

        // Bottom-right corner
        self.stamp_corner(&pk[24..32], width - 8 - offset, height - 8 - offset, background);
    }

    /// Encode 8 bytes as an 8×8 binary grid at given corner position
    /// Each byte becomes one row of 8 pixels
    fn stamp_corner(&mut self, bytes: &[u8], start_x: u64, start_y: u64, background: Rgb<u8>) {
        for (row, &byte) in bytes.iter().enumerate() {
            for col in 0..8 {
                let bit = (byte >> (7 - col)) & 1;
                let pixel = if bit == 1 {
                    self.pixel  // Use foreground color for 1
                } else {
                    background  // Use background color for 0
                };
                self.drawyer.image.set_pixel(start_x + col, start_y + row as u64, pixel);
            }
        }
    }

    fn draw_random(&mut self, steps: u64, walks: u64) -> () {
        for _ in 0..walks {
            self.drawyer.random_cursor();
            self.drawyer.draw(self.pixel);
            for _ in 0..steps {
                let direction = self.decide_direction();
                self.move_cursor_by_decision(direction);
                self.drawyer.draw(self.pixel);
            }
        }

        self.mirror();
    }
}

fn _is_nth_bit_set(num: u64, n: u64) -> bool {
    (num & (1 << n)) != 0
}

/// Derive deterministic walk and step parameters from private key
/// Uses bytes 0-3 for walks (range 3-10) and bytes 4-7 for steps (range 100-300)
fn derive_parameters(pk: &[u8; 32]) -> (u64, u64) {
    let walks_raw = u32::from_le_bytes([pk[0], pk[1], pk[2], pk[3]]);
    let steps_raw = u32::from_le_bytes([pk[4], pk[5], pk[6], pk[7]]);

    // Map to reasonable ranges
    let walks = 3 + (walks_raw % 8) as u64;  // Range: 3-10
    let steps = 100 + (steps_raw % 201) as u64;  // Range: 100-300

    (walks, steps)
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Sets a custom config file
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
}

#[derive(Debug, Clone)]
struct RgbX(u8, u8, u8);

impl FromStr for RgbX {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts = s
            .split(',')
            .map(|s| s.parse().expect("xxx"))
            .collect::<Vec<u8>>();
        if parts.len() < 3 {
            Err("kek".to_string())
        } else {
            Ok(RgbX(parts[0], parts[1], parts[2]))
        }
    }
}

impl RgbX {
    fn to_rgb(&self) -> Rgb<u8> {
        Rgb([self.0, self.1, self.2])
    }
}


fn main() {
    let cli = Cli::parse();

    // Generate or parse private key
    let private_key: [u8; 32] = if let Some(pk_hex) = cli.private_key {
        // Parse hex string (with or without 0x prefix)
        let pk_hex = pk_hex.trim_start_matches("0x");
        hex::decode(pk_hex)
            .expect("Invalid hex string for private key")
            .try_into()
            .expect("Private key must be exactly 32 bytes")
    } else if cli.generate_key {
        // Generate new random private key
        let signer = PrivateKeySigner::random();
        let pk_bytes = signer.credential().to_bytes();
        println!("Generated private key: 0x{}", hex::encode(pk_bytes));
        println!("Address: {}", signer.address());
        pk_bytes.into()
    } else {
        eprintln!("Error: Must provide --private-key or --generate-key");
        std::process::exit(1);
    };

    // Derive walks/steps from private key (or use CLI overrides)
    let (default_walks, default_steps) = derive_parameters(&private_key);
    let walks = cli.walks.unwrap_or(default_walks);
    let steps = cli.steps.unwrap_or(default_steps);

    if cli.debug > 0 {
        println!("Using walks={}, steps={}", walks, steps);
        if cli.walks.is_none() && cli.steps.is_none() {
            println!("(derived from private key)");
        }
    }

    let pixel = cli.color.to_rgb();
    let background = cli.background.to_rgb();

    // Create artist with private key as seed
    let mut artist = Artist::with_image(
        private_key,
        pixel,
        PixelImage::new(64, 64, Some(background)),
    );

    // Generate Rorschach pattern
    artist.draw_random(steps, walks);

    // Add private key stamp (optional)
    if !cli.no_stamp {
        artist.private_key_stamp(&private_key, background, cli.stamp_offset);
    }

    let image = &artist.drawyer.image;

    // Upscale and save
    image.upscale(8).export_image().save(cli.output).unwrap();
}
