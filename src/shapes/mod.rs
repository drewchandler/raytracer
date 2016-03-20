pub mod sphere;

use super::ray::Ray;

pub trait Shape {
    fn intersect(&self, r: &Ray) -> Option<f64>;
}
