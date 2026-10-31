use super::color::Color;
use super::material::Material;
use super::point::Point;
use super::ray::Ray;
use super::shapes::Shape;

pub struct Object {
    pub shape: Box<Shape>,
    pub color: Color,
    pub material: Material
}

#[cfg(test)]
mod tests {
    use super::Object;
    use super::super::color::Color;
    use super::super::material::{Material, Pattern};
    use super::super::point::Point;
    use super::super::ray::Ray;
    use super::super::shapes::sphere::Sphere;
    use super::super::shapes::transformed::Transformed;
    use super::super::transform::Transform;
    use super::super::vector::Vector;

    #[test]
    fn object_preserves_shape_intersection_entry_point() {
        let object = Object::new(Box::new(Sphere::new(Point::new(0.0, 0.0, 0.0), 1.0)), Color::new(255, 0, 0));
        let ray = Ray::new(Point::new(0.0, 0.0, -3.0), Vector::new(0.0, 0.0, 1.0));
        assert_eq!(object.intersect(&ray), Some(2.0));
    }

    #[test]
    fn patterns_follow_shape_scaling_and_translation() {
        let white = Color::new(255, 255, 255);
        let black = Color::new(0, 0, 0);
        let shape = Transformed::new(Box::new(Sphere::new(Point::new(0.0, 0.0, 0.0), 1.0)),
            Transform::identity().scale(2.0, 2.0, 2.0).translate(1.0, 0.0, 0.0));
        let material = Material { pattern: Some(Pattern::Stripes(white, black, 1.0)), ..Material::default() };
        let object = Object::with_material(Box::new(shape), black, material);
        assert_eq!(object.color_at(&Point::new(2.5, 0.0, 0.0)), white);
        assert_eq!(object.color_at(&Point::new(3.5, 0.0, 0.0)), black);
    }
}

impl Object {
    pub fn new(shape: Box<Shape>, color: Color) -> Object {
        Object::with_material(shape, color, Material::default())
    }

    pub fn with_material(shape: Box<Shape>, color: Color, material: Material) -> Object {
        material.validate();
        Object { shape: shape, color: color, material: material }
    }

    pub fn intersect(&self, r: &Ray) -> Option<f64> {
        self.shape.intersect(&r)
    }

    pub fn color_at(&self, point: &Point) -> Color {
        match self.material.pattern {
            Some(pattern) => pattern.color_at(&self.shape.local_point(point)),
            None => self.color
        }
    }
}
