use super::Shape;
use super::super::point::Point;
use super::super::ray::Ray;
use super::super::transform::Transform;
use super::super::vector::Vector;

/// Adds transforms to any existing Shape without changing its geometry code.
pub struct Transformed {
    shape: Box<Shape>,
    transform: Transform
}

impl Transformed {
    pub fn new(shape: Box<Shape>, transform: Transform) -> Transformed {
        Transformed { shape: shape, transform: transform }
    }
}

impl Shape for Transformed {
    fn intersections(&self, ray: &Ray) -> Vec<f64> {
        // Do not normalize: retaining the local direction's length preserves world t.
        let local_ray = Ray::new(self.transform.inverse_point(&ray.origin), self.transform.inverse_vector(&ray.direction));
        self.shape.intersections(&local_ray)
    }

    fn normal_at(&self, point: &Point) -> Vector {
        let local_point = self.transform.inverse_point(point);
        self.transform.normal(&self.shape.normal_at(&local_point))
    }

    fn local_point(&self, point: &Point) -> Point {
        self.shape.local_point(&self.transform.inverse_point(point))
    }

    fn normal_for_ray(&self, ray: &Ray, distance: f64) -> Vector {
        let local=Ray::new(self.transform.inverse_point(&ray.origin),self.transform.inverse_vector(&ray.direction));
        self.transform.normal(&self.shape.normal_for_ray(&local,distance))
    }

    fn normal_to_world(&self, normal: &Vector) -> Vector {
        self.transform.normal_vector(&self.shape.normal_to_world(normal))
    }

    fn normals_for_ray(&self,ray: &Ray,distance: f64) -> (Vector,Vector) {
        let local=Ray::new(self.transform.inverse_point(&ray.origin),self.transform.inverse_vector(&ray.direction));
        let (geometric,shading)=self.shape.normals_for_ray(&local,distance);
        (self.transform.normal(&geometric),self.transform.normal(&shading))
    }
}

#[cfg(test)]
mod tests {
    use super::Transformed;
    use super::super::{Shape, sphere};
    use super::super::super::point::Point;
    use super::super::super::ray::Ray;
    use super::super::super::transform::Transform;
    use super::super::super::vector::Vector;

    #[test]
    fn transformed_hits_preserve_world_distance() {
        let shape = Transformed::new(Box::new(sphere::Sphere::new(Point::new(0.0, 0.0, 0.0), 1.0)),
            Transform::identity().scale(2.0, 2.0, 2.0).translate(0.0, 0.0, 1.0));
        let ray = Ray::new(Point::new(0.0, 0.0, -5.0), Vector::new(0.0, 0.0, 1.0));
        assert_eq!(shape.intersections(&ray), vec![4.0, 8.0]);
        assert_eq!(shape.local_point(&Point::new(2.0, 0.0, 1.0)), Point::new(1.0, 0.0, 0.0));
    }

    #[test]
    fn nonuniform_scale_uses_inverse_transpose_normal() {
        let shape = Transformed::new(Box::new(sphere::Sphere::new(Point::new(0.0, 0.0, 0.0), 1.0)),
            Transform::identity().scale(1.0, 0.5, 1.0));
        let k = (0.5f64).sqrt();
        let normal = shape.normal_at(&Point::new(0.0, k * 0.5, -k));
        let expected = Vector::new(0.0, 2.0, -1.0).normalize();
        assert!((normal - expected).magnitude() < 1.0e-10);
    }
}
