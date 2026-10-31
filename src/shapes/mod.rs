pub mod sphere;
pub mod plane;
pub mod cube;
pub mod triangle;
pub mod transformed;

use super::point::Point;
use super::ray::{Ray, EPSILON};
use super::vector::Vector;

pub trait Shape {
    /// All intersections, including negative ones, are needed to track glass media.
    /// Ray directions may be unnormalized (e.g. after a nonuniform transform).
    fn intersections(&self, r: &Ray) -> Vec<f64>;
    fn normal_at(&self, point: &Point) -> Vector;

    fn intersect(&self, r: &Ray) -> Option<f64> {
        let mut nearest = None;
        for distance in self.intersections(r) {
            if distance.is_finite() && distance > EPSILON && nearest.map_or(true, |t| distance < t) {
                nearest = Some(distance);
            }
        }
        nearest
    }

    fn local_point(&self, point: &Point) -> Point { *point }
}
