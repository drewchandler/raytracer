use super::color::{Color, LinearColor};
use super::light::Light;
use super::object::Object;
use super::ray::{Ray, EPSILON};
use super::texture::Environment;
use super::vector::Vector;

pub struct Scene {
    pub background: Color,
    pub environment: Option<Environment>,
    pub haze: Option<Haze>,
    pub ambient: Color,
    pub objects: Vec<Object>,
    pub lights: Vec<Light>
}

/// An artistic distance fade for the horizon; foreground geometry stays crisp.
pub struct Haze {
    start: f64,
    end: f64
}

impl Haze {
    pub fn new(start: f64, end: f64) -> Haze {
        assert!(start.is_finite() && end.is_finite() && start >= 0.0 && end > start);
        Haze { start: start, end: end }
    }
}

pub struct Intersection<'a> {
    pub object: &'a Object,
    pub distance: f64,
    pub object_index: usize
}

impl Scene {
    pub fn new(background: Color) -> Scene {
        Scene { background: background, environment: None, haze: None, ambient: Color::new(255, 255, 255), objects: vec![], lights: vec![] }
    }

    pub fn background_at(&self, direction: &Vector) -> LinearColor {
        self.background_ray(&Ray::new(::point::Point::new(0.0,0.0,0.0),*direction))
    }

    pub fn background_ray(&self,ray: &Ray) -> LinearColor {
        match self.environment {
            Some(ref environment) => environment.sample_ray(ray),
            None => self.background.linear()
        }
    }

    pub fn apply_haze(&self, color: LinearColor, distance: f64, direction: &Vector) -> LinearColor {
        match self.haze {
            Some(ref haze) => {
                let t = ((distance - haze.start) / (haze.end - haze.start)).max(0.0).min(1.0);
                let amount = t * t * (3.0 - 2.0 * t);
                if amount == 0.0 { return color; }
                color * (1.0 - amount) + self.background_at(direction) * amount
            },
            None => color
        }
    }

    pub fn surface_color(&self,color: LinearColor,distance: f64,ray: &Ray) -> LinearColor {
        let color=self.apply_haze(color,distance,&ray.direction);
        match self.environment {
            Some(ref environment)=>environment.transmit_surface(ray,distance,color),
            None=>color
        }
    }

    /// Keep negative hits as well: they tell us which volumes contain the ray origin.
    pub fn intersections(&self, ray: &Ray) -> Vec<Intersection> {
        let mut intersections = Vec::new();
        for (index, object) in self.objects.iter().enumerate() {
            for t in object.shape.intersections(ray) {
                if t.is_finite() {
                    intersections.push(Intersection { object: object, distance: t, object_index: index });
                }
            }
        }
        intersections.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap());
        intersections
    }

    pub fn nearest_intersection(&self, ray: &Ray) -> Option<Intersection> {
        self.intersections(ray).into_iter().find(|i| i.distance > EPSILON)
    }

    /// Compatibility with the original method's spelling.
    pub fn nearest_interection(&self, ray: &Ray) -> Option<Intersection> {
        self.nearest_intersection(ray)
    }

    /// Refractive indices on either side of one hit, including nested volumes.
    pub fn refractive_indices(&self, hit: usize, intersections: &[Intersection]) -> (f64, f64) {
        let mut containers: Vec<usize> = Vec::new();
        let mut n1 = 1.0;
        for (index, intersection) in intersections.iter().enumerate() {
            if index == hit {
                n1 = containers.last().map(|i| self.objects[*i].material.refractive_index).unwrap_or(1.0);
            }
            // Opaque planes/triangles are surfaces, not infinite optical media.
            if intersection.object.material.transparency > 0.0 {
                if let Some(position) = containers.iter().position(|i| *i == intersection.object_index) {
                    containers.remove(position);
                } else {
                    containers.push(intersection.object_index);
                }
            }
            if index == hit {
                let n2 = containers.last().map(|i| self.objects[*i].material.refractive_index).unwrap_or(1.0);
                return (n1, n2);
            }
        }
        (n1, 1.0)
    }

    /// Straight shadow rays approximate colored transmission through glass.
    /// Count each object once, rather than darkening at both entry and exit.
    pub fn transmittance(&self, ray: &Ray, max_distance: f64) -> LinearColor {
        let mut visibility = LinearColor::new(1.0, 1.0, 1.0);
        let mut seen = Vec::new();
        for hit in self.intersections(ray) {
            if hit.distance <= EPSILON || hit.distance >= max_distance { continue; }
            if seen.contains(&hit.object_index) { continue; }
            seen.push(hit.object_index);
            let transmission = hit.object.material.transparency;
            if transmission == 0.0 { return LinearColor::black(); }
            visibility = visibility * hit.object.albedo_at(&ray.at(hit.distance)) * transmission;
        }
        visibility
    }
}

#[cfg(test)]
mod tests {
    use super::{Scene, Haze};
    use super::super::color::{Color, LinearColor};
    use super::super::material::Material;
    use super::super::point::Point;
    use super::super::ray::Ray;
    use super::super::vector::Vector;
    use super::super::object::Object;
    use super::super::shapes::Shape;
    use super::super::shapes::sphere::Sphere;

    fn sphere(z: f64, radius: f64, material: Material) -> Object {
        Object::with_material(Box::new(Sphere::new(Point::new(0.0, 0.0, z), radius)), Color::new(255, 255, 255), material)
    }

    #[test]
    fn test_nearest_intersection_no_intersection() {
        let s = Scene::new(Color::new(0, 0, 0));
        assert!(s.nearest_interection(&Ray::new(Point::new(0.0, 0.0, 0.0), Vector::new(0.0, 0.0, 1.0))).is_none());
    }

