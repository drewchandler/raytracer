use super::color::Color;
use super::object::Object;
use super::ray::Ray;
use std::cmp::Ordering;

pub struct Scene {
    pub background: Color,
    pub objects: Vec<Object>
}

pub struct Intersection<'a> {
    pub object: &'a Object,
    pub distance: f64
}

impl<'a> Intersection<'a> {
    pub fn new(object: &'a Object, distance: f64) -> Intersection<'a> {
        Intersection {
            object: object,
            distance: distance
        }
    }
}

#[derive(PartialEq,PartialOrd)]
struct NonNan(f64);

impl NonNan {
    fn new(val: f64) -> Option<NonNan> {
        if val.is_nan() {
            None
        } else {
            Some(NonNan(val))
        }
    }
}

impl Eq for NonNan {}

impl Ord for NonNan {
    fn cmp(&self, other: &NonNan) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl Scene {
    pub fn nearest_interection(&self, r: &Ray) -> Option<Intersection> {
        self.objects.iter()
            .filter_map(|o| {
                match o.intersect(&r) {
                    Some(t) => Some(Intersection::new(o, t)),
                    _ => None
                }
            })
            .min_by_key(|o| NonNan::new(o.distance))
    }
}

#[cfg(test)]
mod tests {
    use super::Scene;
    use super::super::color::Color;
    use super::super::point::Point;
    use super::super::ray::Ray;
    use super::super::vector::Vector;
    use super::super::object::Object;
    use super::super::shapes::sphere::Sphere;

    #[test]
    fn test_nearest_intersection_no_intersection() {
        let s = Scene {
            background: Color::new(0, 0, 0),
            objects: vec![]
        };

        let result = s.nearest_interection(&Ray::new(Point::new(0.0, 0.0, 0.0), Vector::new(0.0, 0.0, 1.0)));

        assert!(result.is_none());
    }

    #[test]
    fn test_nearest_intersection_with_intersection() {
        let nearest_object = Object {
            shape: Box::new(Sphere::new(
                Point::new(0.0, 0.0, -50.0),
                10.0
            )),
            color: Color::new(0, 255, 0)
        };
        let scene = Scene {
            background: Color::new(255, 0, 0),
            objects: vec![
                nearest_object,
                Object {
                    shape: Box::new(Sphere::new(
                        Point::new(0.0, 0.0, -100.0),
                        10.0
                    )),
                    color: Color::new(0, 255, 0)
                }
            ]
        };

        let result = scene.nearest_interection(&Ray::new(Point::new(0.0, 0.0, 0.0), Vector::new(0.0, 0.0, -1.0)));

        assert_eq!(result.unwrap().distance, 40.0);
    }
}
