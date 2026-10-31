use std::f64;
use super::color::{Color, LinearColor};
use super::point::Point;
use super::vector::Vector;

pub enum Light {
    /// Point/area intensity is radiant intensity; illumination falls off as 1/r^2.
    Point { position: Point, color: Color, intensity: f64 },
    /// Direction is the direction in which the light travels (towards the scene).
    Directional { direction: Vector, color: Color, intensity: f64 },
    Area { corner: Point, u: Vector, v: Vector, steps: u32, color: Color, intensity: f64 }
}

pub struct LightSample {
    pub direction: Vector,
    pub distance: f64,
    pub radiance: LinearColor
}

impl Light {
    pub fn point(position: Point, color: Color, intensity: f64) -> Light {
        assert!(intensity >= 0.0 && intensity.is_finite());
        Light::Point { position: position, color: color, intensity: intensity }
    }

    pub fn directional(direction: Vector, color: Color, intensity: f64) -> Light {
        assert!(intensity >= 0.0 && intensity.is_finite());
        Light::Directional { direction: direction.normalize(), color: color, intensity: intensity }
    }

    /// A two-sided rectangular emitter, sampled on a deterministic steps x steps grid.
    pub fn area(corner: Point, u: Vector, v: Vector, steps: u32, color: Color, intensity: f64) -> Light {
        assert!(steps > 0 && steps <= 64, "Area light samples must be between 1 and 64 per side");
        assert!(u.cross(&v).magnitude() > 0.0);
        assert!(intensity >= 0.0 && intensity.is_finite());
        Light::Area { corner: corner, u: u, v: v, steps: steps, color: color, intensity: intensity }
    }

    fn sample(position: Point, point: &Point, color: Color, intensity: f64) -> Option<LightSample> {
        let offset = position - *point;
        let distance = offset.magnitude();
        if distance <= 1.0e-10 { return None; }
        Some(LightSample { direction: offset.scale(1.0 / distance), distance: distance,
                           radiance: color.linear() * (intensity / (distance * distance)) })
    }

    pub fn samples(&self, point: &Point) -> Vec<LightSample> {
        match *self {
            Light::Point { position, color, intensity } => Light::sample(position, point, color, intensity).into_iter().collect(),
            Light::Directional { direction, color, intensity } => vec![LightSample {
                direction: -direction.normalize(), distance: f64::INFINITY, radiance: color.linear() * intensity
            }],
            Light::Area { corner, u, v, steps, color, intensity } => {
                let mut samples = Vec::new();
                for y in 0..steps {
                    for x in 0..steps {
                        let position = corner + u.scale((x as f64 + 0.5) / steps as f64) + v.scale((y as f64 + 0.5) / steps as f64);
                        if let Some(sample) = Light::sample(position, point, color, intensity / (steps * steps) as f64) {
                            samples.push(sample);
                        }
                    }
                }
                samples
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Light;
    use super::super::color::Color;
    use super::super::point::Point;
    use super::super::vector::Vector;

    #[test]
    fn point_light_has_inverse_square_falloff() {
        let light = Light::point(Point::new(0.0, 0.0, 0.0), Color::new(255, 255, 255), 4.0);
        assert_eq!(light.samples(&Point::new(0.0, 0.0, 1.0))[0].radiance.r, 4.0);
        assert_eq!(light.samples(&Point::new(0.0, 0.0, 2.0))[0].radiance.r, 1.0);
        assert!(light.samples(&Point::new(0.0, 0.0, 0.0)).is_empty());
    }

    #[test]
    fn area_light_samples_cover_rectangle_without_changing_total_intensity() {
        let light = Light::area(Point::new(-1.0, 2.0, -1.0), Vector::new(2.0, 0.0, 0.0),
            Vector::new(0.0, 0.0, 2.0), 2, Color::new(255, 255, 255), 4.0);
        let samples = light.samples(&Point::new(0.0, 0.0, 0.0));
        assert_eq!(samples.len(), 4);
        let sum = samples.iter().fold(0.0, |sum, s| sum + s.radiance.r);
        assert!((sum - 4.0 / 4.5).abs() < 1.0e-10);
    }
}
