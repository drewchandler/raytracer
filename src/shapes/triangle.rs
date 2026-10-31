use super::Shape;
use super::super::point::Point;
use super::super::ray::Ray;
use super::super::vector::Vector;

pub struct Triangle {
    a: Point,
    edge1: Vector,
    edge2: Vector,
    normal: Vector
}

impl Triangle {
    pub fn new(a: Point, b: Point, c: Point) -> Triangle {
        let edge1 = b - a;
        let edge2 = c - a;
        Triangle { a: a, edge1: edge1, edge2: edge2, normal: edge1.cross(&edge2).normalize() }
    }
}

impl Shape for Triangle {
    /// Moller-Trumbore, two-sided, with barycentric edge tests.
    fn intersections(&self, ray: &Ray) -> Vec<f64> {
        let p = ray.direction.cross(&self.edge2);
        let determinant = self.edge1.dot(&p);
        if determinant.abs() < 1.0e-10 * self.edge1.magnitude() * p.magnitude() || determinant == 0.0 {
            return vec![];
        }
        let inverse = 1.0 / determinant;
        let offset = ray.origin - self.a;
        let u = inverse * offset.dot(&p);
        if u < 0.0 || u > 1.0 { return vec![]; }
        let q = offset.cross(&self.edge1);
        let v = inverse * ray.direction.dot(&q);
        if v < 0.0 || u + v > 1.0 { return vec![]; }
        vec![inverse * self.edge2.dot(&q)]
    }

    fn normal_at(&self, _: &Point) -> Vector { self.normal }
}

#[cfg(test)]
mod tests {
    use super::Triangle;
    use super::super::Shape;
    use super::super::super::point::Point;
    use super::super::super::ray::Ray;
    use super::super::super::vector::Vector;

    #[test]
    fn triangle_barycentric_and_parallel_cases() {
        let tri = Triangle::new(Point::new(0.0, 1.0, 0.0), Point::new(-1.0, 0.0, 0.0), Point::new(1.0, 0.0, 0.0));
        assert_eq!(tri.intersect(&Ray::new(Point::new(0.0, 0.5, -2.0), Vector::new(0.0, 0.0, 1.0))), Some(2.0));
        assert_eq!(tri.intersect(&Ray::new(Point::new(0.0, 0.5, 2.0), Vector::new(0.0, 0.0, -1.0))), Some(2.0));
        for origin in &[Point::new(-1.0, 1.0, -2.0), Point::new(1.0, 1.0, -2.0), Point::new(0.0, -1.0, -2.0)] {
            assert!(tri.intersect(&Ray::new(*origin, Vector::new(0.0, 0.0, 1.0))).is_none());
        }
        assert!(tri.intersect(&Ray::new(Point::new(0.0, 0.5, -2.0), Vector::new(0.0, 1.0, 0.0))).is_none());
        assert_eq!(tri.normal_at(&Point::new(0.0, 0.5, 0.0)), Vector::new(0.0, 0.0, 1.0));
    }
}
