#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;
use alloc::vec::Vec;

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Simple RGB pixel
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Pixel {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Pixel {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Convert to image crate's Rgb type (for host-side image operations)
    #[cfg(feature = "std")]
    pub fn to_rgb_array(self) -> [u8; 3] {
        [self.r, self.g, self.b]
    }
}

/// Fixed-size image for ZK (32×64 half-canvas)
#[derive(Clone, Serialize, Deserialize)]
pub struct Image32x64 {
    // Using Vec instead of array for easier serialization
    // In ZK guest, this will be stack-allocated during generation
    pub pixels: Vec<Pixel>,
}

impl Image32x64 {
    pub fn new(background: Pixel) -> Self {
        Self {
            pixels: alloc::vec![background; 32 * 64],
        }
    }

    pub fn get_pixel(&self, x: u64, y: u64) -> Option<Pixel> {
        if x < 32 && y < 64 {
            Some(self.pixels[(y * 32 + x) as usize])
        } else {
            None
        }
    }

    pub fn set_pixel(&mut self, x: u64, y: u64, pixel: Pixel) {
        if x < 32 && y < 64 {
            self.pixels[(y * 32 + x) as usize] = pixel;
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(32 * 64 * 3);
        for pixel in &self.pixels {
            bytes.push(pixel.r);
            bytes.push(pixel.g);
            bytes.push(pixel.b);
        }
        bytes
    }
}

/// Binary image for ZK proof (1 bit per pixel)
/// Stores pixels as packed bits: 32×64 pixels = 2,048 bits = 256 bytes
/// This is 24x smaller than RGB representation!
#[derive(Clone)]
pub struct BinaryImage32x64 {
    // Back to Vec for simplicity - we'll use risc0's bytes encoding
    pub data: Vec<u8>,
}

// Manual Serialize/Deserialize to avoid Vec length prefix issues
impl Serialize for BinaryImage32x64 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Serialize as a byte array to avoid variable length encoding
        serializer.serialize_bytes(&self.data)
    }
}

impl<'de> Deserialize<'de> for BinaryImage32x64 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let data = Vec::<u8>::deserialize(deserializer)?;
        Ok(BinaryImage32x64 { data })
    }
}

impl BinaryImage32x64 {
    pub fn new() -> Self {
        Self {
            data: alloc::vec![0u8; 256], // 32×64/8 = 256 bytes
        }
    }

    pub fn set_pixel(&mut self, x: u64, y: u64, value: bool) {
        if x < 32 && y < 64 {
            let bit_index = (y * 32 + x) as usize;
            let byte_index = bit_index / 8;
            let bit_offset = bit_index % 8;

            if value {
                self.data[byte_index] |= 1 << (7 - bit_offset);
            } else {
                self.data[byte_index] &= !(1 << (7 - bit_offset));
            }
        }
    }

