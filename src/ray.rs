use super::point::Point;
use super::vector::Vector;

/// Offset secondary rays and reject self intersections at the same surface.
pub const EPSILON: f64 = 1.0e-5;

#[derive(Clone, Copy, Debug)]
pub struct Ray {
    pub origin: Point,
    pub direction: Vector
}

impl Ray {
    pub fn new(origin: Point, direction: Vector) -> Ray {
        Ray {
            origin: origin,
            direction: direction
        }
    }

    pub fn at(&self, distance: f64) -> Point {
        self.origin + self.direction.scale(distance)
    }
}

