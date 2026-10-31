use super::super::shapes::Shape;
use super::super::point::Point;
use super::super::ray::Ray;
use super::super::vector::Vector;

pub struct Sphere {
    center: Point,
    radius: f64
}

impl Sphere {
    pub fn new(center: Point, radius: f64) -> Sphere {
        assert!(radius > 0.0 && radius.is_finite(), "Sphere radius must be positive and finite");
        Sphere { center: center, radius: radius }
    }
}

impl Shape for Sphere {
    fn intersections(&self, r: &Ray) -> Vec<f64> {
        let offset = r.origin - self.center;
        let a = r.direction.dot(&r.direction);
        let half_b = r.direction.dot(&offset);
        let c = offset.dot(&offset) - self.radius * self.radius;
        let discriminant = half_b * half_b - a * c;
        if a == 0.0 || discriminant < 0.0 { return vec![]; }
        vec![(-half_b - discriminant.sqrt()) / a, (-half_b + discriminant.sqrt()) / a]
    }

    fn normal_at(&self, point: &Point) -> Vector {
        (*point - self.center).normalize()
    }

    fn local_point(&self,point: &Point) -> Point {
        let v=*point-self.center;Point::new(v.dx,v.dy,v.dz)
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
    fn test_inside_sphere_uses_exit_root() {
        let s = Sphere::new(Point::new(0.0, 0.0, 0.0), 2.0);
        let r = Ray::new(Point::new(0.0, 0.0, 0.0), Vector::new(0.0, 0.0, 2.0));
        assert_eq!(s.intersections(&r), vec![-1.0, 1.0]);
        assert_eq!(s.intersect(&r), Some(1.0));
        assert_eq!(s.normal_at(&Point::new(0.0, 2.0, 0.0)), Vector::new(0.0, 1.0, 0.0));
    }

    #[test]
    fn test_surface_ray_can_hit_opposite_side() {
        let s = Sphere::new(Point::new(0.0, 0.0, 0.0), 1.0);
        let r = Ray::new(Point::new(0.0, 0.0, 1.0), Vector::new(0.0, 0.0, -1.0));
        assert_eq!(s.intersect(&r), Some(2.0));
        let tangent = Ray::new(Point::new(0.0, 1.0, -5.0), Vector::new(0.0, 0.0, 1.0));
        assert_eq!(s.intersect(&tangent), Some(5.0));
    }

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
