use std::f64;
use super::Shape;
use super::super::point::Point;
use super::super::ray::Ray;
use super::super::vector::Vector;

/// An axis-aligned box. Wrap it in Transformed for an oriented box.
pub struct Cube {
    min: Point,
    max: Point
}

impl Cube {
    pub fn new(min: Point, max: Point) -> Cube {
        assert!(min.x < max.x && min.y < max.y && min.z < max.z);
        Cube { min: min, max: max }
    }

    fn slab(origin: f64, direction: f64, min: f64, max: f64) -> Option<(f64, f64)> {
        if direction == 0.0 {
            if origin < min || origin > max { None } else { Some((f64::NEG_INFINITY, f64::INFINITY)) }
        } else {
            let a = (min - origin) / direction;
            let b = (max - origin) / direction;
            Some((a.min(b), a.max(b)))
        }
    }
}

impl Shape for Cube {
    fn intersections(&self, ray: &Ray) -> Vec<f64> {
        let slabs = [
            Cube::slab(ray.origin.x, ray.direction.dx, self.min.x, self.max.x),
            Cube::slab(ray.origin.y, ray.direction.dy, self.min.y, self.max.y),
            Cube::slab(ray.origin.z, ray.direction.dz, self.min.z, self.max.z)
        ];
        let mut enter = f64::NEG_INFINITY;
        let mut exit = f64::INFINITY;
        for slab in &slabs {
            match *slab {
                Some((a, b)) => { enter = enter.max(a); exit = exit.min(b); },
                None => return vec![]
            }
        }
        if enter > exit { vec![] } else { vec![enter, exit] }
    }

    fn normal_at(&self, point: &Point) -> Vector {
        let faces = [
            ((point.x - self.min.x).abs(), Vector::new(-1.0, 0.0, 0.0)),
            ((point.x - self.max.x).abs(), Vector::new(1.0, 0.0, 0.0)),
            ((point.y - self.min.y).abs(), Vector::new(0.0, -1.0, 0.0)),
            ((point.y - self.max.y).abs(), Vector::new(0.0, 1.0, 0.0)),
            ((point.z - self.min.z).abs(), Vector::new(0.0, 0.0, -1.0)),
            ((point.z - self.max.z).abs(), Vector::new(0.0, 0.0, 1.0))
        ];
        let mut closest = faces[0];
        for face in &faces[1..] {
            if face.0 < closest.0 { closest = *face; }
        }
        closest.1
    }
}

#[cfg(test)]
mod tests {
    use super::Cube;
    use super::super::Shape;
    use super::super::super::point::Point;
    use super::super::super::ray::Ray;
    use super::super::super::vector::Vector;

    #[test]
    fn box_hits_from_outside_and_inside() {
        let cube = Cube::new(Point::new(-1.0, -1.0, -1.0), Point::new(1.0, 1.0, 1.0));
        let ray = Ray::new(Point::new(0.0, 0.0, 5.0), Vector::new(0.0, 0.0, -1.0));
        assert_eq!(cube.intersections(&ray), vec![4.0, 6.0]);
        let inside = Ray::new(Point::new(0.0, 0.0, 0.0), Vector::new(1.0, 0.0, 0.0));
        assert_eq!(cube.intersect(&inside), Some(1.0));
        assert_eq!(cube.normal_at(&Point::new(1.0, 0.3, -0.2)), Vector::new(1.0, 0.0, 0.0));
    }

    #[test]
    fn parallel_ray_on_a_face_is_finite_but_outside_misses() {
        let cube = Cube::new(Point::new(-1.0, -1.0, -1.0), Point::new(1.0, 1.0, 1.0));
        let ray = Ray::new(Point::new(1.0, 0.0, -5.0), Vector::new(0.0, 0.0, 1.0));
        assert_eq!(cube.intersections(&ray), vec![4.0, 6.0]);
        let miss = Ray::new(Point::new(1.1, 0.0, -5.0), Vector::new(0.0, 0.0, 1.0));
        assert!(cube.intersect(&miss).is_none());
    }
}
