// Rorschach ZK Image Generation - SP1 Implementation
// This is the guest program that runs inside the zkVM
// It generates a deterministic Rorschach-style pattern from a private key

use tiny_keccak::{Hasher, Keccak};

/// Constants for the Rorschach pattern generation
pub const VIRTUAL_WIDTH: usize = 64;
pub const PHYSICAL_WIDTH: usize = 32;
pub const HEIGHT: usize = 64;
pub const LEFT_MARGIN: usize = VIRTUAL_WIDTH / 4;   // 16
pub const RIGHT_BOUNDARY: usize = 3 * VIRTUAL_WIDTH / 4; // 48
pub const TOP_MARGIN: usize = HEIGHT / 4;           // 16
pub const BOTTOM_BOUNDARY: usize = 3 * HEIGHT / 4;  // 48

/// ZK-friendly PRNG using Keccak256
///
/// Replaces ChaCha8Rng from the original implementation.
/// Keccak costs ~300 constraints per call vs ~20,000 for ChaCha8.
pub struct ZkPrng {
    seed: [u8; 32],
    counter: u64,
}

impl ZkPrng {
    pub fn new(seed: [u8; 32]) -> Self {
        Self { seed, counter: 0 }
    }

    /// Generate next u64 using Keccak256(seed || counter)
    pub fn next_u64(&mut self) -> u64 {
        let mut hasher = Keccak::v256();
        hasher.update(&self.seed);
        hasher.update(&self.counter.to_le_bytes());

        let mut output = [0u8; 32];
        hasher.finalize(&mut output);

        self.counter += 1;

        // Take first 8 bytes as u64
        u64::from_le_bytes([
            output[0], output[1], output[2], output[3],
            output[4], output[5], output[6], output[7],
        ])
    }

    /// Generate random number in range [min, max)
    pub fn gen_range(&mut self, min: usize, max: usize) -> usize {
        let range = (max - min) as u64;
        let random = self.next_u64();
        min + (random % range) as usize
    }
}

/// Binary image stored as 32×64 (physical width × height)
/// Each pixel is 1 bit. Left half is mirrored to create symmetry.
#[derive(Clone)]
pub struct BinaryImage32x64 {
    data: [[u8; PHYSICAL_WIDTH]; HEIGHT],
}

impl BinaryImage32x64 {
    pub fn new() -> Self {
        Self {
            data: [[0u8; PHYSICAL_WIDTH]; HEIGHT],
        }
    }

    /// Map virtual x coordinate (0-63) to physical x (0-31)
    /// Left half mirrors right half for Rorschach symmetry
    fn map_x(x: usize) -> usize {
        if x < PHYSICAL_WIDTH {
            x
        } else {
            VIRTUAL_WIDTH - 1 - x
        }
    }

    /// Set pixel at virtual coordinates (x, y)
    /// Automatically handles mirroring
    pub fn set_pixel(&mut self, x: usize, y: usize) {
        if x >= VIRTUAL_WIDTH || y >= HEIGHT {
            return;
        }
        let physical_x = Self::map_x(x);
        self.data[y][physical_x] = 1;
    }

    /// Check if pixel is set at virtual coordinates
    pub fn get_pixel(&self, x: usize, y: usize) -> bool {
        if x >= VIRTUAL_WIDTH || y >= HEIGHT {
            return false;
        }
        let physical_x = Self::map_x(x);
        self.data[y][physical_x] == 1
    }

    /// Convert to 256-byte packed format (1 bit per pixel)
    /// 64×64 = 4096 pixels = 512 bytes, but we only store 32×64 = 2048 pixels
    /// Result: 256 bytes (2048 bits)
    pub fn to_bytes(&self) -> [u8; 256] {
        let mut result = [0u8; 256];
        let mut byte_idx = 0;
        let mut bit_idx = 0;

        for y in 0..HEIGHT {
            for x in 0..PHYSICAL_WIDTH {
                if self.data[y][x] == 1 {
                    result[byte_idx] |= 1 << bit_idx;
                }
                bit_idx += 1;
                if bit_idx == 8 {
                    bit_idx = 0;
                    byte_idx += 1;
                }
            }
        }

        result
    }
}

/// Direction for random walk
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

impl Direction {
    /// Get available directions based on cursor position
    /// Returns (directions, weights) for weighted random selection
    fn get_weighted_directions(x: usize, y: usize) -> (Vec<Direction>, Vec<u32>) {
        let mut dirs = Vec::new();
        let mut weights = Vec::new();

        // Calculate distance from boundaries
        let dist_left = x.saturating_sub(LEFT_MARGIN);
        let dist_right = RIGHT_BOUNDARY.saturating_sub(x + 1);
        let dist_top = y.saturating_sub(TOP_MARGIN);
        let dist_bottom = BOTTOM_BOUNDARY.saturating_sub(y + 1);

        // Prefer moving away from boundaries
        // Weight is proportional to how much room we have
        if x > LEFT_MARGIN {
            dirs.push(Direction::Left);
            weights.push(dist_left.max(1) as u32);
        }
        if x < RIGHT_BOUNDARY - 1 {
            dirs.push(Direction::Right);
            weights.push(dist_right.max(1) as u32);
        }
        if y > TOP_MARGIN {
            dirs.push(Direction::Up);
            weights.push(dist_top.max(1) as u32);
        }
        if y < BOTTOM_BOUNDARY - 1 {
            dirs.push(Direction::Down);
            weights.push(dist_bottom.max(1) as u32);
        }

        // Fallback: if somehow no directions available, allow all
        if dirs.is_empty() {
            dirs = vec![Direction::Left, Direction::Right, Direction::Up, Direction::Down];
            weights = vec![1, 1, 1, 1];
        }

        (dirs, weights)
    }

