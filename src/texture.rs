use std::f64::consts::PI;
use std::path::Path;
use image::{self, ImageResult, RgbImage};
use super::color::{Color, LinearColor};
use super::vector::Vector;

fn mix(a: LinearColor, b: LinearColor, t: f64) -> LinearColor {
    a * (1.0 - t) + b * t
}

fn smoothstep(low: f64, high: f64, value: f64) -> f64 {
    let t = ((value - low) / (high - low)).max(0.0).min(1.0);
    t * t * (3.0 - 2.0 * t)
}

/// An image decoded into linear light once, with bilinear UV sampling.
/// Horizontal wrapping makes an equirectangular panorama's seam continuous.
pub struct ImageTexture {
    width: u32,
    height: u32,
    pixels: Vec<LinearColor>
}

impl ImageTexture {
    pub fn load<P: AsRef<Path>>(path: P) -> ImageResult<ImageTexture> {
        image::open(path).map(|image| ImageTexture::from_image(image.to_rgb()))
    }

    pub fn from_image(image: RgbImage) -> ImageTexture {
        let (width, height) = image.dimensions();
        assert!(width > 0 && height > 0);
        let pixels = image.pixels().map(|pixel| Color::new(pixel.data[0], pixel.data[1], pixel.data[2]).linear()).collect();
        ImageTexture { width: width, height: height, pixels: pixels }
    }

    pub fn sample(&self, u: f64, v: f64) -> LinearColor {
        assert!(u.is_finite() && v.is_finite());
        let x = (u - u.floor()) * self.width as f64 - 0.5;
        let y = (v.max(0.0).min(1.0) * self.height as f64 - 0.5).max(0.0).min((self.height - 1) as f64);
        let x0 = x.floor() as i64;
        let y0 = y.floor() as u32;
        let y1 = ::std::cmp::min(y0 + 1, self.height - 1);
        let wrap = |i: i64| ((i % self.width as i64 + self.width as i64) % self.width as i64) as usize;
        let at = |i: i64, j: u32| self.pixels[j as usize * self.width as usize + wrap(i)];
        let top = mix(at(x0, y0), at(x0 + 1, y0), x - x.floor());
        let bottom = mix(at(x0, y1), at(x0 + 1, y1), x - x.floor());
        mix(top, bottom, y - y.floor())
    }
}

/// A deterministic, continuous cloud texture over the sky dome.
/// Fractal value noise provides broad cloud banks and smaller billowing edges.
pub struct CloudSky {
    seed: u32
}

impl CloudSky {
    pub fn new(seed: u32) -> CloudSky { CloudSky { seed: seed } }

    fn lattice(&self, x: i32, y: i32, z: i32, octave: u32) -> f64 {
        let mut h = self.seed.wrapping_add(octave.wrapping_mul(1013));
        h ^= (x as u32).wrapping_mul(374761393);
        h ^= (y as u32).wrapping_mul(668265263);
        h ^= (z as u32).wrapping_mul(2246822519);
        h = (h ^ (h >> 13)).wrapping_mul(1274126177);
        (h ^ (h >> 16)) as f64 / 4294967295.0
    }

    fn noise(&self, point: Vector, octave: u32) -> f64 {
        let x = point.dx.floor() as i32;
        let y = point.dy.floor() as i32;
        let z = point.dz.floor() as i32;
        let u = smoothstep(0.0, 1.0, point.dx - x as f64);
        let v = smoothstep(0.0, 1.0, point.dy - y as f64);
        let w = smoothstep(0.0, 1.0, point.dz - z as f64);
        let lerp = |a: f64, b: f64, t: f64| a * (1.0 - t) + b * t;
        let a = lerp(self.lattice(x, y, z, octave), self.lattice(x + 1, y, z, octave), u);
        let b = lerp(self.lattice(x, y + 1, z, octave), self.lattice(x + 1, y + 1, z, octave), u);
        let c = lerp(self.lattice(x, y, z + 1, octave), self.lattice(x + 1, y, z + 1, octave), u);
        let d = lerp(self.lattice(x, y + 1, z + 1, octave), self.lattice(x + 1, y + 1, z + 1, octave), u);
        lerp(lerp(a, b, v), lerp(c, d, v), w)
    }

    fn fractal(&self, mut point: Vector, octaves: u32) -> f64 {
        let mut sum = 0.0;
        let mut weight = 0.5;
        let mut total = 0.0;
        for octave in 0..octaves {
            sum += self.noise(point, octave) * weight;
            total += weight;
            point = point.scale(2.03);
            weight *= 0.5;
        }
        sum / total
    }

    fn sample(&self, direction: &Vector) -> LinearColor {
        let height = direction.dy.max(0.0);
        let horizon = Color::new(191, 218, 241).linear();
        let zenith = Color::new(30, 88, 173).linear();
        let clear = mix(horizon, zenith, height.powf(0.42));
        if direction.dy <= 0.0 { return horizon; }
        // Sampling in 3D keeps the sky continuous across the longitude seam and poles.
        let point = Vector::new(direction.dx * 4.0 + 7.1, direction.dy * 8.0 + 2.3, direction.dz * 4.0 - 3.7);
        let density = self.fractal(point, 5);
        let sun = Vector::new(-0.45, 0.55, -0.7).normalize();
        let light_density = self.fractal(point + sun.scale(0.22), 3);
        let illumination = (0.8 + (density - light_density) * 2.5).max(0.25).min(1.0);
        let cloud = mix(Color::new(133, 157, 188).linear(), Color::new(255, 252, 244).linear(), illumination);
        let opacity = smoothstep(0.43, 0.63, density) * smoothstep(0.0, 0.12, height);
        let glow = Color::new(255, 236, 201).linear() * (direction.dot(&sun).max(0.0).powf(32.0) * 0.1);
        mix(clear + glow, cloud, opacity)
    }
}

