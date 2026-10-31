use super::Shape;
use super::super::point::Point;
use super::super::ray::Ray;
use super::super::vector::Vector;

pub struct Plane {
    point: Point,
    normal: Vector
}

impl Plane {
    pub fn new(point: Point, normal: Vector) -> Plane {
        Plane { point: point, normal: normal.normalize() }
    }
}

impl Shape for Plane {
    fn intersections(&self, ray: &Ray) -> Vec<f64> {
        let denominator = self.normal.dot(&ray.direction);
        if denominator.abs() < 1.0e-10 * ray.direction.magnitude() { return vec![]; }
        if denominator == 0.0 { return vec![]; }
        vec![(self.point - ray.origin).dot(&self.normal) / denominator]
    }

    fn normal_at(&self, _: &Point) -> Vector { self.normal }
}

#[cfg(test)]
mod tests {
    use super::Plane;
    use super::super::Shape;
    use super::super::super::point::Point;
    use super::super::super::ray::Ray;
    use super::super::super::vector::Vector;

    #[test]
    fn plane_hits_from_both_sides_and_rejects_parallel_rays() {
        let plane = Plane::new(Point::new(0.0, 0.0, 0.0), Vector::new(0.0, 2.0, 0.0));
        for sign in &[-1.0, 1.0] {
            let ray = Ray::new(Point::new(0.0, *sign, 0.0), Vector::new(0.0, -*sign, 0.0));
            assert_eq!(plane.intersect(&ray), Some(1.0));
        }
        assert_eq!(plane.normal_at(&Point::new(9.0, 0.0, 2.0)), Vector::new(0.0, 1.0, 0.0));
        assert!(plane.intersect(&Ray::new(Point::new(0.0, 0.0, 0.0), Vector::new(1.0, 0.0, 0.0))).is_none());
        assert!(plane.intersect(&Ray::new(Point::new(0.0, 1.0, 0.0), Vector::new(1.0, 0.0, 0.0))).is_none());
    }
}
