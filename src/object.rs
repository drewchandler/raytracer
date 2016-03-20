use super::color::Color;
use super::ray::Ray;
use super::shapes::Shape;

pub struct Object {
    pub shape: Box<Shape>,
    pub color: Color
}

impl Object {
    pub fn intersect(&self, r: &Ray) -> Option<f64> {
        self.shape.intersect(&r)
    }
}