    /// Choose random direction using weighted probabilities
    fn choose_weighted(rng: &mut ZkPrng, x: usize, y: usize) -> Direction {
        let (dirs, weights) = Self::get_weighted_directions(x, y);

        // Calculate total weight
        let total: u32 = weights.iter().sum();

        // Generate random number in range [0, total)
        let mut choice = (rng.next_u64() % total as u64) as u32;

        // Find which direction was selected
        for (i, &weight) in weights.iter().enumerate() {
            if choice < weight {
                return dirs[i];
            }
            choice -= weight;
        }

        // Fallback (should never happen)
        dirs[0]
    }

    /// Apply direction to coordinates
    fn apply(&self, x: usize, y: usize) -> (usize, usize) {
        match self {
            Direction::Left => (x.saturating_sub(1), y),
            Direction::Right => ((x + 1).min(VIRTUAL_WIDTH - 1), y),
            Direction::Up => (x, y.saturating_sub(1)),
            Direction::Down => (x, (y + 1).min(HEIGHT - 1)),
        }
    }
}

/// Hash private key to derive deterministic seed
fn hash_private_key(private_key: &[u8; 32]) -> [u8; 32] {
    let mut hasher = Keccak::v256();
    hasher.update(private_key);
    let mut output = [0u8; 32];
    hasher.finalize(&mut output);
    output
}

/// Derive generation parameters from private key hash
/// Returns (walks, steps, seed_for_rng)
pub fn derive_parameters(pk_hash: &[u8; 32]) -> (usize, usize, [u8; 32]) {
    // Use different parts of hash for different parameters
    let walks_byte = pk_hash[0];
    let steps_bytes = u16::from_le_bytes([pk_hash[1], pk_hash[2]]);

    // walks: 3-20
    let walks = 3 + (walks_byte % 18) as usize;

    // steps: 80-300
    let steps = 80 + (steps_bytes % 221) as usize;

    // Use full hash as RNG seed
    (walks, steps, *pk_hash)
}

/// Generate Rorschach pattern from private key
/// This is the main algorithm that runs in the zkVM
pub fn generate_rorschach_binary(private_key: &[u8; 32]) -> (usize, usize, BinaryImage32x64) {
    // Step 1: Hash private key
    let pk_hash = hash_private_key(private_key);

    // Step 2: Derive parameters
    let (walks, steps, seed) = derive_parameters(&pk_hash);

    // Step 3: Initialize RNG and image
    let mut rng = ZkPrng::new(seed);
    let mut image = BinaryImage32x64::new();

    // Step 4: Perform random walks
    for _ in 0..walks {
        // Random starting position within bounds
        let start_x = rng.gen_range(LEFT_MARGIN, RIGHT_BOUNDARY);
        let start_y = rng.gen_range(TOP_MARGIN, BOTTOM_BOUNDARY);

        let mut x = start_x;
        let mut y = start_y;

        // Set starting pixel
        image.set_pixel(x, y);

        // Random walk
        for _ in 0..steps {
            // Choose direction with boundary-aware weighting
            let direction = Direction::choose_weighted(&mut rng, x, y);

            // Move
            let (new_x, new_y) = direction.apply(x, y);
            x = new_x;
            y = new_y;

            // Set pixel
            image.set_pixel(x, y);
        }
    }

    (walks, steps, image)
}

/// Public output structure
#[derive(Debug, Clone)]
pub struct RorschachOutput {
    pub address: [u8; 20],      // Ethereum address derived from private key
    pub walks: u64,
    pub steps: u64,
    pub image_hash: [u8; 32],   // Keccak256 of image data
    pub image_data: [u8; 256],  // Full binary image
}

/// Generate complete output including address derivation
pub fn generate_complete(private_key: &[u8; 32]) -> RorschachOutput {
    // Generate pattern
    let (walks, steps, image) = generate_rorschach_binary(private_key);

    // Convert to bytes
    let image_data = image.to_bytes();

    // Hash image
    let mut hasher = Keccak::v256();
    hasher.update(&image_data);
    let mut image_hash = [0u8; 32];
    hasher.finalize(&mut image_hash);

    // Derive address from private key (last 20 bytes of hash)
    let pk_hash = hash_private_key(private_key);
    let mut address = [0u8; 20];
    address.copy_from_slice(&pk_hash[12..32]);

    RorschachOutput {
        address,
        walks: walks as u64,
        steps: steps as u64,
        image_hash,
        image_data,
    }
}
