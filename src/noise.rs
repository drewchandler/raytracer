use super::vector::Vector;

pub fn hash(mut x: u32) -> u32 {
    x = (x ^ (x >> 16)).wrapping_mul(0x7feb352d);
    x = (x ^ (x >> 15)).wrapping_mul(0x846ca68b);
    x ^ (x >> 16)
}

pub fn random(seed: u32) -> f64 {
    (hash(seed) as f64 + 0.5) / 4294967296.0
}

/// Seeded 3D gradient noise with quintic interpolation (C2 at lattice boundaries).
#[derive(Clone, Copy, Debug)]
pub struct Perlin {
    pub seed: u32,
}

impl Perlin {
    pub fn new(seed: u32) -> Perlin {
        Perlin { seed: seed }
    }

    pub fn sample(&self, point: Vector) -> f64 {
        let p = Vector::new(point.dx % 65536.0, point.dy % 65536.0, point.dz % 65536.0);
        assert!(p.dx.is_finite() && p.dy.is_finite() && p.dz.is_finite());
        let x = p.dx.floor() as i32;
        let y = p.dy.floor() as i32;
        let z = p.dz.floor() as i32;
        let f = p - Vector::new(x as f64, y as f64, z as f64);
        let fade = |t: f64| t * t * t * (t * (t * 6.0 - 15.0) + 10.0);
        let w = Vector::new(fade(f.dx), fade(f.dy), fade(f.dz));
        let gradients = [
            Vector::new(1.0, 1.0, 0.0),
            Vector::new(-1.0, 1.0, 0.0),
            Vector::new(1.0, -1.0, 0.0),
            Vector::new(-1.0, -1.0, 0.0),
            Vector::new(1.0, 0.0, 1.0),
            Vector::new(-1.0, 0.0, 1.0),
            Vector::new(1.0, 0.0, -1.0),
            Vector::new(-1.0, 0.0, -1.0),
            Vector::new(0.0, 1.0, 1.0),
            Vector::new(0.0, -1.0, 1.0),
            Vector::new(0.0, 1.0, -1.0),
            Vector::new(0.0, -1.0, -1.0),
        ];
        let mut sum = 0.0;
        for k in 0..2 {
            for j in 0..2 {
                for i in 0..2 {
                    let h = hash(
                        self.seed
                            ^ (((x + i as i32) as u32) & 65535).wrapping_mul(374761393)
                            ^ (((y + j as i32) as u32) & 65535).wrapping_mul(668265263)
                            ^ (((z + k as i32) as u32) & 65535).wrapping_mul(2246822519),
                    );
                    let offset = f - Vector::new(i as f64, j as f64, k as f64);
                    let weight = (if i == 0 { 1.0 - w.dx } else { w.dx })
                        * (if j == 0 { 1.0 - w.dy } else { w.dy })
                        * (if k == 0 { 1.0 - w.dz } else { w.dz });
                    sum += gradients[h as usize % 12].dot(&offset) * weight;
                }
            }
        }
        sum.max(-1.0).min(1.0)
    }

    pub fn fbm(&self, mut p: Vector, octaves: u32) -> f64 {
        assert!(octaves > 0 && octaves <= 10);
        let mut sum = 0.0;
        let mut total = 0.0;
        let mut weight = 0.5;
        for octave in 0..octaves {
            sum += Perlin::new(self.seed.wrapping_add(octave * 1013)).sample(p) * weight;
            total += weight;
            weight *= 0.5;
            p = p.scale(2.03);
        }
        sum / total
    }

    pub fn turbulence(&self, mut p: Vector, octaves: u32) -> f64 {
        assert!(octaves > 0 && octaves <= 10);
        let mut sum = 0.0;
        let mut total = 0.0;
        let mut weight = 0.5;
        for octave in 0..octaves {
            sum += Perlin::new(self.seed.wrapping_add(octave * 1013))
                .sample(p)
                .abs()
                * weight;
            total += weight;
            weight *= 0.5;
            p = p.scale(2.03);
        }
        sum / total
    }

    pub fn gradient(&self, p: Vector) -> Vector {
        let e = 0.001;
        let dx = Vector::new(e, 0.0, 0.0);
        let dy = Vector::new(0.0, e, 0.0);
        let dz = Vector::new(0.0, 0.0, e);
        Vector::new(
            self.sample(p + dx) - self.sample(p - dx),
            self.sample(p + dy) - self.sample(p - dy),
            self.sample(p + dz) - self.sample(p - dz),
        )
        .scale(0.5 / e)
    }
}

#[cfg(test)]
mod tests {
    use super::super::vector::Vector;
    use super::Perlin;
    #[test]
    fn seeded_gradient_noise_and_turbulence() {
        let n = Perlin::new(17);
        let p = Vector::new(0.31, -1.27, 2.41);
        assert_eq!(n.sample(Vector::new(1.0, -2.0, 3.0)), 0.0);
        assert_eq!(n.sample(p), Perlin::new(17).sample(p));
        assert!((n.sample(p) - Perlin::new(23).sample(p)).abs() > 0.001);
        for i in -50..50 {
            let q = Vector::new(i as f64 * 0.17, 0.27, -0.41);
            assert!(n.fbm(q, 6).abs() <= 1.0);
            assert!(n.turbulence(q, 6) >= 0.0);
        }
    }
    #[test]
    fn gradients_are_continuous_at_cell_boundaries() {
        let n = Perlin::new(2);
        let p = Vector::new(1.0, 0.37, -0.22);
        let e = Vector::new(0.00001, 0.0, 0.0);
        assert!((n.sample(p + e) - n.sample(p - e)).abs() < 0.001);
        assert!((n.gradient(p + e) - n.gradient(p - e)).magnitude() < 0.001);
    }
}
