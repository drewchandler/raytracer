use std::ops::{Add, Mul};

/// An sRGB color for scene descriptions and the final image.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8
}

impl Color {
    pub fn new(r: u8, g: u8, b: u8) -> Color {
        Color { r: r, g: g, b: b }
    }

    pub fn linear(&self) -> LinearColor {
        fn decode(channel: u8) -> f64 {
            let v = channel as f64 / 255.0;
            if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
        }
        LinearColor::new(decode(self.r), decode(self.g), decode(self.b))
    }
}

/// Light is accumulated without clamping or quantizing, in linear color space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LinearColor {
    pub r: f64,
    pub g: f64,
    pub b: f64
}

impl LinearColor {
    pub fn new(r: f64, g: f64, b: f64) -> LinearColor {
        LinearColor { r: r, g: g, b: b }
    }

    pub fn black() -> LinearColor {
        LinearColor::new(0.0, 0.0, 0.0)
    }

    pub fn to_color(&self) -> Color {
        fn encode(v: f64) -> u8 {
            let v = v.max(0.0).min(1.0);
            let srgb = if v <= 0.0031308 { 12.92 * v } else { 1.055 * v.powf(1.0 / 2.4) - 0.055 };
            (srgb * 255.0 + 0.5) as u8
        }
        Color::new(encode(self.r), encode(self.g), encode(self.b))
    }
}

impl Add for LinearColor {
    type Output = LinearColor;

    fn add(self, other: LinearColor) -> LinearColor {
        LinearColor::new(self.r + other.r, self.g + other.g, self.b + other.b)
    }
}

impl Mul<f64> for LinearColor {
    type Output = LinearColor;

    fn mul(self, scalar: f64) -> LinearColor {
        LinearColor::new(self.r * scalar, self.g * scalar, self.b * scalar)
    }
}

impl Mul for LinearColor {
    type Output = LinearColor;

    fn mul(self, other: LinearColor) -> LinearColor {
        LinearColor::new(self.r * other.r, self.g * other.g, self.b * other.b)
    }
}

#[cfg(test)]
mod tests {
    use super::{Color, LinearColor};

    #[test]
    fn srgb_round_trip() {
        for v in 0..256 {
            let c = Color::new(v as u8, v as u8, v as u8);
            assert_eq!(c.linear().to_color(), c);
        }
    }

    #[test]
    fn light_and_samples_are_averaged_in_linear_space() {
        let mean = (Color::new(255, 255, 255).linear() + LinearColor::black()) * 0.5;
        assert_eq!(mean.to_color(), Color::new(188, 188, 188));
        assert_eq!(LinearColor::new(2.0, -1.0, 0.0).to_color(), Color::new(255, 0, 0));
    }
}
