use super::super::shapes::Shape;
use super::super::point::Point;
use super::super::ray::Ray;

pub struct Sphere {
    center: Point,
    radius: f64
}

impl Sphere {
    pub fn new(center: Point, radius: f64) -> Sphere {
        Sphere { center: center, radius: radius }
    }
}

impl Shape for Sphere {
    fn intersect(&self, r: &Ray) -> Option<f64> {
        let direction = &r.direction;
        let origin = &r.origin;
        let center = &self.center;

        let a = direction.dx.powi(2) + direction.dy.powi(2) + direction.dz.powi(2);
        let b = 2.0 * direction.dx * (origin.x - center.x) +
                2.0 * direction.dy * (origin.y - center.y) +
                2.0 * direction.dz * (origin.z - center.z);
        let c = center.x.powi(2) + center.y.powi(2) + center.z.powi(2) +
                origin.x.powi(2) + origin.y.powi(2) + origin.z.powi(2) +
                -2.0 * (center.x * origin.x + center.y * origin.y + center.z * origin.z) -
                self.radius.powi(2);

        let discriminant = b.powi(2) - 4.0 * a * c;

        if discriminant < 0.0 {
            None
        } else {
            let d = (-b - discriminant.sqrt()) / (2.0 * a);

            if d >= 0.0 {
                Some(d)
            } else {
                None
            }
        }

    }
}

#[cfg(test)]
mod tests {
    use super::Sphere;
    use super::super::super::shapes::Shape;
    use super::super::super::point::Point;
    use super::super::super::ray::Ray;
    use super::super::super::vector::Vector;

    #[test]
    fn test_intersect() {
        let s = Sphere::new(Point::new(0.0, 0.0, -100.0), 50.0);
        let r = Ray::new(Point::new(0.0, 0.0, 0.0), Vector::new(0.0, 0.0, -1.0));
        let result = s.intersect(&r);

        assert_eq!(result, Some(50.0));
    }

    #[test]
    fn test_intersect_behind_ray() {
        let s = Sphere::new(Point::new(0.0, 0.0, -100.0), 50.0);
        let r = Ray::new(Point::new(0.0, 0.0, 0.0), Vector::new(0.0, 0.0, 1.0));
        let result = s.intersect(&r);

        assert_eq!(result, None);
    }

    #[test]
    fn test_intersect_no_intersection() {
        let s = Sphere::new(Point::new(0.0, 0.0, -100.0), 50.0);
        let r = Ray::new(Point::new(0.0, 0.0, 0.0), Vector::new(0.0, 1.0, 0.0));
        let result = s.intersect(&r);

        assert_eq!(result, None);
    }
}
