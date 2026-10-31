use std::ops::{Add, Sub};
use super::vector::Vector;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
    pub z: f64
}

impl Add<Vector> for Point {
    type Output = Point;

    fn add(self, v: Vector) -> Point {
        Point::new(self.x + v.dx, self.y + v.dy, self.z + v.dz)
    }
}

impl Sub for Point {
    type Output = Vector;

    fn sub(self, other: Point) -> Vector {
        Vector::new(self.x - other.x, self.y - other.y, self.z - other.z)
    }
}

impl Point {
    pub fn new(x: f64, y: f64, z: f64) -> Point {
        Point { x: x, y: y, z: z }
    }
}

