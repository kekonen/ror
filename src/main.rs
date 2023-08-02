use std::{u8, marker::PhantomData, path::PathBuf, str::FromStr};

use clap::Parser;
use image::{ImageBuffer, Pixel, Primitive, Rgb};
use rand::{prelude::*, distributions};


struct PixelImage<T> {
    width: u32,
    height: u32,
    // image::Rgb<u8>
    pixels: Vec<T>,
}

impl PixelImage<Rgb<u8>> {

    fn new(width: u32, height: u32, rgb: Option<Rgb<u8>>) -> Self {
        let pixel = rgb.unwrap_or(Rgb([255, 255, 255]));
        let mut pixels: Vec<Rgb<u8>> = Vec::with_capacity((width * height) as usize);
        for _ in 0..width * height {
            pixels.push(rgb.unwrap_or(pixel));
        }
        println!("{}", pixels.len());
        Self {
            width,
            height,
            pixels,
        }
    }

    fn get_pixel(&self, x: u32, y: u32) -> Option<&Rgb<u8>> {
        if x < self.width && y < self.height {
            Some(&self.pixels[(y * self.width + x) as usize])
        } else {
            None
        }
    }

    fn set_pixel(&mut self, x: u32, y: u32, pixel: Rgb<u8>) {
        if x < self.width && y < self.height {
            self.pixels[(y * self.width + x) as usize] = pixel.into();
        }
    }

    fn export_image(&self) -> ImageBuffer<Rgb<u8>, Vec<u8>> {
        let pixel_data = self.pixels.iter().flat_map(|p| p.channels().to_vec()).collect::<Vec<u8>>();
        ImageBuffer::from_raw(self.width, self.height, pixel_data).unwrap()
    }

    fn upscale(&self, factor: u32) -> Self {
        let mut new_image = Self::new(self.width * factor, self.height * factor, None);
        for x in 0..self.height {
            for y in 0..self.width {
                let pixel = self.get_pixel(x, y).unwrap();
                
                for ix in 0..factor {
                    for iy in 0..factor {
                        new_image.set_pixel(x*factor + ix, y*factor + iy, pixel.clone());
                    }
                }
            }
        }

        new_image
    }
}

struct Drawyer<T> {
    cursor_x: u32,
    cursor_y: u32,
    image: PixelImage<T>,
}

impl Drawyer<Rgb<u8>> {
    fn new(width: u32, height: u32) -> Self {
        Self {
            cursor_x: 0,
            cursor_y: 0,
            image: PixelImage::new(width, height, None),
        }
    }

    fn random_cursor(&mut self) {
        let mut rng = rand::thread_rng();
        // self.cursor_x = rng.gen_range(0..self.image.width);
        // self.cursor_y = rng.gen_range(0..self.image.height);
        let quarter = self.image.width / 4;
        self.cursor_x = rng.gen_range(quarter..quarter * 3);
        self.cursor_y = rng.gen_range(quarter..quarter * 3);
    }

    fn draw(&mut self, pixel: Rgb<u8>) {
        self.image.set_pixel(self.cursor_x, self.cursor_y, pixel);
    }

    fn move_cursor(&mut self, x: u32, y: u32) {
        self.cursor_x = x;
        self.cursor_y = y;
    }

    fn move_cursor_relative(&mut self, x: i32, y: i32) {
        let x = self.cursor_x as i32 + x;
        let y = self.cursor_y as i32 + y;
        if x >= 0 && y >= 0 {
            self.cursor_x = x as u32;
            self.cursor_y = y as u32;
        }
    }

    fn distance_from_top(&self) -> u32 {
        self.cursor_y
    }

    fn distance_from_bottom(&self) -> u32 {
        self.image.height - self.cursor_y
    }

    fn distance_from_left(&self) -> u32 {
        self.cursor_x
    }

    fn distance_from_right(&self) -> u32 {
        if self.cursor_x > self.image.width {
            println!("Cursor out of bounds {} > {}", self.cursor_x, self.image.width);
        }
        self.image.width - self.cursor_x
    }
}



struct Artist <T> {
    drawyer: Drawyer<T>,
}

enum Decision {
    Left,
    Right,
    Up,
    Down,
}

impl Artist<Rgb<u8>> {
    fn new(width: u32, height: u32) -> Self {
        Self {
            drawyer: Drawyer::new(width, height),
        }
    }

    fn draw(&mut self, pixel: Rgb<u8>) {
        self.drawyer.draw(pixel);
    }

    fn move_cursor(&mut self, x: u32, y: u32) {
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

    fn decide_direction(&self) -> Decision {
        
        let distribution = distributions::WeightedIndex::new(&[
            self.left_probablity(),
            self.right_probablity(),
            self.up_probablity(),
            self.down_probablity(),
        ]).unwrap();

        let mut rng = rand::thread_rng();
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
        let mut new_image = PixelImage::new(self.drawyer.image.width, self.drawyer.image.height, None);
        for x in 0..self.drawyer.image.width {
            for y in 0..self.drawyer.image.height {
                let pixel = self.drawyer.image.get_pixel(x, y).unwrap();
                // new_image.set_pixel(x, self.drawyer.image.height - y - 1, *pixel);
                new_image.set_pixel(x, y, *pixel);
                if x >= self.drawyer.image.width / 2 {
                    let half_image = self.drawyer.image.width / 2;
                    let mirrored_x = x - half_image;
                    let mirrored_x = half_image - mirrored_x;
                    new_image.set_pixel(mirrored_x , y, *pixel);
                }
                
            }
        }
        self.drawyer.image = new_image;
    }
    
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
    steps: u32,

    #[arg(short, long, default_value = "5")]
    walks: u32,
}

#[derive(Debug, Clone)]
struct RgbX(u8, u8, u8);


impl FromStr for RgbX {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts = s.split(',').map(|s| s.parse().expect("xxx")).collect::<Vec<u8>>();
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

    // let mut i: PixelImage<Rgb<u8>> = PixelImage::new(64, 64, Some(Rgb([255, 255, 255])));

    // i.set_pixel(32, 32, Rgb((0, 0, 0).into()));
    // i.export_image().save("test1.png").unwrap();
    // define image 64x64
    // let mut img:ImageBuffer<image::Rgb<u8>, Vec<_>> = image::ImageBuffer::new(64, 64);



    // // define color black and white
    // let black = image::Rgb([0, 0, 0]);
    // let white = image::Rgb([255, 255, 255]);

    // let mut rng = rand::thread_rng();

    let cli = Cli::parse();

    // let mut artist: Artist<image::Rgb<u8>> = Artist::new(64, 64);

    let mut artist = Artist {
        drawyer: Drawyer { cursor_x: 0, cursor_y: 0, image: PixelImage::new(64, 64, Some(cli.background.to_rgb())) }
    };

    let pixel = cli.color.to_rgb();

    
    let steps = 200;
    let walks = 5;

    for _ in 0..walks {
        artist.drawyer.random_cursor();
        artist.drawyer.draw(pixel);
        for _ in 0..steps {
            let direction = artist.decide_direction();
            artist.move_cursor_by_decision(direction);
            artist.drawyer.draw(pixel);
        }
    }

    artist.mirror();

    let image = artist.drawyer.image;
    image.export_image().save("./test.png").unwrap();
    
    image.upscale(8).export_image().save(cli.output).unwrap();

}



// fn main() {
    
// }