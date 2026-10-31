use super::color::{Color, LinearColor};
use super::noise::Perlin;
use super::point::Point;
use super::ray::Ray;
use super::vector::Vector;

fn smooth(a: f64, b: f64, x: f64) -> f64 {
    let t = ((x - a) / (b - a)).max(0.0).min(1.0);
    t * t * (3.0 - 2.0 * t)
}

/// A world-space sky cloud layer integrated with Beer-Lambert extinction.
pub struct CloudVolume {
    pub seed: u32,
    pub time: f64,
    pub steps: u32,
    pub loop_period: f64,
}

impl CloudVolume {
    pub fn new(seed: u32, steps: u32) -> CloudVolume {
        assert!(steps >= 8 && steps <= 128);
        CloudVolume {
            seed: seed,
            time: 0.0,
            steps: steps,
            loop_period: 0.0,
        }
    }
    pub fn density(&self, p: Point) -> f64 {
        let profile = smooth(20.0, 23.0, p.y) * (1.0 - smooth(29.0, 32.0, p.y));
        if profile == 0.0 {
            return 0.0;
        }
        let noise = Perlin::new(self.seed);
        let offset=::motion::cloud_offset(self.time,self.loop_period,Vector::new(0.04,0.0,0.015));
        let q = Vector::new(
            p.x * 0.045,
            p.y * 0.16,
            p.z * 0.045,
        )+offset;
        let weather = 0.5 + 0.5 * noise.sample(Vector::new(p.x * 0.015, 0.37, p.z * 0.015));
        let shape = 0.5 + 0.5 * noise.fbm(q, 3);
        ((shape - 0.52 + (weather - 0.5) * 0.25) * 4.5)
            .max(0.0)
            .min(1.0)
            * profile
    }
    fn interval(&self, ray: &Ray) -> Option<(f64, f64)> {
        if ray.direction.dy.abs() < 1.0e-8 {
            return if ray.origin.y > 20.0 && ray.origin.y < 32.0 {
                Some((0.0, 250.0))
            } else {
                None
            };
        }
        let a = (20.0 - ray.origin.y) / ray.direction.dy;
        let b = (32.0 - ray.origin.y) / ray.direction.dy;
        let lo = a.min(b).max(0.0);
        let hi = a.max(b).min(lo + 250.0);
        if hi <= lo || lo > 1500.0 {
            None
        } else {
            Some((lo, hi))
        }
    }
    fn sunlight(&self, p: Point, sun: Vector) -> f64 {
        let length = ((32.0 - p.y) / sun.dy).max(0.0);
        let step = length / 4.0;
        let mut optical = 0.0;
        for i in 0..4 {
            optical += self.density(p + sun.scale((i as f64 + 0.5) * step)) * step * 0.45;
        }
        (-optical).exp()
    }
    pub fn sample(&self, ray: &Ray, background: LinearColor) -> LinearColor {
        self.sample_until(ray, background, ::std::f64::INFINITY)
    }

    pub fn sample_until(
        &self,
        ray: &Ray,
        background: LinearColor,
        max_distance: f64,
    ) -> LinearColor {
        let ray = Ray::new(ray.origin, ray.direction.normalize());
        let (lo, mut hi) = match self.interval(&ray) {
            Some(interval) => interval,
            None => return background,
        };
        hi = hi.min(max_distance);
        if hi <= lo {
            return background;
        }
        let step = (hi - lo) / self.steps as f64;
        let sun = Vector::new(-0.45, 0.55, -0.7).normalize();
        let cosine = ray.direction.dot(&sun);
        let g = 0.35;
        let phase = (1.0 - g * g) / (1.0 + g * g - 2.0 * g * cosine).powf(1.5);
        let mut transmittance = 1.0;
        let mut scattered = LinearColor::black();
        for i in 0..self.steps {
            let p = ray.at(lo + (i as f64 + 0.5) * step);
            let density = self.density(p);
            if density > 0.001 {
                let alpha = 1.0 - (-density * 0.45 * step).exp();
                let illumination = Color::new(119, 151, 190).linear() * 0.45
                    + Color::new(255, 247, 228).linear()
                        * (self.sunlight(p, sun) * (0.6 + 0.4 * phase));
                scattered = scattered + illumination * (transmittance * alpha);
                transmittance *= 1.0 - alpha;
                if transmittance < 0.005 {
                    break;
                }
            }
        }
        let result = scattered + background * transmittance;
        let fade = 1.0 - smooth(700.0, 1500.0, lo);
        background * (1.0 - fade) + result * fade
    }
}

#[cfg(test)]
mod tests {
    use super::super::color::LinearColor;
    use super::super::point::Point;
    use super::super::ray::Ray;
    use super::super::vector::Vector;
    use super::CloudVolume;
    #[test]
    fn layer_intervals_and_empty_rays() {
        let v = CloudVolume::new(17, 16);
        let sky = LinearColor::new(0.2, 0.4, 0.8);
        let up = Ray::new(Point::new(0.0, 0.0, 0.0), Vector::new(0.0, 1.0, 0.0));
        assert_eq!(v.interval(&up), Some((20.0, 32.0)));
        assert_eq!(v.sample(&Ray::new(up.origin, -up.direction), sky), sky);
        assert_eq!(v.density(Point::new(0.0, 19.0, 0.0)), 0.0);
    }
    #[test]
    fn volume_has_depth_self_shadow_and_continuous_animation() {
        let mut v = CloudVolume::new(17, 24);
        let mut dense = None;
        for x in -20..20 {
            for z in -20..20 {
                let p = Point::new(x as f64 * 5.0, 26.0, z as f64 * 5.0);
                if v.density(p) > 0.2 {
                    dense = Some(p);
                    break;
                }
            }
            if dense.is_some() {
                break;
            }
        }
        let p = dense.expect("cloud layer must contain density");
        let ray = Ray::new(Point::new(p.x, 0.0, p.z), Vector::new(0.0, 1.0, 0.0));
        let bg = LinearColor::new(0.0, 0.0, 1.0);
        assert_eq!(v.sample_until(&ray, bg, 10.0), bg);
        assert!(v.sample_until(&ray, bg, 40.0).r > 0.0);
        assert!(v.sunlight(p, Vector::new(0.0, 1.0, 0.0)) < 1.0);
        let a = v.density(p);
        v.time = 0.0001;
        assert!((a - v.density(p)).abs() < 0.001);
        v.time = 20.0;
        assert!((a - v.density(p)).abs() > 0.001);
    }
}