    #[test]
    fn horizon_haze_preserves_foreground_and_fades_in_linear_light() {
        let mut scene = Scene::new(Color::new(0, 0, 255));
        scene.haze = Some(Haze::new(20.0, 100.0));
        let red = Color::new(255, 0, 0).linear();
        let direction = Vector::new(0.0, 0.0, -1.0);
        assert_eq!(scene.apply_haze(red, 10.0, &direction), red);
        assert_eq!(scene.apply_haze(red, 60.0, &direction), LinearColor::new(0.5, 0.0, 0.5));
        assert_eq!(scene.apply_haze(red, 1000.0, &direction), scene.background.linear());
    }

    #[test]
    #[should_panic]
    fn haze_rejects_an_invalid_range() { Haze::new(20.0, 10.0); }

    #[test]
    fn test_nearest_intersection_with_intersection() {
        let mut s = Scene::new(Color::new(255, 0, 0));
        s.objects = vec![sphere(-100.0, 10.0, Material::default()), sphere(-50.0, 10.0, Material::default())];
        let ray = Ray::new(Point::new(0.0, 0.0, 0.0), Vector::new(0.0, 0.0, -1.0));
        assert_eq!(s.nearest_interection(&ray).unwrap().distance, 40.0);
    }

    struct InvalidHits;
    impl Shape for InvalidHits {
        fn intersections(&self, _: &Ray) -> Vec<f64> { vec![::std::f64::NAN, ::std::f64::INFINITY, -1.0, 0.0, 8.0] }
        fn normal_at(&self, _: &Point) -> Vector { Vector::new(0.0, 1.0, 0.0) }
    }

    #[test]
    fn ignores_nonfinite_and_nonpositive_hits() {
        let mut scene = Scene::new(Color::new(0, 0, 0));
        scene.objects.push(Object::new(Box::new(InvalidHits), Color::new(255, 255, 255)));
        assert_eq!(scene.nearest_intersection(&Ray::new(Point::new(0.0, 0.0, 0.0), Vector::new(0.0, 0.0, 1.0))).unwrap().distance, 8.0);
    }

    #[test]
    fn nested_glass_tracks_each_medium_on_entry_and_exit() {
        let mut scene = Scene::new(Color::new(0, 0, 0));
        scene.objects = vec![sphere(0.0, 2.0, Material::glass(1.5)), sphere(-0.25, 1.0, Material::glass(2.0)), sphere(0.25, 1.0, Material::glass(2.5))];
        let ray = Ray::new(Point::new(0.0, 0.0, -4.0), Vector::new(0.0, 0.0, 1.0));
        let xs = scene.intersections(&ray);
        let expected = [(1.0, 1.5), (1.5, 2.0), (2.0, 2.5), (2.5, 2.5), (2.5, 1.5), (1.5, 1.0)];
        assert_eq!(xs.len(), expected.len());
        for i in 0..xs.len() { assert_eq!(scene.refractive_indices(i, &xs), expected[i]); }
    }

    #[test]
    fn camera_inside_glass_starts_in_the_correct_medium() {
        let mut scene = Scene::new(Color::new(0, 0, 0));
        scene.objects.push(sphere(0.0, 1.0, Material::glass(1.5)));
        let ray = Ray::new(Point::new(0.0, 0.0, 0.0), Vector::new(0.0, 0.0, 1.0));
        let xs = scene.intersections(&ray);
        assert_eq!(scene.refractive_indices(1, &xs), (1.5, 1.0));
    }

    #[test]
    fn opaque_surfaces_do_not_replace_a_glass_medium() {
        let mut scene = Scene::new(Color::new(0, 0, 0));
        scene.objects.push(sphere(0.0, 2.0, Material::glass(1.5)));
        scene.objects.push(Object::new(Box::new(::shapes::plane::Plane::new(Point::new(0.0, 0.0, -1.0),
            Vector::new(0.0, 0.0, 1.0))), Color::new(255, 255, 255)));
        let ray = Ray::new(Point::new(0.0, 0.0, 0.0), Vector::new(0.0, 0.0, 1.0));
        let xs = scene.intersections(&ray);
        assert_eq!(scene.refractive_indices(2, &xs), (1.5, 1.0));
    }

    #[test]
    fn shadows_only_count_occluders_between_point_and_light() {
        let mut scene = Scene::new(Color::new(0, 0, 0));
        scene.objects.push(sphere(5.0, 1.0, Material::default()));
        let ray = Ray::new(Point::new(0.0, 0.0, 0.0), Vector::new(0.0, 0.0, 1.0));
        assert_eq!(scene.transmittance(&ray, 3.0), LinearColor::new(1.0, 1.0, 1.0));
        assert_eq!(scene.transmittance(&ray, 10.0), LinearColor::black());
        let away = Ray::new(ray.origin, -ray.direction);
        assert_eq!(scene.transmittance(&away, 10.0), LinearColor::new(1.0, 1.0, 1.0));
    }

    #[test]
    fn glass_shadows_are_filtered_once_per_object() {
        let mut scene = Scene::new(Color::new(0, 0, 0));
        let mut glass = Material::glass(1.5);
        glass.transparency = 0.5;
        scene.objects.push(sphere(5.0, 1.0, glass));
        let ray = Ray::new(Point::new(0.0, 0.0, 0.0), Vector::new(0.0, 0.0, 1.0));
        assert_eq!(scene.transmittance(&ray, 10.0), LinearColor::new(0.5, 0.5, 0.5));
    }
}