enum SkyTexture {
    Clouds(CloudSky),
    Panorama(ImageTexture)
}

/// An infinitely distant environment, visible on misses of any recursive ray.
pub struct Environment {
    texture: SkyTexture,
    rotation: f64
}

impl Environment {
    pub fn cloudy(seed: u32) -> Environment {
        Environment { texture: SkyTexture::Clouds(CloudSky::new(seed)), rotation: 0.0 }
    }

    pub fn panorama(texture: ImageTexture) -> Environment {
        Environment { texture: SkyTexture::Panorama(texture), rotation: 0.0 }
    }

    pub fn load<P: AsRef<Path>>(path: P) -> ImageResult<Environment> {
        ImageTexture::load(path).map(Environment::panorama)
    }

    pub fn set_rotation(&mut self, degrees: f64) {
        assert!(degrees.is_finite());
        self.rotation = (degrees % 360.0) * PI / 180.0;
    }

    pub fn sample(&self, direction: &Vector) -> LinearColor {
        let direction = direction.normalize();
        let (s, c) = self.rotation.sin_cos();
        let direction = Vector::new(c * direction.dx + s * direction.dz, direction.dy, -s * direction.dx + c * direction.dz);
        match self.texture {
            SkyTexture::Clouds(ref sky) => sky.sample(&direction),
            SkyTexture::Panorama(ref image) => {
                let u = 0.5 + direction.dz.atan2(direction.dx) / (2.0 * PI);
                let v = direction.dy.max(-1.0).min(1.0).acos() / PI;
                image.sample(u, v)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use image::{ImageBuffer, Rgb};
    use super::{CloudSky, Environment, ImageTexture};
    use super::super::color::{Color, LinearColor};
    use super::super::vector::Vector;

    fn stripe() -> ImageTexture {
        ImageTexture::from_image(ImageBuffer::from_fn(2, 2, |x, _| if x == 0 { Rgb([255, 255, 255]) } else { Rgb([0, 0, 0]) }))
    }

    #[test]
    fn image_filtering_averages_light_before_srgb_encoding() {
        let texture = stripe();
        assert_eq!(texture.sample(0.25, 0.5).to_color(), Color::new(255, 255, 255));
        assert_eq!(texture.sample(0.75, 0.5), LinearColor::black());
        assert_eq!(texture.sample(0.5, 0.5).to_color(), Color::new(188, 188, 188));
    }

    #[test]
    fn image_longitude_wraps_in_both_directions() {
        let texture = stripe();
        assert_eq!(texture.sample(0.0, 0.5), texture.sample(1.0, 0.5));
        assert_eq!(texture.sample(-0.25, 0.5), texture.sample(0.75, 0.5));
        assert_eq!(texture.sample(1.25, 0.5), texture.sample(0.25, 0.5));
    }

    #[test]
    fn image_poles_clamp_instead_of_wrapping() {
        let texture = ImageTexture::from_image(ImageBuffer::from_fn(1, 2, |_, y| if y == 0 { Rgb([255, 0, 0]) } else { Rgb([0, 0, 255]) }));
        assert_eq!(texture.sample(0.5, -1.0).to_color(), Color::new(255, 0, 0));
        assert_eq!(texture.sample(0.5, 2.0).to_color(), Color::new(0, 0, 255));
        assert_eq!(texture.sample(0.5, 0.5), LinearColor::new(0.5, 0.0, 0.5));
    }

    #[test]
    fn panorama_maps_directions_and_rotates() {
        let mut sky = Environment::panorama(stripe());
        let front = Vector::new(0.0, 0.0, -1.0);
        let back = -front;
        assert_eq!(sky.sample(&front).to_color(), Color::new(255, 255, 255));
        assert_eq!(sky.sample(&back), LinearColor::black());
        sky.set_rotation(180.0);
        assert!(sky.sample(&front).r < 1.0e-10);
        assert!((sky.sample(&back).r - 1.0).abs() < 1.0e-10);
    }

    #[test]
    fn cloud_noise_is_continuous_bounded_and_seeded() {
        let sky = CloudSky::new(17);
        let p = Vector::new(1.0, 2.0, -3.0);
        let a = sky.fractal(p, 5);
        let b = sky.fractal(p + Vector::new(1.0e-7, 0.0, 0.0), 5);
        assert!(a >= 0.0 && a <= 1.0 && (a - b).abs() < 1.0e-5);
        assert_eq!(a, CloudSky::new(17).fractal(p, 5));
        assert!((a - CloudSky::new(23).fractal(p, 5)).abs() > 1.0e-4);
    }

    #[test]
    fn cloudy_environment_is_finite_at_poles_and_horizon() {
        let sky = Environment::cloudy(17);
        for direction in &[Vector::new(0.0, 1.0, 0.0), Vector::new(0.0, -1.0, 0.0), Vector::new(1.0, 0.0, 0.0)] {
            let color = sky.sample(direction);
            for value in &[color.r, color.g, color.b] { assert!(value.is_finite() && *value >= 0.0 && *value <= 1.1); }
        }
    }
}