    pub fn get_pixel(&self, x: u64, y: u64) -> bool {
        if x < 32 && y < 64 {
            let bit_index = (y * 32 + x) as usize;
            let byte_index = bit_index / 8;
            let bit_offset = bit_index % 8;

            (self.data[byte_index] & (1 << (7 - bit_offset))) != 0
        } else {
            false
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.data.clone()
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        Self {
            data: bytes.to_vec(),
        }
    }
}

enum Direction {
    Left,
    Right,
    Up,
    Down,
}

/// Derive deterministic walk and step parameters from private key
pub fn derive_parameters(pk: &[u8; 32]) -> (u64, u64) {
    let walks_raw = u32::from_le_bytes([pk[0], pk[1], pk[2], pk[3]]);
    let steps_raw = u32::from_le_bytes([pk[4], pk[5], pk[6], pk[7]]);

    let walks = 3 + (walks_raw % 8) as u64;
    let steps = 100 + (steps_raw % 201) as u64;

    (walks, steps)
}

/// Generate Rorschach half-canvas (32×64) from private key
/// This is the core deterministic function used in ZK proof
///
/// Uses virtual 64×64 coordinate system for drawing, but stores in 32×64 by mirroring
/// coordinates at write time. This creates a cohesive centered pattern.
pub fn generate_rorschach_half(
    private_key: &[u8; 32],
    walks: u64,
    steps: u64,
    foreground: Pixel,
    background: Pixel,
) -> Image32x64 {
    const VIRTUAL_WIDTH: u64 = 64; // Virtual drawing space
    const PHYSICAL_WIDTH: u64 = 32; // Physical storage width
    const HEIGHT: u64 = 64;

    let mut rng = ChaCha8Rng::from_seed(*private_key);
    let mut image = Image32x64::new(background);

    // Generate centered pattern using virtual 64-wide coordinate system
    for _ in 0..walks {
        // Random starting position in center region (virtual coordinates)
        let left_margin = VIRTUAL_WIDTH / 4; // 16
        let right_boundary = 3 * VIRTUAL_WIDTH / 4; // 48
        let top_margin = HEIGHT / 4; // 16
        let bottom_margin = 3 * HEIGHT / 4; // 48

        let mut cursor_x = rng.gen_range(left_margin..right_boundary);
        let mut cursor_y = rng.gen_range(top_margin..bottom_margin);

        // Draw starting pixel (with coordinate transformation)
        let physical_x = if cursor_x >= PHYSICAL_WIDTH {
            VIRTUAL_WIDTH - cursor_x - 1
        } else {
            cursor_x
        };
        image.set_pixel(physical_x, cursor_y, foreground);

        // Random walk
        for _ in 0..steps {
            let direction = decide_direction_fixed(&mut rng, cursor_x, cursor_y);

            // Move cursor (in virtual space)
            match direction {
                Direction::Left => {
                    if cursor_x > left_margin {
                        cursor_x -= 1;
                    }
                }
                Direction::Right => {
                    if cursor_x < right_boundary - 1 {
                        cursor_x += 1;
                    }
                }
                Direction::Up => {
                    if cursor_y > top_margin {
                        cursor_y -= 1;
                    }
                }
                Direction::Down => {
                    if cursor_y < bottom_margin - 1 {
                        cursor_y += 1;
                    }
                }
            }

            // Draw pixel (with coordinate transformation)
            let physical_x = if cursor_x >= PHYSICAL_WIDTH {
                VIRTUAL_WIDTH - cursor_x - 1
            } else {
                cursor_x
            };
            image.set_pixel(physical_x, cursor_y, foreground);
        }
    }

    image
}

/// Generate binary Rorschach pattern (for ZK proof)
/// Returns only which pixels are foreground (true) vs background (false)
/// This is 24x more efficient than RGB for ZK circuits!
pub fn generate_rorschach_binary(
    private_key: &[u8; 32],
    walks: u64,
    steps: u64,
) -> BinaryImage32x64 {
    const VIRTUAL_WIDTH: u64 = 64; // Virtual drawing space
    const PHYSICAL_WIDTH: u64 = 32; // Physical storage width
    const HEIGHT: u64 = 64;

    let mut rng = ChaCha8Rng::from_seed(*private_key);
    let mut image = BinaryImage32x64::new(); // All pixels start as false (background)

    // Generate centered pattern using virtual 64-wide coordinate system
    for _ in 0..walks {
        // Random starting position in center region (virtual coordinates)
        let left_margin = VIRTUAL_WIDTH / 4; // 16
        let right_boundary = 3 * VIRTUAL_WIDTH / 4; // 48
        let top_margin = HEIGHT / 4; // 16
        let bottom_margin = 3 * HEIGHT / 4; // 48

        let mut cursor_x = rng.gen_range(left_margin..right_boundary);
        let mut cursor_y = rng.gen_range(top_margin..bottom_margin);

        // Draw starting pixel (with coordinate transformation)
        let physical_x = if cursor_x >= PHYSICAL_WIDTH {
            VIRTUAL_WIDTH - cursor_x - 1
        } else {
            cursor_x
        };
        image.set_pixel(physical_x, cursor_y, true); // true = foreground

        // Random walk
        for _ in 0..steps {
            let direction = decide_direction_fixed(&mut rng, cursor_x, cursor_y);

            // Move cursor (in virtual space)
            match direction {
                Direction::Left => {
                    if cursor_x > left_margin {
                        cursor_x -= 1;
                    }
                }
                Direction::Right => {
                    if cursor_x < right_boundary - 1 {
                        cursor_x += 1;
                    }
                }
                Direction::Up => {
                    if cursor_y > top_margin {
                        cursor_y -= 1;
                    }
                }
                Direction::Down => {
                    if cursor_y < bottom_margin - 1 {
                        cursor_y += 1;
                    }
                }
            }

            // Draw pixel (with coordinate transformation)
            let physical_x = if cursor_x >= PHYSICAL_WIDTH {
                VIRTUAL_WIDTH - cursor_x - 1
            } else {
                cursor_x
            };
            image.set_pixel(physical_x, cursor_y, true);
        }
    }

    image
}

/// Convert binary image to RGB with specified colors
/// This is done on the host side after proof generation
pub fn binary_to_rgb(
    binary: &BinaryImage32x64,
    foreground: Pixel,
    background: Pixel,
) -> Image32x64 {
    let mut image = Image32x64::new(background);

    for y in 0..64 {
        for x in 0..32 {
            if binary.get_pixel(x, y) {
                image.set_pixel(x, y, foreground);
            }
        }
    }

    image
}

/// Deterministic direction decision using fixed-point arithmetic (no f32)
/// Uses u32 instead of f32 for ZK efficiency
/// Now uses VIRTUAL_WIDTH for probability calculations to work with virtual coordinate system
fn decide_direction_fixed(rng: &mut ChaCha8Rng, cursor_x: u64, cursor_y: u64) -> Direction {
    const VIRTUAL_WIDTH: u64 = 64; // Virtual coordinate space
    const HEIGHT: u64 = 64;
    const SCALE: u32 = 1_000_000; // Fixed-point scale

    let left_margin = VIRTUAL_WIDTH / 4; // 16
    let right_boundary = 3 * VIRTUAL_WIDTH / 4; // 48
    let top_margin = HEIGHT / 4; // 16
    let bottom_boundary = 3 * HEIGHT / 4; // 48

    // Calculate probabilities as fixed-point u32 (scaled by 1,000,000)
    // Use VIRTUAL_WIDTH for distance calculations
    let distance_from_left_margin = cursor_x.saturating_sub(left_margin);
    let left_prob = if distance_from_left_margin >= VIRTUAL_WIDTH / 4 {
        SCALE
    } else {
        ((distance_from_left_margin * SCALE as u64) / (VIRTUAL_WIDTH / 4)) as u32
    };

    let distance_from_right = right_boundary.saturating_sub(cursor_x);
    let right_prob = if distance_from_right >= VIRTUAL_WIDTH / 4 {
        SCALE
    } else {
        ((distance_from_right * SCALE as u64) / (VIRTUAL_WIDTH / 4)) as u32
    };

    let distance_from_top = cursor_y.saturating_sub(top_margin);
    let up_prob = if distance_from_top >= HEIGHT / 4 {
        SCALE
    } else {
        ((distance_from_top * SCALE as u64) / (HEIGHT / 4)) as u32
    };

    let distance_from_bottom = bottom_boundary.saturating_sub(cursor_y);
    let down_prob = if distance_from_bottom >= HEIGHT / 4 {
        SCALE
    } else {
        ((distance_from_bottom * SCALE as u64) / (HEIGHT / 4)) as u32
    };

    let total = left_prob + right_prob + up_prob + down_prob;
    let rand_val = rng.gen::<u32>() % total;

    // Cumulative distribution
    if rand_val < left_prob {
        Direction::Left
    } else if rand_val < left_prob + right_prob {
        Direction::Right
    } else if rand_val < left_prob + right_prob + up_prob {
        Direction::Up
    } else {
        Direction::Down
    }
}
