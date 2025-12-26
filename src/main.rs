use std::{marker::PhantomData, path::PathBuf, str::FromStr, u8};

use clap::Parser;
use image::{ImageBuffer, Pixel, Primitive, Rgb};
use rand::distr::weighted::WeightedIndex;
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
        let mut rng = StdRng::seed_from_u64(seed);
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
        // self.cursor_x = rng.gen_range(0..self.image.width);
        // self.cursor_y = rng.gen_range(0..self.image.height);
        let quarter = self.image.width / 4;
        self.cursor_x = rng.gen_range(quarter..quarter * 3);
        self.cursor_y = rng.gen_range(quarter..quarter * 3);
    }

    fn draw(&mut self, pixel: Rgb<u8>) {
        self.image.set_pixel(self.cursor_x, self.cursor_y, pixel);
    }

    fn move_cursor(&mut self, x: u64, y: u64) {
        self.cursor_x = x;
        self.cursor_y = y;
    }

    fn move_cursor_relative(&mut self, x: i32, y: i32) {
        let x = self.cursor_x as i32 + x;
        let y = self.cursor_y as i32 + y;
        if x >= 0 && y >= 0 {
            self.cursor_x = x as u64;
            self.cursor_y = y as u64;
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
    seed: [u8; 32],
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
        let mut rng = StdRng::from_seed(seed);
        Self {
            drawyer: Drawyer::with_image(rng, image),
            seed,
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
        // let distance = self.drawyer.distance_from_left() as f32 / self.drawyer.image.width as f32
        if self.drawyer.distance_from_left() >= self.drawyer.image.width / 4 {
            1.0
        } else {
            self.drawyer.distance_from_left() as f32 / self.drawyer.image.width as f32
        }
    }

    fn right_probablity(&self) -> f32 {
        // self.drawyer.distance_from_right() as f32 / self.drawyer.image.width as f32
        if self.drawyer.distance_from_right() >= self.drawyer.image.width / 4 {
            1.0
        } else {
            self.drawyer.distance_from_right() as f32 / self.drawyer.image.width as f32
        }
    }

    fn up_probablity(&self) -> f32 {
        // self.drawyer.distance_from_top() as f32 / self.drawyer.image.height as f32
        if self.drawyer.distance_from_top() >= self.drawyer.image.height / 4 {
            1.0
        } else {
            self.drawyer.distance_from_top() as f32 / self.drawyer.image.height as f32
        }
    }

    fn down_probablity(&self) -> f32 {
        // self.drawyer.distance_from_bottom() as f32 / self.drawyer.image.height as f32
        if self.drawyer.distance_from_bottom() >= self.drawyer.image.height / 4 {
            1.0
        } else {
            self.drawyer.distance_from_bottom() as f32 / self.drawyer.image.height as f32
        }
    }

    fn decide_direction(&mut self) -> Decision {
        let distribution = WeightedIndex::new(&[
            self.left_probablity(),
            self.right_probablity(),
            self.up_probablity(),
            self.down_probablity(),
        ])
        .unwrap();

        let mut rng = &mut self.drawyer.rng;
        let decision = match distribution.sample(&mut rng) {
            0 => Decision::Left,
            1 => Decision::Right,
            2 => Decision::Up,
            3 => Decision::Down,
            _ => panic!("Invalid decision"),
        };

        decision
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

    fn stamp(&mut self, offset: u64) -> () {
        let corners = x(self.seed);

        let width = self.drawyer.image.width - 1;

        let side_modifier = [(false, false), (false, true), (true, false), (true, true)];

        for (i, corner) in corners.iter().enumerate() {
            let a = side_modifier[i].0;
            let b = side_modifier[i].1;
            for (x, y) in corner {
                let x = x + offset;
                let y = y + offset;
                let x = if a { x } else { width - x };
                let y = if b { y } else { width - y };
                self.move_cursor(x, y);
                self.drawyer.draw(self.pixel);
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

    #[arg(short, long, default_value = "200")]
    steps: u64,

    #[arg(short, long, default_value = "5")]
    walks: u64,

    #[arg(long)]
    seed: Option<u64>,
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

fn u64_to_bool_list(mut num: u64) -> Vec<bool> {
    let mut bits = Vec::with_capacity(64);
    for _ in 0..64 {
        bits.push(num & 1 == 1);
        num >>= 1;
    }
    bits
}

fn transpose(data: [u8; 32]) -> [u64; 4] {
    let mut result = [0u64; 4];

    for i in 0..4 {
        let offset = i * 8;
        result[i] = ((data[offset] as u64) << 56)
            | ((data[offset + 1] as u64) << 48)
            | ((data[offset + 2] as u64) << 40)
            | ((data[offset + 3] as u64) << 32)
            | ((data[offset + 4] as u64) << 24)
            | ((data[offset + 5] as u64) << 16)
            | ((data[offset + 6] as u64) << 8)
            | (data[offset + 7] as u64);
    }

    result
}

fn reverse_transpose(data: [u64; 4]) -> [u8; 32] {
    let mut result = [0u8; 32];

    for i in 0..4 {
        let offset = i * 8;
        result[offset] = (data[i] >> 56) as u8;
        result[offset + 1] = (data[i] >> 48) as u8;
        result[offset + 2] = (data[i] >> 40) as u8;
        result[offset + 3] = (data[i] >> 32) as u8;
        result[offset + 4] = (data[i] >> 24) as u8;
        result[offset + 5] = (data[i] >> 16) as u8;
        result[offset + 6] = (data[i] >> 8) as u8;
        result[offset + 7] = data[i] as u8;
    }

    result
}

fn draw(val: u64) -> Vec<(u64, u64)> {
    let mut v = Vec::new();
    let mut p = 0u64;
    let mut x = 0u64;
    let mut y = 0u64;
    let mut clockwise: bool = true;

    for b in u64_to_bool_list(val).into_iter() {
        if b {
            v.push((x, y));
        }

        if (x == 0 && y == p && clockwise) || (x == p && y == 0 && !clockwise) {
            p += 1;
            clockwise = !clockwise;
            if x == 0 {
                y = p
            } else {
                x = p
            }
        } else if y == x {
            if clockwise {
                x -= 1;
            } else {
                y -= 1;
            }
        } else if x == p {
            if clockwise {
                y += 1;
            } else {
                y -= 1;
            }
        } else if y == p {
            if clockwise {
                x -= 1;
            } else {
                x += 1;
            }
        }
    }
    v
}

fn x(val: [u8; 32]) -> [Vec<(u64, u64)>; 4] {
    let mut corners: [Vec<(u64, u64)>; 4] = [vec![], vec![], vec![], vec![]];
    let val = transpose(val);

    corners[0] = draw(val[0]);
    corners[1] = draw(val[1]);
    corners[2] = draw(val[2]);
    corners[3] = draw(val[3]);
    corners
}

fn main() {
    let cli = Cli::parse();

    let seed: [u64; 4] = if let Some(seed) = cli.seed {
        [seed, seed, seed, seed]
    } else {
        rand::rng().random()
    };

    let rng_seed = reverse_transpose(seed);

    let pixel = cli.color.to_rgb();

    let mut artist = Artist::with_image(
        rng_seed,
        pixel,
        PixelImage::new(64, 64, Some(cli.background.to_rgb())),
    );

    artist.draw_random(cli.steps, cli.walks);

    artist.stamp(4);

    let image = &artist.drawyer.image;

    image.upscale(8).export_image().save(cli.output).unwrap();
}
